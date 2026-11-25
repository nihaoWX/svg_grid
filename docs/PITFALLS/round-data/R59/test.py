#!/usr/bin/env python3
"""R59 evidence — the four residuals the R58 review left behind.

Families
  E1  `feSpotLight x/y/z/pointsAtX/pointsAtY/pointsAtZ` are user lengths, not
      angles (`feDistantLight azimuth/elevation` are the angles and must stay).
      Isotropic s=2 must be exact; anisotropy (sx=4/sy=5) has a *named* residual:
      the lighting normal itself changes under a non-uniform scale.
  E2  `data-scale="x"` / `"y"` must still rewrite a scalar
      `<number-optional-number>` (`stdDeviation`, `radius`, `baseFrequency`):
      the missing axis factor is 1, so `2` becomes `8 2` under x and `2 10`
      under y, and a two-number source is read per axis.
  E3  a malformed primitive length (`stdDeviation="2px"`, `"abc"`, ...) is kept
      verbatim with W7; rc stays 0.  At cad0bef the same input was rc=2, i.e.
      R58 turned a tolerated value into a failed run.
  E4  an `objectBoundingBox` `<clipPath>` body keeps its fractional font-size.
      R59 fixed only the font-size gate and registered the pre-existing obb
      coordinate scaling in meta.toml; R67-B fixed that scaling too (the whole
      body is verbatim now), so E4 asserts the pixel oracle source@2 == after.
  E5  the corrected defs comment must not keep the unreproducible "xor 840".

Teeth and the before binary
  A before binary is built from cad0bef (the R58 HEAD) and must show the bug
  (E1/E2 pixel xor, E3 rc=2, E4 font-size).  It is not /tmp/r51head and not the
  R50 baseline: the regression is R58's.  Build it with:
      git archive cad0bef | tar -x -C /tmp/r59before
      cp -r fontlib /tmp/r59before/ && (cd /tmp/r59before && cargo build --release --offline)
  Default path: /tmp/r59before/target/release/svg_grid (SVG_GRID_BEFORE_BIN).

Arbitration: d1render.  D1_FONT_DIR must point at fontlib or text ink is 0.
cairosvg is recorded for reference only: it ignores filters and does not paint
text inside clipPath.

Run:  python3 docs/PITFALLS/round-data/R59/test.py
      python3 docs/PITFALLS/round-data/R59/test.py --allow-skip  # missing tools skip
Exit: 0 = every check passed, 1 = at least one check failed (fail-closed: a
      missing binary, before binary or renderer is a failure unless --allow-skip).
"""

import argparse
import os
import re
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
BEFORE = Path(os.environ.get("SVG_GRID_BEFORE_BIN", "/tmp/r59before/target/release/svg_grid"))
RENDER = Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO = Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(Path.home() / ".venv/bin/python")))
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r59-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))

PASSED, FAILED, SKIPPED, ROWS = [], [], [], []
ALLOW_SKIP = False
COUNTER = [0]


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label + ((" — " + str(detail)) if detail and not ok else ""))
    return ok


def skip(label, why):
    if ALLOW_SKIP:
        SKIPPED.append(label)
        print("SKIP " + label + " — " + why)
        return True
    return check(False, label, "skipped (" + why + ") without --allow-skip")


def slug(name):
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", name)


def tmp(name):
    COUNTER[0] += 1
    return TMP / ("%03d-%s" % (COUNTER[0], slug(name)))


# --------------------------------------------------------------------------- #
# PNG reading (8-bit RGB/RGBA) + ink/xor, identical to R58
# --------------------------------------------------------------------------- #
def read_png(path):
    data = Path(path).read_bytes()
    pos, rows, w, h, ch = 8, [], 0, 0, 0
    while pos + 8 <= len(data):
        ln, kind = struct.unpack(">I4s", data[pos:pos + 8])
        payload = data[pos + 8:pos + 8 + ln]
        pos += 12 + ln
        if kind == b"IHDR":
            w, h, _, colour = struct.unpack(">IIBB", payload[:10])
            ch = {0: 1, 2: 3, 4: 2, 6: 4}[colour]
        elif kind == b"IDAT":
            rows.append(payload)
        elif kind == b"IEND":
            break
    raw = zlib.decompress(b"".join(rows))
    stride = w * ch
    out, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f = raw[i]
        i += 1
        line = bytearray(raw[i:i + stride])
        i += stride
        if f == 1:
            for x in range(ch, stride):
                line[x] = (line[x] + line[x - ch]) & 255
        elif f == 2:
            for x in range(stride):
                line[x] = (line[x] + prev[x]) & 255
        elif f == 3:
            for x in range(stride):
                a = line[x - ch] if x >= ch else 0
                line[x] = (line[x] + ((a + prev[x]) >> 1)) & 255
        elif f == 4:
            for x in range(stride):
                a = line[x - ch] if x >= ch else 0
                b, c = prev[x], prev[x - ch] if x >= ch else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        out.append(bytes(line))
        prev = line
    return w, h, ch, out


def ink(path):
    w, h, ch, rows = read_png(path)
    n = 0
    for row in rows:
        for x in range(w):
            px = row[x * ch:(x + 1) * ch]
            if px[0] < 128 and px[1] < 128 and px[2] < 128:
                n += 1
    return n


def xor(a, b):
    wa, ha, ca, ra = read_png(a)
    wb, hb, cb, rb = read_png(b)
    if (wa, ha) != (wb, hb):
        return ("SIZE", "size %s vs %s" % ((wa, ha), (wb, hb)))
    n = md = 0
    for y in range(ha):
        for x in range(wa):
            pa = ra[y][x * ca:(x + 1) * ca][:3]
            pb = rb[y][x * cb:(x + 1) * cb][:3]
            d = max(abs(pa[i] - pb[i]) for i in range(3))
            if d:
                n += 1
                md = max(md, d)
    return (n, md)


# --------------------------------------------------------------------------- #
# SVG / renderer helpers
# --------------------------------------------------------------------------- #
TAG_RE = re.compile(r'<([A-Za-z][\w:-]*)((?:\s+[\w:-]+\s*=\s*"[^"]*")*)\s*/?>')
ATTR_RE = re.compile(r'([\w:-]+)\s*=\s*"([^"]*)"')


def elements(svg, tag):
    return [dict(ATTR_RE.findall(m.group(2))) for m in TAG_RE.finditer(svg) if m.group(1) == tag]


def element(svg, tag):
    found = elements(svg, tag)
    return found[0] if found else None


def render(svg_path, png_path, scale):
    r = subprocess.run([str(RENDER), str(svg_path), str(png_path), str(scale)],
                       env=ENV, capture_output=True, text=True)
    return r.returncode == 0


def compose(binary, src, cell, out, verbose=False):
    cmd = [str(binary), "plot-grid", "--input", str(src), "--output", str(out),
           "--width", str(cell[0]), "--ncol", "1", "--plot-margin", "0"]
    if cell[1] is not None:
        cmd += ["--height", str(cell[1])]
    if verbose:
        cmd.append("--verbose")
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stderr


def cairo(svg_path, png_path, scale):
    code = ("import cairosvg,sys;"
            "cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2], scale=float(sys.argv[3]))")
    try:
        r = subprocess.run([str(CAIRO), "-c", code, str(svg_path), str(png_path), str(scale)],
                           capture_output=True, text=True)
    except OSError:
        return "no-cairo"
    return "no" if r.returncode else "ok"


WHITE = '<rect width="100" height="80" fill="white"/>'


def source_svg(body, policy="xy", width=100, height=80):
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
            'viewBox="0 0 %d %d">%s<g data-scale="%s">%s</g></svg>'
            % (width, height, width, height, WHITE, policy, body))


def oracle_svg(body, sx, sy, width=100, height=80):
    # The cell here is always 400x400 (a 100x80 panel stretched 4x/5x), so the
    # oracle canvas is the cell, not sx*width by sy*height: under a one-axis
    # policy the untouched axis has no content there either.
    cw = ch = 400
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
            'viewBox="0 0 %d %d"><rect width="%d" height="%d" fill="white"/>'
            '<g transform="scale(%s %s)"><rect width="%d" height="%d" fill="white"/>%s</g></svg>'
            % (cw, ch, cw, ch, cw, ch, sx, sy, width, height, body))


def row(name, before, after, oracle, d1, cairo_note):
    ROWS.append((name, before, after, oracle, d1, cairo_note))


def pair(value):
    if value is None:
        return "n/a"
    if isinstance(value[0], int):
        return "%d/%d" % value
    return str(value)


def pixels(name, body, cell, oracle_body, sx, sy, policy="xy"):
    """Compose before/after, render both at 1, render the oracle, return both xor.

    Isotropic cases pass `oracle_body is None`: the oracle is the *source* at the
    cell scale (2), exactly as R58 defines it.  Anisotropic cases pass the source
    body wrapped in `<g transform="scale(sx sy)">` on a 400x400 canvas.
    """
    src = tmp(name + "-src.svg")
    src.write_text(source_svg(body, policy))
    outs, gots, xors = {}, {}, {}
    for tag, binary in (("before", BEFORE), ("after", BIN)):
        out = tmp(name + "-%s.svg" % tag)
        rc, err = compose(binary, src, cell, out)
        if rc != 0:
            return rc, err, None, None, None
        outs[tag] = out
        png = tmp(name + "-%s.png" % tag)
        if not render(out, png, 1):
            return rc, "render failed", None, None, None
        gots[tag] = png
    if oracle_body is None:
        oracle_svg_path = src
        oracle_scale = sx
    else:
        oracle_svg_path = tmp(name + "-oracle.svg")
        oracle_svg_path.write_text(oracle_svg(oracle_body, sx, sy))
        oracle_scale = 1
    oracle_png = tmp(name + "-oracle.png")
    if not render(oracle_svg_path, oracle_png, oracle_scale):
        return 0, "oracle render failed", None, None, None
    for tag in ("before", "after"):
        xors[tag] = xor(oracle_png, gots[tag])
    cairo_note = "ok" if cairo(outs["after"], tmp(name + "-cairo.png"), 1) == "ok" else "n/a"
    row(name, pair(xors["before"]), pair(xors["after"]),
        "ink %d" % ink(oracle_png), "d1", cairo_note)
    return 0, "", outs["after"].read_text(), xors, oracle_png


# --------------------------------------------------------------------------- #
# E1: feSpotLight positions are lengths; feDistantLight angles are not
# --------------------------------------------------------------------------- #
SPOT = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
        '<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">'
        '<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0" '
        'limitingConeAngle="30"/></feDiffuseLighting></filter></defs>'
        '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>')


def family_e1():
    print("\n── E1: feSpotLight x/y/z/pointsAt* are user lengths ──")
    rc, err, svg, xors, _ = pixels("E1-spot-iso", SPOT, (200, 160), None, 2, 2)
    if not check(rc == 0, "E1 iso: both binaries compose") or not check(svg is not None, "E1 iso: output"):
        return
    check(element(svg, "feSpotLight") is not None, "E1 iso: feSpotLight still present")
    check(xors["before"][0] > 1000, "E1 iso: the before binary is wrong (teeth)",
          pair(xors["before"]))
    check(xors["after"] == (0, 0), "E1 iso: xor 0 / md 0 after the fix", pair(xors["after"]))
    spot = element(svg, "feSpotLight") or {}
    for name, want in (("x", "40"), ("y", "40"), ("z", "20"), ("pointsAtX", "100"),
                       ("pointsAtY", "80"), ("pointsAtZ", "0"),
                       ("limitingConeAngle", "30")):
        check(spot.get(name) == want, "E1 iso: feSpotLight@%s == %s" % (name, want),
              spot.get(name))

    # Anisotropy: x/y/pointsAtX/Y per axis, z/pointsAtZ by the RMS factor the renderer
    # itself uses (10*sqrt((4^2+5^2)/2)=45.276928, not the geometric mean 44.721 which
    # R59 shipped and which cost 37011/206 pixels).  Full precision matters: the renderer
    # consumes the f32, and rounding to 45.277 flips 11 pixels.
    rc, err, svg, xors, _ = pixels("E1-spot-aniso", SPOT, (400, 400), SPOT, 4, 5)
    if check(rc == 0, "E1 aniso: both binaries compose") and check(svg is not None, "E1 aniso: output"):
        check(xors["before"][0] > 50000, "E1 aniso: the before binary is wrong (teeth)",
              pair(xors["before"]))
        spot = element(svg, "feSpotLight") or {}
        for name, want in (("x", "80"), ("y", "100"), ("z", "45.276928"),
                           ("pointsAtX", "200"), ("pointsAtY", "200"), ("pointsAtZ", "0"),
                           ("limitingConeAngle", "30")):
            check(spot.get(name) == want, "E1 aniso: feSpotLight@%s == %s" % (name, want),
                  spot.get(name))
        check(xors["after"][1] <= 5,
              "E1 aniso: the RMS factor lands on the oracle (no normal residual left)",
              "%s (expected 0/0; 37011/206 was the R59 geometric-mean bug)" % pair(xors["after"]))

    # The point light was fixed in R58; keep it in the same table.
    point = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
             '<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">'
             '<fePointLight x="20" y="20" z="10"/></feDiffuseLighting></filter></defs>'
             '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>')
    rc, err, svg, xors, _ = pixels("E1-point-iso", point, (200, 160), None, 2, 2)
    if check(rc == 0, "E1 point: composes") and check(svg is not None, "E1 point: output"):
        check(xors["after"] == (0, 0), "E1 point iso: xor 0 / md 0", pair(xors["after"]))
        light = element(svg, "fePointLight") or {}
        check((light.get("x"), light.get("y"), light.get("z")) == ("40", "40", "20"),
              "E1 point iso: x/y/z doubled", light)

    # feDistantLight really is angular: scaling it would move the sun.
    distant = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
               '<feDiffuseLighting><feDistantLight azimuth="45" elevation="20"/>'
               '</feDiffuseLighting></filter></defs>'
               '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>')
    rc, err, svg, xors, _ = pixels("E1-distant-iso", distant, (200, 160), None, 2, 2)
    if check(rc == 0, "E1 distant: composes") and check(svg is not None, "E1 distant: output"):
        light = element(svg, "feDistantLight") or {}
        check((light.get("azimuth"), light.get("elevation")) == ("45", "20"),
              "E1 distant: azimuth/elevation untouched", light)
        check(xors["after"] == (0, 0), "E1 distant iso: xor 0 / md 0", pair(xors["after"]))


# --------------------------------------------------------------------------- #
# E2: one-axis policy writes a pair
# --------------------------------------------------------------------------- #
def filter_body(inner):
    return ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
            '%s</filter></defs><rect x="10" y="10" width="80" height="60" fill="black" '
            'filter="url(#f)"/>' % inner)


E2_CASES = [
    ("E2-x-blur", '<feGaussianBlur stdDeviation="2"/>', "x", 4, 1, "stdDeviation", "8 2"),
    ("E2-y-blur", '<feGaussianBlur stdDeviation="2"/>', "y", 1, 5, "stdDeviation", "2 10"),
    ("E2-x-pair", '<feGaussianBlur stdDeviation="3 4"/>', "x", 4, 1, "stdDeviation", "12 4"),
    ("E2-y-pair", '<feGaussianBlur stdDeviation="3 4"/>', "y", 1, 5, "stdDeviation", "3 20"),
    ("E2-x-morph", '<feMorphology operator="dilate" radius="2"/>', "x", 4, 1, "radius", "8 2"),
    ("E2-y-morph", '<feMorphology operator="dilate" radius="2"/>', "y", 1, 5, "radius", "2 10"),
    ("E2-x-turb", '<feTurbulence baseFrequency="0.05" numOctaves="1" seed="3"/>',
     "x", 4, 1, "baseFrequency", "0.0125 0.05"),
]


def family_e2():
    print("\n── E2: data-scale=x/y must rewrite scalar filter lengths ──")
    for name, inner, policy, sx, sy, attr_name, want in E2_CASES:
        body = filter_body(inner)
        rc, err, svg, xors, _ = pixels(name, body, (400, 400), body, sx, sy, policy)
        if not check(rc == 0, "%s: composes" % name, err[-100:]) or not check(svg is not None, "%s: output" % name):
            continue
        tag = "feTurbulence" if "turb" in name else ("feMorphology" if "morph" in name else "feGaussianBlur")
        got = (element(svg, tag) or {}).get(attr_name)
        check(got == want, "%s: %s == %r" % (name, attr_name, want), got)
        check(xors["before"][0] > 300, "%s: before binary is wrong (teeth)" % name,
              pair(xors["before"]))
        check(xors["after"] == (0, 0), "%s: xor 0 / md 0" % name, pair(xors["after"]))


# --------------------------------------------------------------------------- #
# E3: malformed primitive length => verbatim + W7, rc 0
# --------------------------------------------------------------------------- #
E3_CASES = [
    ("E3-blur-px", '<feGaussianBlur stdDeviation="2px"/>', "feGaussianBlur", "stdDeviation", "2px"),
    ("E3-blur-abc", '<feGaussianBlur stdDeviation="abc"/>', "feGaussianBlur", "stdDeviation", "abc"),
    ("E3-offset-px", '<feOffset dx="2px" dy="5"/>', "feOffset", "dx", "2px"),
    ("E3-morph-px", '<feMorphology operator="dilate" radius="2px"/>', "feMorphology", "radius", "2px"),
    ("E3-turb-abc", '<feTurbulence baseFrequency="abc" numOctaves="1" seed="3"/>',
     "feTurbulence", "baseFrequency", "abc"),
]


def family_e3():
    print("\n── E3: malformed primitive lengths stay verbatim + W7, rc 0 ──")
    for name, inner, tag, attr_name, value in E3_CASES:
        src = tmp(name + "-src.svg")
        src.write_text(source_svg(filter_body(inner)))
        old_out, new_out = tmp(name + "-before.svg"), tmp(name + "-after.svg")
        old_rc, _ = compose(BEFORE, src, (200, 160), old_out)
        new_rc, err = compose(BIN, src, (200, 160), new_out, verbose=True)
        check(old_rc == 2, "%s: before binary fails the run (teeth)" % name, "rc=%d" % old_rc)
        check(new_rc == 0, "%s: rc stays 0" % name, "rc=%d" % new_rc)
        got = (element(new_out.read_text(), tag) or {}).get(attr_name) if new_out.is_file() else None
        check(got == value, "%s: %s kept verbatim" % (name, attr_name), got)
        check("could not be rewritten, kept verbatim" in err and value in err,
              "%s: W7 names the value" % name, err.strip().splitlines()[-1][:160] if err.strip() else "")
        row(name, "rc=%d" % old_rc, "rc=%d" % new_rc, "n/a (attribute oracle)", "d1", "n/a")

    # The untouched half of a pair must still scale: only the bad number is kept.
    src = tmp("E3-half-src.svg")
    src.write_text(source_svg(filter_body('<feOffset dx="2px" dy="5"/>')))
    out = tmp("E3-half-out.svg")
    rc, err = compose(BIN, src, (200, 160), out)
    offset = element(out.read_text(), "feOffset") or {} if out.is_file() else {}
    check(rc == 0 and offset.get("dx") == "2px" and offset.get("dy") == "10",
          "E3 half-parsed pair: dx kept, dy still doubled", offset)


# --------------------------------------------------------------------------- #
# E4: obb clipPath body keeps its fractional font-size
# --------------------------------------------------------------------------- #
def family_e4():
    print("\n── E4: objectBoundingBox clipPath keeps font-size as a fraction ──")
    body = ('<defs><clipPath id="c" clipPathUnits="objectBoundingBox">'
            '<text x="0.05" y="0.7" font-family="Arial" font-size="0.2">AB</text>'
            '</clipPath></defs>'
            '<rect x="25" y="15" width="50" height="50" fill="black" clip-path="url(#c)"/>')
    src = tmp("E4-src.svg")
    src.write_text(source_svg(body))
    outs = {}
    for tag, binary in (("before", BEFORE), ("after", BIN)):
        out = tmp("E4-%s.svg" % tag)
        rc, err = compose(binary, src, (200, None), out)
        if not check(rc == 0, "E4 %s: composes" % tag, err[-100:]):
            return
        outs[tag] = out
        text = element(out.read_text(), "text") or {}
        want = "0.400" if tag == "before" else "0.2"
        check(text.get("font-size") == want, "E4 %s: font-size == %s" % (tag, want),
              text.get("font-size"))
    pngs = {}
    for tag, out in outs.items():
        pngs[tag] = tmp("E4-%s.png" % tag)
        check(render(out, pngs[tag], 1), "E4 %s: d1 renders" % tag)
    # R67-B made the whole obb clipPath body verbatim, so the pre-existing obb
    # coordinate scaling that used to empty this clip is gone: the product now
    # has a pixel oracle — the source rendered at the cell scale (2).
    oracle = tmp("E4-oracle.png")
    check(render(src, oracle, 2), "E4: d1 renders the source at the cell scale")
    check(xor(oracle, pngs["after"]) == (0, 0),
          "E4: after == source@2 (obb clip body verbatim, R67-B)",
          pair(xor(oracle, pngs["after"])))
    check(ink(pngs["before"]) == 0 and ink(pngs["after"]) > 0,
          "E4: before empties the clip, after draws it",
          "ink %d -> %d" % (ink(pngs["before"]), ink(pngs["after"])))
    row("E4-obb-clip", "font 0.400 ink %d" % ink(pngs["before"]),
        "font 0.2 ink %d" % ink(pngs["after"]),
        "source@2 xor %s" % pair(xor(oracle, pngs["after"])),
        "d1", cairo(outs["after"], tmp("E4-cairo.png"), 1))


# --------------------------------------------------------------------------- #
# E5: the corrected comment is reproducible
# --------------------------------------------------------------------------- #
def family_e5():
    print("\n── E5: the defs comment no longer claims an unreproducible xor 840 ──")
    source = (ROOT / "tests/selftest/defs/mod.rs").read_text()
    check("xor 840" not in source, "E5: no unreproducible 'xor 840' left")
    check("radius 2 and radius 4 both" in source,
          "E5: the comment says radius 2 and 4 both render 0/0 on this fixture")


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #
def main():
    global ALLOW_SKIP
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-skip", action="store_true",
                        help="a missing binary/renderer is SKIP instead of FAIL")
    args = parser.parse_args()
    ALLOW_SKIP = args.allow_skip

    print("R59 evidence: new %s\n  before %s\n  render %s" % (BIN, BEFORE, RENDER))
    missing = [p for p in (BIN, BEFORE, RENDER) if not p.is_file()]
    if missing:
        for path in missing:
            if ALLOW_SKIP:
                skip("required tool present", "missing %s" % path)
            else:
                check(False, "required tool present", "missing %s" % path)
        return finish()

    family_e1()
    family_e2()
    family_e3()
    family_e4()
    family_e5()

    print("\n%-20s %-14s %-14s %-26s %-4s %s" %
          ("case", "before xor/md", "after xor/md", "oracle", "d1", "cairo"))
    for r in ROWS:
        print("%-20s %-14s %-14s %-26s %-4s %s" % r)
    print("\n%d passed, %d failed, %d skipped" % (len(PASSED), len(FAILED), len(SKIPPED)))
    return finish()


def finish():
    if FAILED:
        print("FAIL: %d check(s) failed" % len(FAILED))
        sys.exit(1)
    print("OK: every R59 check passed")
    sys.exit(0)


if __name__ == "__main__":
    main()
