#!/usr/bin/env python3
"""R55 evidence: `patternTransform` / `gradientTransform` must follow the panel scale.

Families
  A  a user-space pattern transform is conjugated with the panel scale
     (`T' = S·T·S⁻¹`): under an isotropic scale that multiplies translation
     vectors and a `rotate(a cx cy)` centre, under an anisotropic scale a pure
     translation stays `translate(...)` and a rotation/shear becomes `matrix(...)`.
     `scale()`/`skew()` under an isotropic scale commute with `S` and must keep
     their exact bytes.
  B  a user-space gradient transform follows the same rule (both gradient
     kinds); an object-bounding-box gradient keeps it verbatim.
  C  the parser (six keywords, commas/whitespace, negatives, exponents, chained
     segments, unparsable values kept verbatim with a W7 warning) and the
     byte-identity of a transform that does not need rewriting.

Ground truth: every expected attribute string and every pixel number below was
measured with resvg 0.47 (`d1render`) against the oracle "render the *source*
panel at the target size" (isotropic: the source rendered at `s`; anisotropic:
`viewBox` + `preserveAspectRatio="none"`, so the whole source is scaled by
`(sx, sy)`); cairosvg is a directional cross-check only — it has an intrinsic
pattern floor even with no transform at all, so a cairo row is compared
*before vs after*, never to 0. The anisotropic *rotation* case cannot be exact
(a non-orthogonal pattern lattice is not expressible by one `patternTransform`);
its residual is printed and asserted only as "much smaller than verbatim".

`SVG_GRID_OLD_BIN` must point at a pre-R55 build (`/tmp/r50base/...`): the run
asserts the old verbatim value for every rewritten case, so pointing the tested
`SVG_GRID_BIN` at that binary turns the run red -- a green run cannot come from
the unfixed tool. A missing renderer is a hard failure unless `--allow-skip`.

Usage
  python3 docs/PITFALLS/round-data/R55/test.py
  SVG_GRID_BIN=/tmp/r50base/target/release/svg_grid python3 ...   # must go red
  python3 ... --allow-skip                                        # no renderer here
"""

import os
import pathlib
import re
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "release" / "svg_grid")
OLD = pathlib.Path(
    os.environ.get("SVG_GRID_OLD_BIN") or "/tmp/r50base/target/release/svg_grid"
)
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
CAIRO = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON") or "/home/nihao/.venv/bin/python")
FONTDIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="r55-"))
ALLOW_SKIP = "--allow-skip" in sys.argv

PASSED, FAILED, SKIPPED = [], [], []


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


def panel(width, height, body):
    """A source panel in the tag protocol: panel box + a scaling group."""
    return (
        f'<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
        'xmlns="http://www.w3.org/2000/svg">'
        f'<rect data-panel-box="main" x="0" y="0" width="{width}" height="{height}" '
        'fill="none" stroke="none"/>'
        f'<g data-scale="xy">{body}</g></svg>'
    )


def compose(binary, width, height, body, cell, tag, verbose=False):
    """Compose the panel into a 1-column, margin-free canvas of `cell` size."""
    source = write(f"{tag}.src.svg", panel(width, height, body))
    out = TMP / f"{tag}.out.svg"
    argv = [
        "plot-grid", "--input", str(source), "--output", str(out),
        "--width", str(cell[0]), "--height", str(cell[1]),
        "--ncol", "1", "--plot-margin", "0",
    ]
    if verbose:
        argv.append("--verbose")
    result = subprocess.run([str(binary), *argv], capture_output=True, text=True, cwd=ROOT)
    text = out.read_text(encoding="utf-8") if out.exists() else ""
    return text, result

def oracle_png(width, height, body, cell, tag):
    """The oracle: the whole *source* panel scaled to the cell, per the ruling."""
    svg = write(
        f"{tag}.oracle.svg",
        f'<svg width="{cell[0]}" height="{cell[1]}" viewBox="0 0 {width} {height}" '
        'preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg">'
        f"{body}</svg>",
    )
    png = TMP / f"{tag}.oracle.png"
    env = dict(os.environ)
    env["D1_FONT_DIR"] = str(FONTDIR)
    result = subprocess.run([str(RENDER), str(svg), str(png), "1"], capture_output=True, env=env)
    if result.returncode != 0 or not png.exists():
        raise RuntimeError(f"d1render failed: {result.stderr[-300:]!r}")
    return png


def render_product(svg_text, tag):
    svg = write(f"{tag}.product.svg", svg_text)
    png = TMP / f"{tag}.product.png"
    env = dict(os.environ)
    env["D1_FONT_DIR"] = str(FONTDIR)
    result = subprocess.run([str(RENDER), str(svg), str(png), "1"], capture_output=True, env=env)
    if result.returncode != 0 or not png.exists():
        raise RuntimeError(f"d1render failed: {result.stderr[-300:]!r}")
    return png


def render_cairo(svg_text, tag):
    svg = write(f"{tag}.cairo.svg", svg_text)
    png = TMP / f"{tag}.cairo.png"
    code = "import cairosvg, sys; cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2])"
    result = subprocess.run([str(CAIRO), "-c", code, str(svg), str(png)], capture_output=True)
    return png if result.returncode == 0 and png.exists() else None


# --- PNG reading / comparison (8-bit RGB/RGBA, filters 0-4) -------------------


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


def rgb_pixels(path):
    width, height, channels, rows = read_png(path)
    pixels = []
    for y in range(height):
        for x in range(width):
            pixels.append(rows[y][x * channels : x * channels + 3])
    return width, height, pixels


def diff(left, right):
    """(different pixels, max channel difference); the sizes must match."""
    lw, lh, lp = rgb_pixels(left)
    rw, rh, rp = rgb_pixels(right)
    if (lw, lh) != (rw, rh):
        return lw * lh, 255
    xor = maxdiff = 0
    for a, b in zip(lp, rp):
        delta = max(abs(x - y) for x, y in zip(a, b))
        if delta:
            xor += 1
            maxdiff = max(maxdiff, delta)
    return xor, maxdiff


def ink(path):
    _, _, pixels = rgb_pixels(path)
    return sum(1 for pixel in pixels if max(pixel) < 200)


# --- cases -------------------------------------------------------------------

PATTERN = (
    '<defs><pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" '
    'width="10" height="10" patternTransform="{transform}"{extra}>'
    '<rect x="0" y="0" width="4" height="3" fill="#000000"/></pattern></defs>'
    '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>'
)

NESTED = (
    '<defs><pattern id="inner" patternUnits="userSpaceOnUse" x="0" y="0" '
    'width="8" height="8" patternTransform="translate(2 2)">'
    '<rect x="0" y="0" width="3" height="2" fill="#000000"/></pattern>'
    '<pattern id="outer" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20" '
    'patternTransform="translate(4 4)">'
    '<rect x="0" y="0" width="20" height="20" fill="url(#inner)"/></pattern></defs>'
    '<rect x="5" y="5" width="75" height="55" fill="url(#outer)"/>'
)

GRADIENT = (
    '<defs><{kind} id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="0" '
    'gradientTransform="{transform}">'
    '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ff0000"/>'
    '</{kind}></defs>'
    '<rect x="0" y="0" width="{w}" height="{h}" fill="url(#g)"/>'
)

OBB_PATTERN = (
    '<defs><pattern id="p" patternUnits="objectBoundingBox" '
    'patternContentUnits="objectBoundingBox" width="0.2" height="0.2" '
    'patternTransform="{transform}">'
    '<rect x="0" y="0" width="0.05" height="0.04" fill="#000000"/></pattern></defs>'
    '<rect x="0" y="0" width="{w}" height="{h}" fill="url(#p)"/>'
)

VIEWBOX_PATTERN = (
    '<defs><pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" '
    'width="10" height="10" viewBox="0 0 10 10" patternTransform="{transform}">'
    '<rect x="0" y="0" width="4" height="3" fill="#000000"/></pattern></defs>'
    '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>'
)

OBB_GRADIENT = (
    '<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0" '
    'gradientTransform="{transform}">'
    '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ff0000"/>'
    '</linearGradient></defs>'
    '<rect x="0" y="0" width="{w}" height="{h}" fill="url(#g)"/>'
)

# name: (panel (w,h), cell (w,h), body, expected attribute value or None,
#        pre-R55 value, verdict)
# verdict: "exact"   -- d1 must be xor 0 / maxdiff 0
#          ("residual", xor_limit, maxdiff_limit) -- d1 must be within
#          ("keep",)   -- nothing may change, d1 must be xor 0
CASES = [
    # A: user-space patterns
    ("A1 pattern iso translate", (100, 80), (200, 160),
     PATTERN.format(transform="translate(3 4)", extra=""),
     "translate(6, 8)", "translate(3 4)", "exact"),
    ("A2 pattern iso rotate centre", (100, 80), (200, 160),
     PATTERN.format(transform="rotate(30 3 4)", extra=""),
     "rotate(30, 6, 8)", "rotate(30 3 4)", "exact"),
    ("A3 pattern iso matrix e/f", (100, 80), (200, 160),
     PATTERN.format(transform="matrix(1 0 0 1 3 4)", extra=""),
     "matrix(1 0 0 1 6 8)", "matrix(1 0 0 1 3 4)", "exact"),
    ("A4 pattern iso scale kept", (100, 80), (200, 160),
     PATTERN.format(transform="scale(2 3)", extra=""),
     "scale(2 3)", "scale(2 3)", "keep"),
    ("A5 pattern iso skew kept", (100, 80), (200, 160),
     PATTERN.format(transform="skewX(20)", extra=""),
     "skewX(20)", "skewX(20)", "keep"),
    ("A6 pattern aniso translate", (60, 40), (240, 200),
     PATTERN.format(transform="translate(3 4)", extra=""),
     "translate(12, 20)", "translate(3 4)", "exact"),
    ("A7 pattern aniso rotate", (60, 40), (240, 200),
     PATTERN.format(transform="rotate(30)", extra=""),
     "matrix(0.866 0.625 -0.400 0.866 0 0)", "rotate(30)", ("residual", 2000, 255)),
    ("A8 pattern obb tile", (60, 40), (120, 80),
     OBB_PATTERN.format(transform="translate(3 4)", w=60, h=40),
     "translate(6, 8)", "translate(3 4)", "exact"),
    ("A9 pattern viewBox parent", (100, 80), (200, 160),
     VIEWBOX_PATTERN.format(transform="translate(3 4)"),
     "translate(6, 8)", "translate(3 4)", "exact"),
    ("A10 nested patterns scale each", (100, 80), (200, 160), NESTED,
     ("translate(4, 4)", "translate(8, 8)"),
     "translate(2 2)|translate(4 4)", "exact"),
    ("A11 pattern without transform", (100, 80), (200, 160),
     PATTERN.format(transform="translate(0 0)", extra="").replace(
         ' patternTransform="translate(0 0)"', ""),
     None, None, "exact"),
    # B: user-space gradients
    ("B1 linear iso translate", (100, 80), (200, 160),
     GRADIENT.format(kind="linearGradient", transform="translate(3 4)", w=100, h=80),
     "translate(6, 8)", "translate(3 4)", "exact"),
    ("B2 radial iso translate", (100, 80), (200, 160),
     GRADIENT.format(kind="radialGradient", transform="translate(3 4)", w=100, h=80),
     "translate(6, 8)", "translate(3 4)", "exact"),
    ("B3 linear aniso translate", (60, 40), (240, 200),
     GRADIENT.format(kind="linearGradient", transform="translate(3 4)", w=60, h=40),
     "translate(12, 20)", "translate(3 4)", "exact"),
    ("B4 linear aniso rotate", (60, 40), (240, 200),
     GRADIENT.format(kind="linearGradient", transform="rotate(30)", w=60, h=40),
     "matrix(0.866 0.625 -0.400 0.866 0 0)", "rotate(30)", ("residual", 40, 2)),
    ("B5 obb gradient kept", (60, 40), (240, 200),
     OBB_GRADIENT.format(transform="rotate(45)", w=60, h=40),
     "rotate(45)", "rotate(45)", "keep"),
    ("B6 gradient without transform", (60, 40), (240, 200),
     OBB_GRADIENT.format(transform="rotate(0)", w=60, h=40).replace(
         ' gradientTransform="rotate(0)"', "").replace(
         ' gradientUnits="userSpaceOnUse"', ""),
     None, None, "exact"),
]

ATTR = {
    "pattern": re.compile(r'patternTransform="([^"]*)"'),
    "gradient": re.compile(r'gradientTransform="([^"]*)"'),
}


def expected_attr(name):
    return ATTR["gradient" if name.startswith("B") else "pattern"]


def main():
    print(f"bin={BIN}\nold={OLD}\nd1={RENDER}\ncairo={CAIRO}")
    if not BIN.exists():
        return check(False, f"binary exists: {BIN}")
    if not OLD.exists():
        return check(False, f"old binary exists: {OLD}")
    if not RENDER.exists() and not skip("d1render available", str(RENDER)):
        return finish()
    cairo_ok = CAIRO.exists() and subprocess.run(
        [str(CAIRO), "-c", "import cairosvg"], capture_output=True
    ).returncode == 0
    if not cairo_ok and not skip("cairosvg available", str(CAIRO)):
        return finish()

    rows = []
    for name, panel_size, cell, body, want, before, verdict in CASES:
        tag = name.split()[0]
        new_svg, new_run = compose(BIN, *panel_size, body, cell, f"new_{tag}")
        old_svg, old_run = compose(OLD, *panel_size, body, cell, f"old_{tag}")
        check(new_run.returncode == 0, f"{name}: compose rc 0", new_run.stderr.strip()[:120])
        pattern = expected_attr(name)
        found = pattern.findall(new_svg)
        old_found = pattern.findall(old_svg)
        if want is None:
            check(not found and "Transform" not in new_svg,
                  f"{name}: no attribute is invented", new_svg[:120])
        elif isinstance(want, tuple):
            check(sorted(found) == sorted(want),
                  f"{name}: both nesting levels conjugated",
                  f"found={found} want={list(want)}")
        else:
            check(found == [want], f"{name}: attribute is {want}", f"found={found}")
        if before is None:
            check(not old_found, f"{name}: pre-R55 wrote no attribute", f"old={old_found}")
        elif isinstance(before, str) and "|" in before:
            check(sorted(old_found) == sorted(before.split("|")),
                  f"{name}: pre-R55 kept both transforms verbatim (teeth)",
                  f"old={old_found}")
        elif before is not None:
            check(old_found == [before],
                  f"{name}: pre-R55 kept {before} (teeth)", f"old={old_found}")

        oracle = oracle_png(*panel_size, body, cell, tag)
        after_png = render_product(new_svg, f"after_{tag}")
        before_png = render_product(old_svg, f"before_{tag}")
        xor_after, max_after = diff(oracle, after_png)
        xor_before, max_before = diff(oracle, before_png)
        check(ink(oracle) > 0 and ink(after_png) > 0,
              f"{name}: the oracle and the product both have ink",
              f"ink={ink(oracle)}/{ink(after_png)}")
        if verdict == "exact":
            check((xor_after, max_after) == (0, 0),
                  f"{name}: d1 matches the oracle exactly", f"xor={xor_after} md={max_after}")
        elif verdict == "keep":
            check((xor_after, max_after) == (0, 0) and xor_before == 0,
                  f"{name}: d1 already matched and still matches",
                  f"before={xor_before}/{max_before} after={xor_after}/{max_after}")
        else:
            limit, md_limit = verdict[1], verdict[2]
            check(xor_after <= limit and max_after <= md_limit,
                  f"{name}: d1 residual within {limit}",
                  f"xor={xor_after} md={max_after}")
            check(xor_before > limit,
                  f"{name}: pre-R55 was far worse (teeth)",
                  f"before={xor_before}/{max_before}")
        cairo_before = cairo_after = None
        if cairo_ok:
            cb = render_cairo(old_svg, f"cbefore_{tag}")
            ca = render_cairo(new_svg, f"cafter_{tag}")
            if cb and ca:
                cairo_before = diff(oracle, cb)[0]
                cairo_after = diff(oracle, ca)[0]
                check(cairo_after <= cairo_before,
                      f"{name}: cairo no worse than pre-R55",
                      f"before={cairo_before} after={cairo_after}")
        rows.append((name, xor_before, max_before, xor_after, max_after,
                     cairo_before, cairo_after))

    # D: object-bounding-box gradients are NOT conjugated; show the teeth of
    # that "keep" ruling by rendering a hand-conjugated variant.
    tag = "D1"
    body_keep = OBB_GRADIENT.format(transform="rotate(45)", w=60, h=40)
    keep_svg, _ = compose(BIN, 60, 40, body_keep, (240, 200), f"new_{tag}")
    conj_svg = keep_svg.replace("rotate(45)", "matrix(0.707 0.884 -0.566 0.707 0 0)")
    oracle = oracle_png(60, 40, body_keep, (240, 200), tag)
    xor_keep, _ = diff(oracle, render_product(keep_svg, f"keep_{tag}"))
    xor_conj, _ = diff(oracle, render_product(conj_svg, f"conj_{tag}"))
    check(xor_keep == 0, "D1 obb gradient keeps rotate(45) (xor 0)", f"xor={xor_keep}")
    check(xor_conj > 10000, "D1 conjugating an obb gradient is wrong (teeth)",
          f"xor={xor_conj}")

    # E: parser and byte-identity, no rendering needed.
    parser_cases = [
        ("E1 no-space chain", "rotate(30)translate(1 2)", (100, 80), (200, 160),
         "rotate(30) translate(2, 4)"),
        ("E2 commas and exponents", "translate(1e1, 2.5e0)", (100, 80), (200, 160),
         "translate(20, 5)"),
        ("E3 negative without space", "translate(3-4)", (100, 80), (200, 160),
         "translate(6, -8)"),
    ]
    for name, transform, panel_size, cell, want in parser_cases:
        body = PATTERN.format(transform=transform, extra="")
        svg, _ = compose(BIN, *panel_size, body, cell, name.split()[0])
        found = ATTR["pattern"].findall(svg)
        check(found == [want], f"{name}: {want}", f"found={found}")
    for name, transform in [
        ("E4 unparsable keyword", "wobble(1)"),
        ("E5 matrix with five numbers", "matrix(1 0 0 1 3)"),
        ("E6 unbalanced parenthesis", "translate(3"),
    ]:
        body = PATTERN.format(transform=transform, extra="")
        svg, run = compose(BIN, 100, 80, body, (200, 160), name.split()[0], verbose=True)
        found = ATTR["pattern"].findall(svg)
        check(found == [transform], f"{name}: kept verbatim", f"found={found}")
        check("patternTransform" in run.stderr and "verbatim" in run.stderr,
              f"{name}: W7 passthrough warning", run.stderr.strip().splitlines()[0][:120])
    # A transform at scale 1 keeps its exact bytes (nothing is reformatted).
    for name, transform in [
        ("E7 scale 1 translate", "translate(3 4) rotate(30)"),
        ("E8 scale 1 matrix", "matrix(1 0 0 1 3,4)"),
    ]:
        body = PATTERN.format(transform=transform, extra="")
        svg, _ = compose(BIN, 100, 80, body, (100, 80), name.split()[0])
        check(f'patternTransform="{transform}"' in svg,
              f"{name}: untouched at scale 1", svg[:120])

    print("\ncase | d1 修前 xor/md | d1 修后 xor/md | cairo 修前 xor | cairo 修后 xor")
    print("oracle = 源面板整体缩放到 cell（等向按 s；各向异性 viewBox+pAR=none），d1render ×1")
    for row in rows:
        name, xb, mb, xa, ma, cb, ca = row
        print(f"{name} | {xb}/{mb} | {xa}/{ma} | {cb} | {ca}")
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
