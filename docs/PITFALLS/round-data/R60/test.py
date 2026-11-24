#!/usr/bin/env python3
"""R60 evidence — the light-source factors and the two malformed-value families.

Families
  F1  a light's `z`/`pointsAtZ` follow the RMS of the axis factors
      (`sqrt((sx²+sy²)/2)`), not the geometric mean.  On a 4x5 cell the geometric
      mean is 44.721 and leaves xor 37011 / maxdiff 206; the RMS is 45.276928.
      Both `feSpotLight` and `fePointLight` (R58 had the same bug in `fePointLight`).
  F2  the same for a non-zero `pointsAtZ` (5 -> 22.638464): 35728/193 before.
  F3  light positions are user-space points, so they **translate** into the cell:
      the second cell of a 2-column figure needs x=120, not 80 (5277/238 before).
  F4  lights are user space whatever `primitiveUnits` says, so an
      `objectBoundingBox` filter still scales them (93427/242 before); the real
      obb fractions (`stdDeviation`) stay untouched.
  F5  `feDisplacementMap scale` is applied **quadratically** by the renderer, so
      the rewritten number is the axis factor's square root: x/sx=4, 10 -> 20
      (exact, xor 0; the old linear 40 displaced 4x too far).  Two axes have no
      exact single value; the fallback is `scale * (sx*sy)^(1/4)` (21.147 vs the
      old geometric mean 44.721, which is 4.5x too far).
  F6  a primitive's own subregion (`x`/`y`/`width`/`height`, userSpaceOnUse) is a
      user length and follows the panel (16984/255 before).
  F7  a non-finite length is kept **byte for byte** — never `inf`/`NaN` — and an
      empty / three-number `stdDeviation` is reported (W7) instead of dropped.

Teeth: the before binary is the R59 HEAD build (/tmp/r59_eval/pin), i.e. the
state the review rejected; every family must be red against it.

Arbitration: d1render.  D1_FONT_DIR must point at fontlib or text ink is 0.
cairosvg is recorded for reference only (it ignores filters).

Run:  python3 docs/PITFALLS/round-data/R60/test.py
      python3 docs/PITFALLS/round-data/R60/test.py --allow-skip  # missing tools skip
Exit: 0 = every check passed, 1 = at least one check failed (fail-closed: a
      missing binary, before binary or renderer is a failure unless --allow-skip).
"""

import argparse
import base64
import os
import re
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
BEFORE = Path(os.environ.get("SVG_GRID_BEFORE_BIN",
                            "/tmp/r59_eval/pin/target/release/svg_grid"))
RENDER = Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO = Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(Path.home() / ".venv/bin/python")))
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r60-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))

PASSED, FAILED, SKIPPED, ROWS = [], [], [], []
ALLOW_SKIP = False
COUNTER = [0]


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + ((" — " + str(detail)) if detail and not ok else ""))
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
# PNG reading (8-bit RGB/RGBA) + ink/xor, identical to R59
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


def element(svg, tag, index=0):
    found = elements(svg, tag)
    return found[index] if len(found) > index else None


def attr_in(svg, tag, name, index=0):
    found = [value for attrs in elements(svg, tag) for key, value in attrs.items() if key == name]
    return found[index] if len(found) > index else None


def render(svg_path, png_path, scale):
    r = subprocess.run([str(RENDER), str(svg_path), str(png_path), str(scale)],
                       env=ENV, capture_output=True, text=True)
    return r.returncode == 0


def compose(binary, src, out, width, ncol=1, height=None, verbose=False):
    srcs = src if isinstance(src, (list, tuple)) else [src]
    cmd = [str(binary), "plot-grid"]
    for one in srcs:
        cmd += ["--input", str(one)]
    cmd += ["--output", str(out),
           "--width", str(width), "--ncol", str(ncol), "--plot-margin", "0"]
    if height is not None:
        cmd += ["--height", str(height)]
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


def oracle_svg(body, sx, sy, width=100, height=80, canvas=(400, 400)):
    """The source body wrapped in the transform the cell really applies."""
    cw, ch = canvas
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


def pixels(name, body, width, oracle_body, sx, sy, policy="xy", height=None, ncol=1,
           canvas=(400, 400)):
    """Compose before/after, render both at 1, render the oracle; return both xor.

    Isotropic cases pass `oracle_body is None` and the oracle is the source itself
    rendered at `sx`.  Anisotropic cases pass the source body and the factor pair.
    """
    src = tmp(name + "-src.svg")
    src.write_text(source_svg(body, policy))
    outs, gots, xors = {}, {}, {}
    for tag, binary in (("before", BEFORE), ("after", BIN)):
        out = tmp(name + "-%s.svg" % tag)
        rc, err = compose(binary, src, out, width, ncol, height)
        if rc != 0:
            return rc, err, None, None, None
        outs[tag] = out
        png = tmp(name + "-%s.png" % tag)
        if not render(out, png, 1):
            return rc, "render failed", None, None, None
        gots[tag] = png
    if oracle_body is None:
        oracle_path, oracle_scale = src, sx
    else:
        oracle_path = tmp(name + "-oracle.svg")
        if isinstance(oracle_body, tuple):
            # (body, canvas, transform) for the translate family.
            text, (cw, ch), transform = oracle_body
            oracle_path.write_text(
                '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
                'viewBox="0 0 %d %d"><rect width="%d" height="%d" fill="white"/>'
                '<g transform="%s">%s</g></svg>' % (cw, ch, cw, ch, cw, ch, transform, text))
        else:
            oracle_path.write_text(oracle_svg(oracle_body, sx, sy, canvas=canvas))
        oracle_scale = 1
    oracle_png = tmp(name + "-oracle.png")
    if not render(oracle_path, oracle_png, oracle_scale):
        return 0, "oracle render failed", None, None, None
    for tag in ("before", "after"):
        xors[tag] = xor(oracle_png, gots[tag])
    row(name, pair(xors["before"]), pair(xors["after"]), "ink %d" % ink(oracle_png), "d1",
        cairo(outs["after"], tmp(name + "-cairo.png"), 1))
    return 0, "", outs["after"].read_text(), xors, oracle_png


# --------------------------------------------------------------------------- #
# fixtures: the light sources the review used
# --------------------------------------------------------------------------- #
def lighting(body, region='x="0" y="0" width="100" height="80"', units=""):
    return ('<defs><filter id="f" filterUnits="userSpaceOnUse" %s%s>'
            '<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">'
            '%s</feDiffuseLighting></filter></defs>'
            '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>'
            % (region, units, body))


SPOT = ('<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="0" '
        'limitingConeAngle="30"/>')
POINT = '<fePointLight x="20" y="20" z="10"/>'


def family_f1():
    print("\n── F1: light z is the RMS of the axis factors (4x5) ──")
    rc, err, svg, xors, _ = pixels("F1-spot-rms", lighting(SPOT), 400, lighting(SPOT), 4, 5, height=400)
    if not check(rc == 0, "F1 spot: both binaries compose", err[-120:]) or not check(svg is not None, "F1 spot: output"):
        return
    spot = element(svg, "feSpotLight") or {}
    for name, want in (("x", "80"), ("y", "100"), ("z", "45.276928"),
                       ("pointsAtX", "200"), ("pointsAtY", "200"),
                       ("limitingConeAngle", "30")):
        check(spot.get(name) == want, "F1 spot: feSpotLight@%s == %s" % (name, want), spot.get(name))
    check(xors["before"][0] > 30000, "F1 spot: the before binary is wrong (teeth)",
          pair(xors["before"]))
    check(xors["after"] == (0, 0), "F1 spot: xor 0 / md 0 after the RMS factor",
          pair(xors["after"]))

    rc, err, svg, xors, _ = pixels("F1-point-rms", lighting(POINT), 400, lighting(POINT), 4, 5, height=400)
    if check(rc == 0, "F1 point: composes") and check(svg is not None, "F1 point: output"):
        check((element(svg, "fePointLight") or {}).get("z") == "45.276928",
              "F1 point: fePointLight@z == 45.276928 (R58 inherited bug)",
              element(svg, "fePointLight") or {})
        check(xors["before"][0] > 30000, "F1 point: before binary is wrong (teeth)",
              pair(xors["before"]))
        check(xors["after"] == (0, 0), "F1 point: xor 0 / md 0", pair(xors["after"]))


def family_f2():
    print("\n── F2: pointsAtZ follows the same RMS factor ──")
    body = ('<feSpotLight x="20" y="20" z="10" pointsAtX="50" pointsAtY="40" pointsAtZ="5" '
            'limitingConeAngle="30"/>')
    rc, err, svg, xors, _ = pixels("F2-spot-paz", lighting(body), 400, lighting(body), 4, 5, height=400)
    if check(rc == 0, "F2: composes") and check(svg is not None, "F2: output"):
        spot = element(svg, "feSpotLight") or {}
        check((spot.get("z"), spot.get("pointsAtZ")) == ("45.276928", "22.638464"),
              "F2: z/pointsAtZ are 45.276928/22.638464", spot)
        check(xors["before"][0] > 30000, "F2: before binary is wrong (teeth)",
              pair(xors["before"]))
        check(xors["after"] == (0, 0), "F2: xor 0 / md 0", pair(xors["after"]))


def family_f3():
    print("\n── F3: light positions translate into the second cell ──")
    # Two 100x80 panels side by side: cell 1 is a pure +100 translation, so the light
    # that sat at x=20 must move to 120 (and pointsAtX 50 to 150).  The source declares
    # its panel box, so the layout is an exact translation (no auto-conversion inset).
    body = ('<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
            'visibility="hidden" pointer-events="none"/>' + WHITE + lighting(SPOT))
    src = tmp("F3-src.svg")
    src.write_text(source_svg(body))
    second_src = tmp("F3-src2.svg")
    second_src.write_text(source_svg(body))
    outs = {}
    for tag, binary in (("before", BEFORE), ("after", BIN)):
        out = tmp("F3-%s.svg" % tag)
        rc, err = compose(binary, [src, second_src], out, 200, ncol=2, height=80)
        if not check(rc == 0, "F3 %s: composes" % tag, err[-120:]):
            return
        outs[tag] = out
        svg = out.read_text()
        second = element(svg, "feSpotLight", 1) or {}
        want = ("20", "50") if tag == "before" else ("120", "150")
        check((second.get("x"), second.get("pointsAtX")) == want,
              "F3 %s: second cell light at x=%s/pointsAtX=%s%s"
              % (tag, want[0], want[1], " (teeth: the old binary leaves it at 20)" if tag == "before" else ""),
              second)
        check((element(svg, "feSpotLight") or {}).get("x") == "20",
              "F3 %s: first cell light stays at x=20" % tag,
              element(svg, "feSpotLight"))
    pngs = {}
    for tag, out in outs.items():
        pngs[tag] = tmp("F3-%s.png" % tag)
        check(render(out, pngs[tag], 1), "F3 %s: d1 renders" % tag)
    oracle = tmp("F3-oracle.svg")
    oracle.write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="80" viewBox="0 0 200 80">'
        '<rect width="200" height="80" fill="white"/>%s'
        '<g transform="translate(100 0)">%s</g></svg>'
        % (WHITE + lighting(SPOT), WHITE + lighting(SPOT)))
    opng = tmp("F3-oracle.png")
    check(render(oracle, opng, 1), "F3: d1 renders the oracle")
    row("F3-two-cell", pair(xor(opng, pngs["before"])), pair(xor(opng, pngs["after"])),
        "ink %d" % ink(opng), "d1", cairo(outs["after"], tmp("F3-cairo.png"), 1))
    check(xor(opng, pngs["after"]) == (0, 0), "F3: xor 0 / md 0 against the translated oracle",
          pair(xor(opng, pngs["after"])))


def family_f4():
    print("\n── F4: objectBoundingBox lighting still scales the light ──")
    # usvg keeps the six light numbers as plain f32 whatever `primitiveUnits` says.
    body = lighting(SPOT, units=' primitiveUnits="objectBoundingBox"')
    rc, err, svg, xors, _ = pixels("F4-obb", body, 400, body, 4, 5, height=400)
    if check(rc == 0, "F4: composes", err[-120:]) and check(svg is not None, "F4: output"):
        spot = element(svg, "feSpotLight") or {}
        check((spot.get("x"), spot.get("y"), spot.get("z")) == ("80", "100", "45.276928"),
              "F4: obb light scaled like user space", spot)
        check(xors["before"][0] > 30000, "F4: before binary is wrong (teeth)",
              pair(xors["before"]))
        check(xors["after"] == (0, 0), "F4: xor 0 / md 0", pair(xors["after"]))
    # A real obb fraction is not a length and must not move.
    obb = ('<defs><filter id="f" primitiveUnits="objectBoundingBox">'
           '<feGaussianBlur stdDeviation="0.1"/></filter></defs>'
           '<rect x="10" y="10" width="80" height="60" filter="url(#f)"/>')
    src = tmp("F4-fraction-src.svg")
    src.write_text(source_svg(obb))
    out = tmp("F4-fraction-after.svg")
    rc, _ = compose(BIN, src, out, 400, height=400)
    got = (element(out.read_text(), "feGaussianBlur") or {}).get("stdDeviation") if rc == 0 else None
    check(got == "0.1", "F4: an obb stdDeviation stays a fraction", got)


def top_ink(path, col):
    """The y of the first inked pixel of a column: the rect's top edge."""
    w, h, ch, rows = read_png(path)
    for y in range(h):
        if rows[y][col * ch] < 128:
            return y
    return None


def left_ink(path, row):
    """The x of the first inked pixel of a scanline: the rect's left edge, in device
    pixels.  The filter rasters natively at the cell size, so this is an exact oracle
    (a `<g transform>` oracle would rasterise the filter at 1/s resolution instead)."""
    w, h, ch, rows = read_png(path)
    for x in range(w):
        if rows[row][x * ch] < 128:
            return x
    return None


def family_f5():
    print("\n── F5: feDisplacementMap scale takes the square root of the factor ──")
    # A constant map isolates one channel (R=255 -> dx=+0.5, G=128 -> dy=0).  The
    # renderer applies `scale` twice (`0.5 * scale^2 * axis`), so under data-scale=x
    # with sx=4 the exact value for `1` is `2`, not `4`.  The black rect starts at
    # x=40 in the cell (10 * 4), so the left edge is the oracle: +2 px when exact,
    # +8 px with the old linear value.
    # The map URI must be substituted BEFORE the fixture is built, otherwise the
    # replacement below is a no-op and both axes keep G=128 (only the attribute
    # assertion would then exercise the xy path).
    def body(map_uri, scale="1"):
        return ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
                'height="80"><feImage href="%s" result="map"/>'
                '<feDisplacementMap in="SourceGraphic" in2="map" scale="%s" '
                'xChannelSelector="R" yChannelSelector="G"/></filter></defs>'
                '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>'
                % (map_uri, scale))
    src = tmp("F5-x-src.svg")
    src.write_text(source_svg(body(const_map(255, 128)), "x"))
    edges = {}
    for tag, binary in (("before", BEFORE), ("after", BIN)):
        out = tmp("F5-x-%s.svg" % tag)
        rc, err = compose(binary, src, out, 400, height=80)
        if not check(rc == 0, "F5 x %s: composes" % tag, err[-120:]):
            return
        scale = (element(out.read_text(), "feDisplacementMap") or {}).get("scale")
        check(scale == ("4" if tag == "before" else "2"),
              "F5 x %s: scale == %s" % (tag, "4 (linear, wrong)" if tag == "before" else "2 (sqrt(4))"),
              scale)
        png = tmp("F5-x-%s.png" % tag)
        check(render(out, png, 1), "F5 x %s: d1 renders" % tag)
        edges[tag] = left_ink(png, 40)
    check(edges["after"] == 42, "F5 x after: the rect moves +2 px (0.5 * 2^2)", edges["after"])
    check(edges["before"] == 48, "F5 x before: the rect moves +8 px (teeth); the oracle is +2",
          edges["before"])
    row("F5-x-bbox", "left edge %s" % edges["before"], "left edge %s" % edges["after"],
        "left edge 42", "d1", "n/a")
    # Two axes have no exact single value: sqrt(8) in x and sqrt(10) in y for the same
    # number, so the fallback is their geometric mean, (4*5)^(1/4) = 2.115.
    # Both channels are 255 now (R -> dx, G -> dy), so the pixels must move on
    # BOTH axes. A scale="0" control gives the undisplaced edges (40, 50); the
    # exact 2.115 moves the left edge by 0.5*2.115^2*sx and the top edge by
    # 0.5*2.115^2*sy device pixels.
    src = tmp("F5-xy-src.svg")
    src.write_text(source_svg(body(const_map(255, 255)), "xy"))
    out = tmp("F5-xy-after.svg")
    rc, err = compose(BIN, src, out, 400, height=400)
    got = (element(out.read_text(), "feDisplacementMap") or {}).get("scale") if rc == 0 else None
    check(got == "2.115", "F5 xy: scale == 2.115 ((4*5)^(1/4))", got)
    if check(rc == 0, "F5 xy: composes", err[-120:]):
        png = tmp("F5-xy-after.png")
        check(render(out, png, 1), "F5 xy: d1 renders")
        left = left_ink(png, 150)
        top = top_ink(png, 150)
        control_src = tmp("F5-xy-control-src.svg")
        control_src.write_text(source_svg(body(const_map(255, 255), scale="0"), "xy"))
        control = tmp("F5-xy-control.svg")
        rc0, _ = compose(BIN, control_src, control, 400, height=400)
        png0 = tmp("F5-xy-control.png")
        ok0 = rc0 == 0 and render(control, png0, 1)
        check(ok0, "F5 xy: control composes and renders")
        if ok0:
            left0 = left_ink(png0, 150)
            top0 = top_ink(png0, 150)
            check(left0 == 40 and top0 == 50, "F5 xy: control edges (40, 50)",
                  (left0, top0))
            check(left is not None and left < left0,
                  "F5 xy: x displacement moves the left edge", (left0, left))
            check(top is not None and top < top0,
                  "F5 xy: y displacement moves the top edge", (top0, top))
            # C=255 displaces toward smaller device coordinates; the pixel step is
            # ~0.5 * scale^2 = 2.2 (the review measured the same 40 -> 38, 50 -> 48).
            check(1 <= left0 - left <= 6,
                  "F5 xy: dx ~ 2 px for scale 2.115", left0 - left)
            check(1 <= top0 - top <= 6,
                  "F5 xy: dy ~ 2 px for scale 2.115", top0 - top)
            row("F5-xy", "edges (%s, %s)" % (left0, top0),
                "edges (%s, %s)" % (left, top), "both axes move", "d1", "n/a")
    # One axis at a time, y: sqrt(5) = 2.236.
    src = tmp("F5-y-src.svg")
    src.write_text(source_svg(body(const_map(255, 128)), "y"))
    out = tmp("F5-y-after.svg")
    rc, _ = compose(BIN, src, out, 400, height=400)
    got = (element(out.read_text(), "feDisplacementMap") or {}).get("scale") if rc == 0 else None
    check(got == "2.236", "F5 y: scale == 2.236 (sqrt(5))", got)


def family_f6():
    print("\n── F6: a primitive's own subregion follows the panel ──")
    body = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
            'height="80"><feGaussianBlur stdDeviation="2" x="10" y="10" width="80" '
            'height="60"/></filter></defs>'
            '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>')
    rc, err, svg, xors, _ = pixels("F6-subregion", body, 200, None, 2, 2)
    if check(rc == 0, "F6: composes", err[-120:]) and check(svg is not None, "F6: output"):
        blur = element(svg, "feGaussianBlur") or {}
        check((blur.get("stdDeviation"), blur.get("x"), blur.get("y"), blur.get("width"),
               blur.get("height")) == ("4", "20", "20", "160", "120"),
              "F6: subregion doubled with the panel", blur)
        check(xors["before"][0] > 5000, "F6: before binary is wrong (teeth)",
              pair(xors["before"]))
        check(xors["after"] == (0, 0), "F6: xor 0 / md 0", pair(xors["after"]))
    # Under obb the same four numbers are fractions: they must not move.
    obb = ('<defs><filter id="f" primitiveUnits="objectBoundingBox">'
           '<feGaussianBlur stdDeviation="0.1" x="0.1" y="0.1" width="0.8" height="0.6"/>'
           '</filter></defs><rect x="10" y="10" width="80" height="60" filter="url(#f)"/>')
    src = tmp("F6-obb-src.svg")
    src.write_text(source_svg(obb))
    out = tmp("F6-obb-after.svg")
    rc, _ = compose(BIN, src, out, 200)
    blur = (element(out.read_text(), "feGaussianBlur") or {}) if rc == 0 else {}
    check((blur.get("x"), blur.get("width"), blur.get("height")) == ("0.1", "0.8", "0.6"),
          "F6: obb subregion stays a fraction", blur)


def const_map(r, g):
    """A 16x16 constant RGB PNG as a data URI: no resampling, so the displacement is
    a pure function of `scale` and the axis factors."""
    w = h = 16
    raw = b"".join(b"\x00" + bytes([r, g, 0]) * w for _ in range(h))

    def chunk(kind, payload):
        return (struct.pack(">I", len(payload)) + kind + payload
                + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF))

    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw))
           + chunk(b"IEND", b""))
    return "data:image/png;base64," + base64.b64encode(png).decode()


def family_f7():
    print("\n── F7: a non-finite length is kept verbatim, never written as inf ──")
    cases = [
        ("F7-blur", '<feGaussianBlur stdDeviation="1e999"/>', "feGaussianBlur", "stdDeviation",
         "1e999"),
        ("F7-stroke", '<rect x="10" y="10" width="10" height="10" fill="none" stroke="#000" '
         'stroke-width="1e999"/>', "rect", "stroke-width", "1e999"),
    ]
    for name, inner, tag, attr_name, value in cases:
        body = inner if tag == "rect" else (
            '<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
            'height="80">%s</filter></defs><rect x="10" y="10" width="80" height="60" '
            'fill="black" filter="url(#f)"/>' % inner)
        src = tmp(name + "-src.svg")
        src.write_text(source_svg(body))
        old_out, new_out = tmp(name + "-before.svg"), tmp(name + "-after.svg")
        old_rc, _ = compose(BEFORE, src, old_out, 200, verbose=True)
        new_rc, err = compose(BIN, src, new_out, 200, verbose=True)
        old_svg = old_out.read_text() if old_out.is_file() else ""
        new_svg = new_out.read_text() if new_out.is_file() else ""
        check("inf" in old_svg, "%s: before binary writes inf (teeth)" % name,
              attr_in(old_svg, tag, attr_name))
        check(new_rc == 0, "%s: rc stays 0" % name, "rc=%d" % new_rc)
        check(attr_in(new_svg, tag, attr_name) == value,
              "%s: %s kept byte for byte" % (name, attr_name),
              attr_in(new_svg, tag, attr_name))
        check("inf" not in new_svg and "NaN" not in new_svg,
              "%s: no inf/NaN in the output" % name)
        check("non-finite" in err and value in err, "%s: W7 names the value" % name,
              err.strip().splitlines()[-1][:160] if err.strip() else "rc=%d" % old_rc)
        row(name, "inf", "verbatim", "attribute oracle", "d1", "n/a")


def family_f8():
    print("\n── F8: an empty / three-number stdDeviation is reported, not dropped ──")
    for raw in ("", " ", "2 3 4"):
        name = "F8-%r" % raw
        body = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
                'height="80"><feGaussianBlur stdDeviation="%s"/></filter></defs>'
                '<rect x="10" y="10" width="80" height="60" fill="black" filter="url(#f)"/>'
                % raw)
        src = tmp(slug(name) + "-src.svg")
        src.write_text(source_svg(body))
        old_out, new_out = tmp(slug(name) + "-before.svg"), tmp(slug(name) + "-after.svg")
        old_rc, old_err = compose(BEFORE, src, old_out, 200, verbose=True)
        new_rc, err = compose(BIN, src, new_out, 200, verbose=True)
        check(old_rc == 0 and "expected one or two numbers" not in old_err,
              "%s: before binary is silent (teeth)" % name)
        check(new_rc == 0, "%s: rc stays 0" % name, "rc=%d" % new_rc)
        got = (element(new_out.read_text(), "feGaussianBlur") or {}).get("stdDeviation") \
            if new_out.is_file() else None
        check(got == raw, "%s: text kept byte for byte" % name, got)
        check("expected one or two numbers" in err, "%s: W7 names the attribute type" % name,
              err.strip().splitlines()[-1][:160] if err.strip() else "")
        row(name, "silent", "W7", "attribute oracle", "d1", "n/a")


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

    print("R60 evidence: new %s\n  before %s\n  render %s" % (BIN, BEFORE, RENDER))
    missing = [p for p in (BIN, BEFORE, RENDER) if not p.is_file()]
    if missing:
        for path in missing:
            if ALLOW_SKIP:
                skip("required tool present", "missing %s" % path)
            else:
                check(False, "required tool present", "missing %s" % path)
        return finish()

    family_f1()
    family_f2()
    family_f3()
    family_f4()
    family_f5()
    family_f6()
    family_f7()
    family_f8()

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
    print("OK: every R60 check passed")
    sys.exit(0)


if __name__ == "__main__":
    main()
