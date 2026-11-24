#!/usr/bin/env python3
"""R56 evidence: the label-letter judgement gaps R54 left behind.

Families
  P1  among equal ``!important`` declarations resvg 0.47 keeps the FIRST one
      (cairosvg keeps the last). Plain declarations still let the later one win
      and an ``!important`` one still outranks a plain one. The keyword stays
      case-insensitive, and a stray ``!`` (``display:none !``) is an important
      flag too.
  P2  a zero alpha channel paints nothing (``#rrggbbaa``, ``rgba(...)``, also on
      a gradient stop); a ``<symbol style="display:none">`` is still drawn
      through ``<use>``; a gradient ``href`` chain has no fixed depth; a marker
      covering nested markers counts one letter per drawn glyph.
  P3  the remaining known limits, each measured and either fixed or registered:
      a ``<symbol viewBox>`` that clips the text away (fixed), a trailing space
      in an opacity attribute (registered: resvg paints, we hide), a tiny but
      non-zero stroke width / fill opacity (registered: fail-closed), a
      zero-size pattern (registered: needs a bbox).

Ground truth: every expected rc/ink below was measured with resvg 0.47
(``d1render``) and cross-checked with cairosvg. Where the two disagree the
resvg answer governs, as R51 ruled; the divergences are listed in ``DIVERGENT``
and printed as notes instead of being asserted. ``SVG_GRID_OLD_BIN`` must point
at a pre-R56 build: the run asserts the *old wrong* rc for every case, so a
green run here cannot be a green run of the unfixed binary. A missing renderer
is a hard failure unless ``--allow-skip`` is passed.

Usage
  python3 docs/PITFALLS/round-data/R56/test.py
  SVG_GRID_BIN=/tmp/r56bin/svg_grid_before python3 ...   # must go red
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
OLD = pathlib.Path(os.environ.get("SVG_GRID_OLD_BIN") or "/tmp/r56bin/svg_grid_before")
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
CAIRO = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON") or "/home/nihao/.venv/bin/python")
FONTDIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="r56-"))
ALLOW_SKIP = "--allow-skip" in sys.argv

PASSED, FAILED, SKIPPED = [], [], []

# Cases where cairosvg disagrees with resvg; R51's ruling follows resvg.
#   P1a/P1d/P1e  cairo keeps the LAST equal !important, resvg the first
#   P1b/P1c/P1f  the same rule seen from the other side
#   P1k/P1l      cairo does not treat a stray "!" as an important flag
#   P2a/P2d      cairo does not parse the alpha byte of #rrggbbaa
#   P2f          cairo hides a display:none <symbol> instanced by <use>
#   P3b/P3c/P3d  cairo agrees with resvg (ink 0); we stay fail-closed
DIVERGENT = {
    "P1a", "P1b", "P1c", "P1d", "P1e", "P1f", "P1k", "P1l",
    "P2a", "P2d", "P2f", "P3b", "P3c", "P3d",
}

# Cases whose verdict deliberately does NOT follow the ink, because matching the
# renderer would need machinery this round does not add. Each one is registered
# in the R56 report; the ink is still asserted, only the "ink agrees with the
# verdict" cross-check is replaced by a note.
#   P3a  resvg paints (a trailing space makes the attribute invalid); we hide.
#   P3b/P3c  a tiny but non-zero width/opacity leaves no pixel; we count it.
#   P3d  a zero-size pattern draws nothing; telling that apart needs a bbox.
REGISTERED = {"P3a", "P3b", "P3c", "P3d"}


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


def group(style, inner):
    return f'<g style="{style}">{inner}</g>'


def symbol(view_box, inner, extra=""):
    return (
        f'<defs><symbol id="s" viewBox="{view_box}" data-svg-grid-label="0">'
        f"{inner}</symbol></defs><use href=\"#s\"{extra}/>"
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
    out, previous = [], bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        filter_type = raw[start]
        line = bytearray(raw[start + 1 : start + 1 + stride])
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            up_left = previous[i - channels] if i >= channels else 0
            if filter_type == 1:
                line[i] = (line[i] + left) & 0xFF
            elif filter_type == 2:
                line[i] = (line[i] + up) & 0xFF
            elif filter_type == 3:
                line[i] = (line[i] + (left + up) // 2) & 0xFF
            elif filter_type == 4:
                estimate = left + up - up_left
                pa, pb, pc = (
                    abs(estimate - left),
                    abs(estimate - up),
                    abs(estimate - up_left),
                )
                predictor = left if pa <= pb and pa <= pc else (up if pb <= pc else up_left)
                line[i] = (line[i] + predictor) & 0xFF
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
A = letter("A")
CHAIN = (
    '<defs><linearGradient id="g0"><stop offset="0" stop-color="black"/>'
    '<stop offset="1" stop-color="black"/></linearGradient>'
    + "".join(f'<linearGradient id="g{i}" href="#g{i - 1}"/>' for i in range(1, 11))
    + "</defs>"
)
ZERO_GRADIENT = (
    '<defs><linearGradient id="g"><stop offset="0" stop-color="{}"/>'
    '<stop offset="1" stop-color="{}"/></linearGradient></defs>'
)

CASES = {
    # P1: the !important cascade
    "P1a first important block wins": (
        [group("display:block !important;display:none !important", A)] * 2, 2, 0, 23,
    ),
    "P1b first important none wins": (
        [group("display:none !important;display:block !important", A)] * 2, 0, 2, 0,
    ),
    "P1c first important, not first declaration": (
        [group("display:none !important;display:block;display:block !important", A)] * 2,
        0, 2, 0,
    ),
    "P1d important keyword is case insensitive": (
        [group("display:block !important;display:none !IMPORTANT", A)] * 2, 2, 0, 23,
    ),
    "P1e first important visibility wins": (
        [group("visibility:visible !important;visibility:hidden !important", A)] * 2,
        2, 0, 23,
    ),
    "P1f first important visibility hides": (
        [group("visibility:hidden !important;visibility:visible !important", A)] * 2,
        0, 2, 0,
    ),
    "P1g later plain block wins (control)": (
        [group("display:none;display:block", A)] * 2, 2, 2, 23,
    ),
    "P1h important none beats plain block (control)": (
        [group("display:none !important;display:block", A)] * 2, 0, 0, 0,
    ),
    "P1i important block beats plain none (control)": (
        [group("display:none;display:block !important", A)] * 2, 2, 2, 23,
    ),
    "P1j later plain none wins (control)": (
        [group("display:block;display:none", A)] * 2, 0, 0, 0,
    ),
    "P1k stray bang is important": ([group("display:none !", A)] * 2, 0, 2, 0),
    "P1l stray bang with junk is important": (
        [group("display:none ! foo", A)] * 2, 0, 2, 0,
    ),
    # P2: alpha, symbol display, gradient chain, covering marker
    "P2a hex8 zero alpha": ([letter("A", attrs='fill="#00000000"')] * 2, 0, 2, 0),
    "P2b rgba zero alpha": ([letter("A", attrs='fill="rgba(0,0,0,0)"')] * 2, 0, 2, 0),
    "P2c gradient stop rgba zero alpha": (
        [ZERO_GRADIENT.format("rgba(0,0,0,0)", "rgba(0,0,0,0)")
         + letter("A", attrs='fill="url(#g)"')] * 2,
        0, 2, 0,
    ),
    "P2d gradient stop hex8 zero alpha": (
        [ZERO_GRADIENT.format("#00000000", "#00000000")
         + letter("A", attrs='fill="url(#g)"')] * 2,
        0, 2, 0,
    ),
    "P2e half alpha still paints": ([letter("A", attrs='fill="#00000080"')] * 2, 2, 2, 1),
    "P2f display none on a symbol is drawn": (
        ['<defs><symbol id="s" style="display:none" data-svg-grid-label="0">'
         + A + "</symbol></defs><use href=\"#s\"/>"] * 2,
        2, 0, 23,
    ),
    "P2g ten deep gradient href chain": (
        [CHAIN + letter("A", attrs='fill="url(#g10)"')] * 2, 2, 0, 23,
    ),
    "P2h circular gradient href paints nothing": (
        ['<defs><linearGradient id="a" href="#b"/><linearGradient id="b" href="#a"/></defs>'
         + letter("A", attrs='fill="url(#a)"')] * 2,
        0, 0, 0,
    ),
    "P2i covering marker counts two glyphs": (
        ['<g data-svg-grid-label="0">' + A + letter("A", "1", x="20") + "</g>"] * 2,
        2, 2, 46,
    ),
    "P2k covering marker counts two different glyphs": (
        ['<g data-svg-grid-label="0">' + A + letter("B", "1", x="20") + "</g>"] * 2,
        2, 2, 49,
    ),
    # P3: measured limits
    "P3a trailing space in fill-opacity (registered)": (
        [letter("A", attrs='fill-opacity=" 0 "')] * 2, 0, 0, 23,
    ),
    "P3b tiny stroke width (registered)": (
        [letter("A", attrs='fill="none" stroke="black" stroke-width="1e-9"')] * 2,
        2, 2, 0,
    ),
    "P3c tiny fill opacity (registered)": (
        [letter("A", attrs='fill-opacity="1e-4"')] * 2, 2, 2, 0,
    ),
    "P3d zero size pattern (registered)": (
        ['<defs><pattern id="p" width="0" height="0" patternUnits="userSpaceOnUse">'
         '<rect width="4" height="4" fill="black"/></pattern></defs>'
         + letter("A", attrs='fill="url(#p)"')] * 2,
        2, 2, 0,
    ),
    "P3e symbol viewBox clips the text away": (
        [symbol("0 0 6 6", '<text x="4" y="20" font-size="12" font-family="Arial">A</text>')]
        * 2,
        0, 2, 0,
    ),
    "P3f symbol viewBox keeps the text": (
        [symbol("0 0 20 20", '<text x="4" y="20" font-size="12" font-family="Arial">A</text>')]
        * 2,
        2, 2, 91,
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
            f"{label}: pre-R56 rc {before_rc} (teeth)",
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
                if tag in REGISTERED:
                    print(f"note {label}: registered divergence, the verdict does "
                          f"not follow the ink (d1 dark={dark} rc={want_rc})")
                else:
                    check(
                        (dark > 0) == (want_rc == 2),
                        f"{label}: d1 ink agrees with the verdict",
                        f"dark={dark} rc={want_rc}",
                    )
        if c_dark is not None:
            if inners[0] == inners[1]:
                # cairo is a cross-check, not the judge: the documented
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
