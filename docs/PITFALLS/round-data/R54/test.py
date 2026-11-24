#!/usr/bin/env python3
"""R54 evidence: the remaining false positives / false negatives in the
composite-figure panel-letter duplicate check.

Families
  A  a glyph painted by an unresolved ``url(#...)`` reference, or under an
     inherited zero ``fill-opacity`` / ``stroke-opacity`` / ``stroke-width``,
     leaves ink 0 in both reference renderers -> it is not a letter.
  B  the inline ``style`` cascade: ``/* ... */`` comments are stripped, the
     later declaration wins, and ``!important`` outranks a plain declaration.
  C  a marker on a ``<symbol>`` is drawn through ``<use>`` -> it is a letter.
  D  a marker's letter is the whole text its subtree draws, nested markers
     included; the outer marker is dropped only when it adds no character of
     its own. Bare character data inside a ``<g>`` is not painted at all.
  E  the fixes must not turn any painted glyph into a non-letter.

Ground truth: every expected rc/ink below was measured with resvg 0.47
(``d1render``) and cross-checked with cairosvg; the two renderers only disagree
on ``display:none /* c */`` (cairo keeps the comment, resvg strips it) and on
uppercase ``DISPLAY:none`` (cairo hides it, resvg keeps it) -- both follow
resvg, as R51 ruled. ``SVG_GRID_OLD_BIN`` must point at a pre-R54 build: the
run asserts the *old wrong* rc for every case, so a green run here cannot be a
green run of the unfixed binary. A missing renderer is a hard failure unless
``--allow-skip`` is passed.

Usage
  python3 docs/PITFALLS/round-data/R54/test.py
  SVG_GRID_BIN=/tmp/r54bin/svg_grid_before python3 ...   # must go red
  python3 ... --allow-skip                               # no renderer here
"""

import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "release" / "svg_grid")
OLD = pathlib.Path(os.environ.get("SVG_GRID_OLD_BIN") or "/tmp/r54bin/svg_grid_before")
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
CAIRO = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON") or "/home/nihao/.venv/bin/python")
FONTDIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="r54-"))
ALLOW_SKIP = "--allow-skip" in sys.argv

PASSED, FAILED, SKIPPED = [], [], []

# Cases where cairosvg disagrees with resvg; R51's ruling follows resvg.
DIVERGENT = {"B1", "E9"}


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("ok   " if ok else "FAIL ") + label + (f"  [{detail}]" if detail else ""))
    return ok


def skip(label, why):
    if ALLOW_SKIP:
        SKIPPED.append(label)
        print(f"skip {label}  [{why}]")
        return True
    return check(False, label, f"refusing to skip: {why} (pass --allow-skip)")


def write(name, text):
    path = TMP / name
    path.write_text(text, encoding="utf-8")
    return path


# The fixture shape the dispatch used: a 100x40 white panel, so "ink" is the
# dark pixel count against white.
PANEL = (
    '<svg width="100" height="40" xmlns="http://www.w3.org/2000/svg">'
    '<rect data-panel-box="main" x="0" y="0" width="100" height="40" fill="white"/>'
    "{}</svg>"
)


def letter(text, marker="0", attrs="", x="4"):
    return (
        f'<text data-svg-grid-label="{marker}" x="{x}" y="20" font-size="12" '
        f'font-family="Arial" {attrs}>{text}</text>'
    )


def panel(inner):
    return PANEL.format(inner)


def compose(binary, inners, tag):
    files = [write(f"{tag}-{i}.svg", panel(inner)) for i, inner in enumerate(inners)]
    out = TMP / f"{tag}.out.svg"
    args = ["plot-grid"]
    for path in files:
        args += ["--input", str(path)]
    args += ["--output", str(out), "--width", "200", "--ncol", str(len(files))]
    result = subprocess.run([str(binary), *args], capture_output=True, text=True, cwd=ROOT)
    return result.returncode, out, result.stderr


# --- PNG ink (8-bit RGB/RGBA, filters 0-4) ------------------------------------


def read_png(path):
    data = pathlib.Path(path).read_bytes()
    position = 8
    rows, width, height, channels = [], 0, 0, 0
    while position + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[position : position + 8])
        payload = data[position + 8 : position + 8 + length]
        position += 12 + length
        if kind == b"IHDR":
            width, height, _, colour = struct.unpack(">IIBB", payload[:10])
            channels = {0: 1, 2: 3, 4: 2, 6: 4}[colour]
        elif kind == b"IDAT":
            rows.append(payload)
        elif kind == b"IEND":
            break
    raw = zlib.decompress(b"".join(rows))
    stride = width * channels
    out, previous, cursor = [], bytearray(stride), 0
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
            line[index] = value & 0xFF
        out.append(bytes(line))
        previous = line
    return width, height, channels, out


def ink(path):
    """(dark, non-white) pixel counts; dark = every channel < 128."""
    width, height, channels, rows = read_png(path)
    dark = nonwhite = 0
    for y in range(height):
        for x in range(width):
            pixel = rows[y][x * channels : x * channels + 3]
            if pixel != b"\xff\xff\xff":
                nonwhite += 1
                if max(pixel) < 128:
                    dark += 1
    return dark, nonwhite


def render_d1(inner, tag):
    svg = write(f"d1-{tag}.svg", panel(inner))
    png = TMP / f"d1-{tag}.png"
    env = dict(os.environ)
    if FONTDIR.is_dir():
        env["D1_FONT_DIR"] = str(FONTDIR)
    result = subprocess.run(
        [str(RENDER), str(svg), str(png), "1"], capture_output=True, env=env
    )
    if result.returncode != 0 or not png.exists():
        raise RuntimeError(f"d1render failed: {result.stderr[-300:]!r}")
    return ink(png)


def render_cairo(inner, tag):
    svg = write(f"cairo-{tag}.svg", panel(inner))
    png = TMP / f"cairo-{tag}.png"
    code = "import cairosvg, sys; cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2])"
    result = subprocess.run([str(CAIRO), "-c", code, str(svg), str(png)], capture_output=True)
    if result.returncode != 0 or not png.exists():
        raise RuntimeError(f"cairosvg failed: {result.stderr[-300:]!r}")
    return ink(png)


# --- cases -------------------------------------------------------------------
# name: (inputs, rc_after, rc_before, d1_dark)
# inputs are panel bodies; two bodies means a two-panel figure.
G = '<g data-svg-grid-label="0">{}</g>'
STROKED = letter("A", attrs="fill='none' stroke='black'")
THREE = (
    '<g data-svg-grid-label="0">'
    '<text x="4" y="20" font-size="12" font-family="Arial">A</text>'
    + letter("B", "1", x="20")
    + '<text x="36" y="20" font-size="12" font-family="Arial">C</text>'
    + "</g>"
)

CASES = {
    # A: nothing painted
    "A1 fill url(#missing)": ([letter("A", attrs='fill="url(#missing)"')] * 2, 0, 2, 0),
    "A2 style fill url(#missing)": (
        [letter("A", attrs='style="fill:url(#missing)"')] * 2, 0, 2, 0,
    ),
    "A3 group fill url(#missing)": (
        ['<g fill="url(#missing)">' + letter("A") + "</g>"] * 2, 0, 2, 0,
    ),
    "A4 stroke url(#missing)": (
        [letter("A", attrs='fill="none" stroke="url(#missing)"')] * 2, 0, 2, 0,
    ),
    "A5 fill-opacity 0": ([letter("A", attrs='fill-opacity="0"')] * 2, 0, 2, 0),
    "A6 group fill-opacity 0": (
        ['<g fill-opacity="0">' + letter("A") + "</g>"] * 2, 0, 2, 0,
    ),
    "A7 group stroke-opacity 0": (
        ['<g stroke-opacity="0">' + STROKED + "</g>"] * 2, 0, 2, 0,
    ),
    "A8 group stroke-width 0": (
        ['<g stroke-width="0">' + STROKED + "</g>"] * 2, 0, 2, 0,
    ),
    # B: inline style cascade
    "B1 comment after none": ([G.format(letter("A")).replace("<g ", '<g style="display:none /* c */" ')] * 2, 0, 2, 0),
    "B2 later block wins": (
        ['<g style="display:none;display:block">' + letter("A") + "</g>"] * 2, 2, 0, 23,
    ),
    "B3 later none wins": (
        ['<g style="display:block;display:none">' + letter("A") + "</g>"] * 2, 0, 2, 0,
    ),
    "B4 important none wins": (
        ['<g style="display:block;display:none !important">' + letter("A") + "</g>"] * 2,
        0, 2, 0,
    ),
    "B5 important none beats earlier none": (
        ['<g style="display:none !important;display:block">' + letter("A") + "</g>"] * 2,
        0, 0, 0,
    ),
    "B5b important block beats plain none": (
        ['<g style="display:none;display:block !important">' + letter("A") + "</g>"] * 2,
        2, 0, 23,
    ),
    "B6 later visibility wins": (
        ['<g style="visibility:hidden;visibility:visible">' + letter("A") + "</g>"] * 2,
        2, 0, 23,
    ),
    # C: a marker on a <symbol>
    "C1 symbol marker through use": (
        [
            '<defs><symbol id="s" data-svg-grid-label="0"><text x="4" y="20" '
            'font-size="12" font-family="Arial">A</text></symbol></defs><use href="#s"/>'
        ] * 2,
        2, 0, 23,
    ),
    "C2 group marker through use (control)": (
        [
            '<defs><g id="s" data-svg-grid-label="0"><text x="4" y="20" '
            'font-size="12" font-family="Arial">A</text></g></defs><use href="#s"/>'
        ] * 2,
        2, 2, 23,
    ),
    # D: nested markers
    "D1 outer reads ABC, collides with ABC": ([THREE, letter("ABC")], 2, 0, 71),
    "D1b plain ABC": ([letter("ABC")] * 2, 2, 2, 71),
    "D2 outer ABC does not collide with AC": ([THREE, letter("AC")], 0, 2, 71),
    "D3 bare text is not painted": (
        ['<g data-svg-grid-label="0">A' + letter("B", "1") + "C</g>", letter("AC")],
        0, 2, 26,
    ),
    "D3b the one drawn glyph is a letter": (
        ['<g data-svg-grid-label="0">A' + letter("B", "1") + "C</g>", letter("B")],
        2, 2, 26,
    ),
    # E: must not break
    "E1 stroke with missing fill reference": (
        [letter("A", attrs='fill="url(#missing)" stroke="black"')] * 2, 2, 2, 41,
    ),
    "E2 gradient reference": (
        [
            '<defs><linearGradient id="g"><stop offset="0" stop-color="black"/>'
            '<stop offset="1" stop-color="black"/></linearGradient></defs>'
            + letter("A", attrs='fill="url(#g)"')
        ] * 2,
        2, 2, 23,
    ),
    "E3 child restores fill-opacity": (
        ['<g fill-opacity="0">' + letter("A", attrs="fill-opacity='1'") + "</g>"] * 2,
        2, 2, 23,
    ),
    "E4 child restores stroke-width": (
        [
            '<g stroke-width="0">'
            + letter("A", attrs="stroke-width='1' fill='none' stroke='black'")
            + "</g>"
        ] * 2,
        2, 2, 41,
    ),
    "E5 half opacity still paints": (
        ['<g fill-opacity="0.5">' + letter("A") + "</g>"] * 2, 2, 2, 1,
    ),
    "E6 fill none": ([letter("A", attrs='fill="none"')] * 2, 0, 0, 0),
    "E7 stroke none still fills": ([letter("A", attrs='stroke="none"')] * 2, 2, 2, 23),
    "E8 display NONE is visible": (
        ['<g display="NONE">' + letter("A") + "</g>"] * 2, 2, 2, 23,
    ),
    "E9 DISPLAY:none is visible": (
        ['<g style="DISPLAY:none">' + letter("A") + "</g>"] * 2, 2, 2, 23,
    ),
    "E10 visibility HIDDEN is visible": (
        ['<g visibility="HIDDEN">' + letter("A") + "</g>"] * 2, 2, 2, 23,
    ),
    "E11 !important spelling": (
        ['<g style="display:none !important">' + letter("A") + "</g>"] * 2, 0, 0, 0,
    ),
    "E12 !IMPORTANT spelling": (
        ['<g style="display:none !IMPORTANT">' + letter("A") + "</g>"] * 2, 0, 0, 0,
    ),
    "E13 '! important' spelling": (
        ['<g style="display:none ! important">' + letter("A") + "</g>"] * 2, 0, 0, 0,
    ),
    "E14 'none!important' spelling": (
        ['<g style="display:none!important">' + letter("A") + "</g>"] * 2, 0, 0, 0,
    ),
}


def main():
    print(f"bin={BIN}\nold={OLD}\nd1={RENDER}\ncairo={CAIRO}")
    # A missing input is a hard failure: returning the value of check() would
    # exit 0 because bool False is 0 (R56 P2.3).
    if not BIN.exists():
        check(False, f"binary exists: {BIN}")
        return 1
    if not OLD.exists():
        check(False, f"old binary exists: {OLD}")
        return 1
    if not RENDER.exists() and not skip("d1render available", str(RENDER)):
        return finish()
    if not CAIRO.exists() and not skip("cairosvg available", str(CAIRO)):
        return finish()

    rows = []
    for label, (inners, want_rc, before_rc, want_dark) in CASES.items():
        tag = label.split()[0]
        code, out, err = compose(BIN, inners, "new_" + tag)
        ok = code == want_rc
        if ok and want_rc == 2:
            ok = not out.exists() and "duplicate panel letters" in err
        if ok and want_rc == 0:
            ok = out.exists()
        check(ok, f"{label}: new rc {want_rc}", f"rc={code} {err.strip()[:120]}")
        old_code, _, _ = compose(OLD, inners, "old_" + tag)
        check(
            old_code == before_rc,
            f"{label}: pre-R54 rc {before_rc} (teeth)",
            f"old rc={old_code}",
        )
        if RENDER.exists():
            dark, nonwhite = render_d1(inners[0], tag)
        else:
            dark, nonwhite = None, None
        if CAIRO.exists():
            c_dark, c_nonwhite = render_cairo(inners[0], tag)
        else:
            c_dark, c_nonwhite = None, None
        if dark is not None:
            check(
                dark == want_dark,
                f"{label}: d1 dark ink {want_dark}",
                f"d1 dark={dark} nonwhite={nonwhite}",
            )
            if inners[0] == inners[1]:
                check(
                    (dark > 0) == (want_rc == 2),
                    f"{label}: d1 ink agrees with the verdict",
                    f"dark={dark} rc={want_rc}",
                )
        if c_dark is not None:
            if inners[0] == inners[1]:
                # cairo is a cross-check, not the judge: two documented
                # divergences follow resvg (see the module docstring).
                if tag in DIVERGENT:
                    print(f"note {label}: cairo differs, resvg governs "
                          f"(cairo dark={c_dark} nonwhite={c_nonwhite})")
                else:
                    check(
                        (c_nonwhite > 0) == (want_rc == 2),
                        f"{label}: cairo ink cross-check",
                        f"cairo dark={c_dark} nonwhite={c_nonwhite}",
                    )
        rows.append((label, before_rc, want_rc, dark, nonwhite, c_dark, c_nonwhite))

    print("\ncase | 修前 rc | 修后 rc | d1 dark | d1 nonwhite | cairo dark | cairo nonwhite")
    for row in rows:
        print(" | ".join(str(value) for value in row))
    return finish()


def finish():
    print(f"\n{len(PASSED)} passed, {len(FAILED)} failed, {len(SKIPPED)} skipped")
    if FAILED:
        for label in FAILED:
            print("  FAILED: " + label)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
