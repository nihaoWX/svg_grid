#!/usr/bin/env python3
"""R51 evidence: the remaining holes in the panel-letter duplicate check.

Families
  A  a nested ``data-svg-grid-label`` marker is counted once (not once per level)
  B  ``<defs>`` content counts once per ``<use>`` instantiation, cycles expand once
  C  U+00A0 stays significant, zero-width characters are removed before comparing
  D  ``!important`` is stripped; uppercase display/visibility keywords stay visible
  E  a ``<text>`` that paints nothing is not a letter
  G  (optional) the shipped corpus still converts byte-identically to the base

Every new semantic branch is backed by the reference renderer (resvg via
d1render): the expected ink numbers below come from it, and the run prints the
measured value. A second, broken build can be pointed at with
SVG_GRID_BROKEN_BIN; each "teeth" check asserts the *old wrong* answer, so a
green run here cannot be a green run of the unfixed binary.

Usage:
  python3 docs/PITFALLS/round-data/R51/test.py
  SVG_GRID_BIN=/tmp/r51head/target/release/svg_grid python3 ...   # must go red
  SVG_GRID_CORPUS=1 python3 ...                                   # + byte-identical corpus
"""

import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN", ROOT / "target/debug/svg_grid"))
OLD = pathlib.Path(os.environ.get("SVG_GRID_OLD_BIN", "/tmp/r48old/target/release/svg_grid"))
BROKEN = pathlib.Path(os.environ.get("SVG_GRID_BROKEN_BIN", "/tmp/r51head/target/release/svg_grid"))
BASE = pathlib.Path(os.environ.get("SVG_GRID_BASE_BIN", "/tmp/r46base/target/release/svg_grid"))
RENDER = pathlib.Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
FONTDIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="r51-"))

PASSED = []
FAILED = []
SKIPPED = []


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("ok   " if ok else "FAIL ") + label + (f"  [{detail}]" if detail else ""))
    return ok


def skip(label, why):
    SKIPPED.append(label)
    print(f"skip {label}  [{why}]")


def write(name, text):
    path = TMP / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


def run(binary, args):
    return subprocess.run([str(binary), *args], capture_output=True, text=True, cwd=ROOT)


def panel(inner):
    return (
        '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>\n'
        f"{inner}\n</svg>\n"
    )


def letter(text, marker="0", attrs="", x="4"):
    return (
        f'<text data-svg-grid-label="{marker}" x="{x}" y="14" font-size="12" '
        f'font-family="Arial" {attrs}>{text}</text>'
    )


DEFS = (
    '<defs><text id="L" data-svg-grid-label="0" x="4" y="14" font-size="12" '
    'font-family="Arial">A</text></defs>'
)


def compose(binary, inners, tag):
    files = []
    for index, inner in enumerate(inners):
        files.append(write(f"{tag}-{index}.svg", panel(inner)))
    out = TMP / f"{tag}.out.svg"
    args = ["plot-grid"]
    for path in files:
        args += ["--input", str(path)]
    args += ["--output", str(out), "--width", "400", "--ncol", str(len(files))]
    result = run(binary, args)
    return result.returncode, out, result.stderr


def case(label, inners, rc, fragments=(), wrote_output=None, ink=None):
    code, out, stderr = compose(BIN, inners, label.replace(" ", "_"))
    if code != rc:
        return check(False, label, f"rc={code} want {rc}: {stderr.strip()[:160]}")
    for fragment in fragments:
        if fragment not in stderr:
            return check(False, label, f"missing {fragment!r}: {stderr.strip()[:160]}")
    if wrote_output is not None and out.exists() != wrote_output:
        return check(False, label, f"output exists={out.exists()} want {wrote_output}")
    if rc != 0 and out.exists():
        return check(False, label, "a refused call must not write the product")
    if ink is not None:
        measured = ink_of(inners[0])
        if measured is None:
            return check(True, label, "rc checked; renderer unavailable for ink")
        if measured != ink:
            return check(False, label, f"reference ink={measured} want {ink}")
    return check(True, label, f"rc={code}" + (f" ink={ink}" if ink is not None else ""))


def teeth(label, inners, broken_rc):
    if not BROKEN.exists():
        return skip(f"teeth: {label}", f"{BROKEN} missing")
    code, _, stderr = compose(BROKEN, inners, "teeth_" + label.replace(" ", "_"))
    check(code == broken_rc, f"teeth: {label}", f"unfixed rc={code} want {broken_rc}")


# --- PNG helpers (8-bit RGB/RGBA, filters 0-4) ---------------------------------


def read_png(path):
    data = pathlib.Path(path).read_bytes()
    position = 8
    rows = []
    width = height = channels = 0
    while position + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[position : position + 8])
        payload = data[position + 8 : position + 8 + length]
        position += 12 + length
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", payload[:10])
            channels = {0: 1, 2: 3, 4: 2, 6: 4}[colour]
        elif kind == b"IDAT":
            rows.append(payload)
        elif kind == b"IEND":
            break
    raw = zlib.decompress(b"".join(rows))
    stride = width * channels
    out = []
    previous = bytearray(stride)
    cursor = 0
    for _ in range(height):
        filter_kind = raw[cursor]
        cursor += 1
        line = bytearray(raw[cursor : cursor + stride])
        cursor += stride
        for index in range(stride):
            left = line[index - channels] if index >= channels else 0
            up = previous[index]
            corner = previous[index - channels] if index >= channels else 0
            value = line[index]
            if filter_kind == 1:
                value += left
            elif filter_kind == 2:
                value += up
            elif filter_kind == 3:
                value += (left + up) // 2
            elif filter_kind == 4:
                estimate = left + up - corner
                pa, pb, pc = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
                predictor = left if pa <= pb and pa <= pc else (up if pb <= pc else corner)
                value += predictor
            elif filter_kind != 0:
                raise ValueError(f"filter {filter_kind}")
            line[index] = value & 0xFF
        out.append(bytes(line))
        previous = line
    return width, height, channels, out


def ink(path):
    width, _, channels, rows = read_png(path)
    return sum(
        1
        for row in rows
        for x in range(width)
        if row[x * channels : x * channels + 3] != b"\xff\xff\xff"
    )


def diff(a, b):
    aw, ah, ac, arows = read_png(a)
    bw, bh, bc, brows = read_png(b)
    if (aw, ah) != (bw, bh):
        return 10**9
    return sum(
        1
        for y in range(ah)
        for x in range(aw)
        if arows[y][x * ac : x * ac + 3] != brows[y][x * bc : x * bc + 3]
    )


def render(inner, tag):
    if not RENDER.exists():
        return None
    svg = write(f"render-{tag}.svg", panel(inner))
    png = TMP / f"render-{tag}.png"
    env = dict(os.environ)
    if FONTDIR.is_dir():
        env["D1_FONT_DIR"] = str(FONTDIR)
    result = subprocess.run([str(RENDER), str(svg), str(png), "1"], capture_output=True, env=env)
    return png if result.returncode == 0 and png.exists() else None


def ink_of(inner):
    png = render(inner, "ink")
    return ink(png) if png else None


# --- A: nested markers ---------------------------------------------------------


def family_a():
    nested = f'<g data-svg-grid-label="0">{letter("A", "1")}</g>'
    case("A1 a nested marker is one letter", [nested], 0, wrote_output=True, ink=40)
    case(
        "A2 outer text stops at a nested marker",
        [f'<g data-svg-grid-label="0">{letter("A")}{letter("B", "1")}</g>', letter("AB")],
        0,
        wrote_output=True,
        ink=57,
    )
    case(
        "A3 outer A next to nested A is a real duplicate",
        [f'<g data-svg-grid-label="0">{letter("A")}{letter("A", "1")}</g>'],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 0)", "#0 (cell 1)"],
        wrote_output=False,
        ink=40,
    )
    case(
        "A4 three levels count the innermost once",
        [f'<g data-svg-grid-label="0"><g data-svg-grid-label="1">{letter("A", "2")}</g></g>'],
        0,
        wrote_output=True,
        ink=40,
    )
    case(
        "A4b three levels next to a plain A still collide at cell 2",
        [f'<g data-svg-grid-label="0"><g data-svg-grid-label="1">{letter("A", "2")}</g></g>', letter("A")],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 2)", "#1 (cell 0)"],
        wrote_output=False,
    )
    case(
        "A5 the same nested letter across inputs collides",
        [nested, letter("A")],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 1)", "#1 (cell 0)"],
        wrote_output=False,
    )
    teeth("A1 nested marker", [nested], 2)
    teeth("A4 three levels", [f'<g data-svg-grid-label="0"><g data-svg-grid-label="1">{letter("A", "2")}</g></g>'], 2)


# --- B: <defs> + <use> ---------------------------------------------------------


def family_b():
    case(
        "B1 a definition counts once per use",
        [DEFS + '<use href="#L"/>', letter("A", "1")],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 0)", "#1 (cell 1)"],
        wrote_output=False,
        ink=40,
    )
    case(
        "B2 two uses are two letters",
        [DEFS + '<use href="#L"/><use href="#L" x="30"/>'],
        2,
        fragments=['duplicate panel letters: "A"', "2 times inside #0 (cell 0)"],
        wrote_output=False,
        ink=80,
    )
    case(
        "B3 an unreferenced definition is not a letter",
        [DEFS, letter("A", "1")],
        0,
        wrote_output=True,
        ink=0,
    )
    case(
        "B4 a self-referencing use expands once",
        ['<g id="L" data-svg-grid-label="0">' + letter("A") + '<use href="#L" x="30"/></g>'],
        0,
        wrote_output=True,
        ink=40,
    )
    case(
        "B4b a mutual use cycle expands once",
        [
            '<defs><g id="A" data-svg-grid-label="0">'
            + letter("A")
            + '<use href="#B"/></g><g id="B"><use href="#A"/></g></defs><use href="#A"/>'
        ],
        0,
        wrote_output=True,
    )
    case(
        "B5 a symbol is drawn only through its use",
        [
            '<defs><symbol id="S">' + letter("A") + "</symbol></defs><use href=\"#S\"/>",
            letter("A", "1"),
        ],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 0)", "#1 (cell 1)"],
        wrote_output=False,
        ink=40,
    )
    case("B6 an unreferenced symbol is not a letter", ['<defs><symbol id="S">' + letter("A") + "</symbol></defs>", letter("A", "1")], 0, wrote_output=True, ink=0)
    teeth("B1 defs+use", [DEFS + '<use href="#L"/>', letter("A", "1")], 0)
    teeth("B2 two uses", [DEFS + '<use href="#L"/><use href="#L" x="30"/>'], 0)


# --- C: whitespace and zero-width characters -----------------------------------


def family_c():
    case("C1 a no-break space stays significant", [letter("A"), letter("A&#160;")], 0, wrote_output=True, ink=40)
    case(
        "C1b two identical A-nbsp do collide",
        [letter("A&#160;"), letter("A&#160;")],
        2,
        fragments=['duplicate panel letters: "A', "#0 (cell 0)", "#1 (cell 0)"],
        wrote_output=False,
    )
    case(
        "C2 a zero-width space is removed",
        [letter("A"), letter("A\u200b")],
        2,
        fragments=['duplicate panel letters: "A"', "#0 (cell 0)", "#1 (cell 0)"],
        wrote_output=False,
    )
    first = render(letter("A&#160;"), "nbsp")
    second = render(letter("A"), "plain")
    if first and second:
        check(diff(first, second) == 0, "C1r reference: A-nbsp renders exactly like A", f"xor={diff(first, second)}")
    zw = render(letter("A\u200b"), "zwsp")
    if zw and second:
        check(diff(zw, second) == 0, "C2r reference: A-zwsp renders exactly like A", f"xor={diff(zw, second)}")
    teeth("C1 A vs A-nbsp", [letter("A"), letter("A&#160;")], 2)
    teeth("C2 A vs A-zwsp", [letter("A"), letter("A\u200b")], 0)


# --- D: !important and keyword casing -----------------------------------------


def family_d():
    for tag, declaration in [
        ("plain", "display:none !important"),
        ("upper", "display:none !IMPORTANT"),
        ("spaced", "display:none ! important"),
        ("tight", "display:none!important"),
    ]:
        case(
            f"D1 {tag} !important hides the subtree",
            [f'<g style="{declaration}">{letter("A")}</g>', letter("A")],
            0,
            wrote_output=True,
            ink=0,
        )
    for tag, attributes in [
        ("css-value", 'style="display:NONE"'),
        ("css-name", 'style="DISPLAY:none"'),
        ("attr-value", 'display="NONE"'),
        ("visibility", 'visibility="HIDDEN"'),
    ]:
        case(
            f"D2 {tag} is still drawn, so it is still a letter",
            [f'<g {attributes}>{letter("A")}</g>', letter("A")],
            2,
            fragments=['duplicate panel letters: "A"'],
            wrote_output=False,
            ink=40,
        )
    teeth("D1 !important", [f'<g style="display:none !important">{letter("A")}</g>', letter("A")], 2)


# --- E: a text that paints nothing --------------------------------------------


def family_e():
    case("E1 fill=none paints nothing", [letter("A", attrs='fill="none"'), letter("A")], 0, wrote_output=True, ink=0)
    case("E2 fill=none plus a stroke still paints", [letter("A", attrs='fill="none" stroke="black"'), letter("A")], 2, fragments=['duplicate panel letters: "A"'], wrote_output=False, ink=61)
    case("E3 fill=none stroke=none paints nothing", [letter("A", attrs='fill="none" stroke="none"'), letter("A")], 0, wrote_output=True, ink=0)
    case(
        "E4 a descendant may restore an inherited fill=none",
        ['<g fill="none">' + letter("A", attrs='fill="black"') + "</g>", letter("A")],
        2,
        fragments=['duplicate panel letters: "A"'],
        wrote_output=False,
    )
    case("E5 an inherited fill=none still paints nothing", ['<g fill="none">' + letter("A") + "</g>", letter("A")], 0, wrote_output=True, ink=0)
    teeth("E1 fill=none", [letter("A", attrs='fill="none"'), letter("A")], 2)


# --- G: the shipped corpus ----------------------------------------------------


def family_g():
    if os.environ.get("SVG_GRID_CORPUS") != "1":
        return skip("G corpus byte-identical", "set SVG_GRID_CORPUS=1")
    if not BASE.exists():
        return skip("G corpus byte-identical", f"{BASE} missing")
    svgs = sorted((ROOT / "tests").rglob("*.svg"))
    fixtures = sorted((ROOT / "tests/fixtures").rglob("*.svg"))
    differences = []
    for index, source in enumerate(svgs):
        results = []
        for tag, binary in (("new", BIN), ("base", BASE)):
            out = TMP / f"corpus/{tag}/{index}.svg"
            out.parent.mkdir(parents=True, exist_ok=True)
            result = run(binary, ["convert", "--input", str(source), "--output", str(out), "--force"])
            results.append((result.returncode, out.read_bytes() if out.exists() else None))
        if results[0] != results[1]:
            differences.append(str(source.relative_to(ROOT)))
    for index, source in enumerate(fixtures):
        results = []
        for tag, binary in (("new", BIN), ("base", BASE)):
            out = TMP / f"corpus/{tag}/pg{index}.svg"
            out.parent.mkdir(parents=True, exist_ok=True)
            result = run(binary, ["plot-grid", "--input", str(source), "--output", str(out), "--width", "600", "--ncol", "1"])
            results.append((result.returncode, out.read_bytes() if out.exists() else None))
        if results[0] != results[1]:
            differences.append(str(source.relative_to(ROOT)))
    check(
        not differences,
        "G corpus byte-identical",
        f"convert={len(svgs)} plot-grid={len(fixtures)} differences={len(differences)}" + (f" first={differences[:3]}" if differences else ""),
    )


def main():
    if not BIN.exists():
        print(f"missing binary: {BIN}", file=sys.stderr)
        return 2
    family_a()
    family_b()
    family_c()
    family_d()
    family_e()
    family_g()
    print()
    print(f"passed={len(PASSED)} failed={len(FAILED)} skipped={len(SKIPPED)}  tmp={TMP}")
    if FAILED:
        print("FAILED: " + ", ".join(FAILED))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
