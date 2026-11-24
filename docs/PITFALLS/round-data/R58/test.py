#!/usr/bin/env python3
"""R58 evidence — filter primitive lengths (`primitiveUnits`), definition-body
`font-size`, and the filter/mask regions that used to be invented as 0x0.

Families
  A  filter primitives under the default `primitiveUnits="userSpaceOnUse"`:
     lengths follow the panel scale (blur, offset, morphology, drop shadow,
     turbulence, lighting, image, displacement); convolve/colour/compositing
     carry no user length and must stay verbatim.
  N  the same under anisotropy (sx=4, sy=5, 100x80 -> 400x400): every
     attribute that accepts one or two numbers must be written per axis.
  B  `font-size` of <text>/<tspan>/style inside pattern, mask and clipPath
     bodies; ordinary panel text and viewBox pattern bodies must not move.
  C  a filter/mask without a region, or one that inherits its region through
     href, must not be given an invented 0x0 box; explicit regions still scale.

Arbitration: d1render.  D1_FONT_DIR must point at fontlib, otherwise text ink
is always 0.  cairosvg is recorded for reference only: it ignores most filters
and does not paint text inside pattern/mask/clipPath.

Run:  python3 docs/PITFALLS/round-data/R58/test.py
      python3 docs/PITFALLS/round-data/R58/test.py --allow-skip   # skips are green
Exit: 0 = all checks pass, 1 = at least one check failed.
Env:  SVG_GRID_BIN, SVG_GRID_OLD_BIN, SVG_GRID_RENDER_BIN, SVG_GRID_CAIRO_PYTHON

The old binary (default /tmp/r50base, @e0c6033) is used only to show the
suite has teeth: when it is absent those columns read n/a and the new-side
checks still run.
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
OLD = Path(os.environ.get("SVG_GRID_OLD_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO = Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(Path.home() / ".venv/bin/python")))
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r58-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))

PASSED, FAILED, SKIPPED, ROWS = [], [], [], []
ALLOW_SKIP = False
COUNTER = [0]


# --------------------------------------------------------------------------- #
# small helpers
# --------------------------------------------------------------------------- #
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
# PNG (8-bit RGB/RGBA, all five filter types) — same reader as R55
# --------------------------------------------------------------------------- #
def read_png(path):
    data = Path(path).read_bytes()
    pos = 8
    rows, w, h, ch = [], 0, 0, 0
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
                b = prev[x]
                c = prev[x - ch] if x >= ch else 0
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
        return None, None
    n = md = 0
    for y in range(ha):
        for x in range(wa):
            pa = ra[y][x * ca:(x + 1) * ca][:3]
            pb = rb[y][x * cb:(x + 1) * cb][:3]
            d = max(abs(pa[i] - pb[i]) for i in range(3))
            if d:
                n += 1
                md = max(md, d)
    return n, md


# --------------------------------------------------------------------------- #
# SVG / renderer helpers
# --------------------------------------------------------------------------- #
TAG_RE = re.compile(r'<([A-Za-z][\w:-]*)((?:\s+[\w:-]+\s*=\s*"[^"]*")*)\s*/?>')
ATTR_RE = re.compile(r'([\w:-]+)\s*=\s*"([^"]*)"')


def elements(svg, tag):
    return [dict(ATTR_RE.findall(m.group(2))) for m in TAG_RE.finditer(svg) if m.group(1) == tag]


def element(svg, tag, ident=None):
    """Find a tag by id; the composer prefixes ids (svg-grid-cell0-base), so a
    suffix match is accepted as well."""
    for attrs in elements(svg, tag):
        if ident is None:
            return attrs
        got = attrs.get("id", "")
        if got == ident or got.endswith("-" + ident):
            return attrs
    return None


def render(svg_path, png_path, scale):
    r = subprocess.run([str(RENDER), str(svg_path), str(png_path), str(scale)],
                       env=ENV, capture_output=True, text=True)
    return r.returncode == 0


def compose(binary, src_svg, cell, out_svg):
    cmd = [str(binary), "plot-grid", "--input", str(src_svg), "--output", str(out_svg),
           "--width", str(cell[0])]
    if cell[1] is not None:
        cmd += ["--height", str(cell[1])]
    cmd += ["--ncol", "1", "--plot-margin", "0"]
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode == 0, (r.stderr.strip().splitlines() or [""])[-1][:100]


def cairo(src_svg, png, scale):
    code = ("import cairosvg,sys;"
            "cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2], scale=float(sys.argv[3]))")
    r = subprocess.run([str(CAIRO), "-c", code, str(src_svg), str(png), str(scale)],
                       capture_output=True, text=True)
    if r.returncode == 0:
        return None
    return (r.stderr.strip().splitlines() or ["cairo failed"])[-1][:90]


WHITE = '<rect width="100" height="80" fill="white"/>'


def source_svg(kind, body):
    if kind == "p":
        body = '<g data-scale="position">' + body + "</g>"
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
            'viewBox="0 0 100 80">%s%s</svg>' % (WHITE, body))


def aniso_oracle_svg(body):
    inner = WHITE + body
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" '
            'viewBox="0 0 400 400"><rect width="400" height="400" fill="white"/>'
            '<g transform="scale(4 5)">%s</g></svg>' % inner)


def cfg(kind):
    if kind == "a":
        return (400, 400), 1, "aniso"
    if kind == "u1":
        return (100, None), 1, "src"
    return (200, None), 2, "src"


# --------------------------------------------------------------------------- #
# attribute assertions
# --------------------------------------------------------------------------- #
def expect(prefix, svg, tag, ident, want):
    got = element(svg, tag, ident)
    ok = check(got is not None, "%s: <%s%s> present" % (prefix, tag, ' id="%s"' % ident if ident else ""))
    if not ok:
        return
    for name, value in want.items():
        if value is None:
            ok = name not in got
            shown = got.get(name)
        elif isinstance(value, float):
            try:
                ok = abs(float(got.get(name, "nan")) - value) <= 0.02
            except ValueError:
                ok = False
            shown = got.get(name)
        elif isinstance(value, tuple):
            ok = value[1] in got.get(name, "")
            shown = got.get(name)
        else:
            ok = got.get(name) == value
            shown = got.get(name)
        check(ok, "%s: <%s>.%s" % (prefix, tag, name),
              "want %r, got %r" % (value, shown))


def no_invented_box(prefix, svg):
    for tag in ("filter", "mask"):
        for attrs in elements(svg, tag):
            bad = [n for n in ("width", "height") if attrs.get(n) == "0"]
            check(not bad, "%s: <%s> did not invent a zero box" % (prefix, tag),
                  "%s → %r" % (bad, attrs))


# --------------------------------------------------------------------------- #
# cases
# --------------------------------------------------------------------------- #
TINY = ("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4"
        "2mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==")


def f(body):
    return ('<defs><filter id="f">%s</filter></defs>'
            '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>' % body)


CASES = [
    # ---- A: primitiveUnits="userSpaceOnUse" (default), s=2 -----------------
    dict(name="A1 blur", kind="u", teeth=2000,
         body=f('<feGaussianBlur stdDeviation="2"/>'),
         attrs=[("feGaussianBlur", None, {"stdDeviation": "4"})], pixel="exact"),
    dict(name="A2 offset", kind="u", teeth=200,
         body=f('<feOffset dx="3" dy="5"/>'),
         attrs=[("feOffset", None, {"dx": "6", "dy": "10"})], pixel="exact"),
    dict(name="A3 morphology", kind="u", teeth=200,
         body=f('<feMorphology operator="dilate" radius="2"/>'),
         attrs=[("feMorphology", None, {"radius": "4"})], pixel="exact"),
    dict(name="A4 drop shadow", kind="u", teeth=200,
         body=f('<feDropShadow dx="3" dy="5" stdDeviation="2"/>'),
         attrs=[("feDropShadow", None, {"dx": "6", "dy": "10", "stdDeviation": "4"})],
         pixel="exact"),
    dict(name="A5 turbulence divides", kind="u", teeth=2000,
         body=f('<feTurbulence baseFrequency="0.05" numOctaves="1" seed="3"/>'),
         attrs=[("feTurbulence", None, {"baseFrequency": "0.025"})], pixel="exact"),
    dict(name="A6 diffuse + point light", kind="u", teeth=2000,
         body=f('<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">'
                '<fePointLight x="30" y="20" z="10"/></feDiffuseLighting>'),
         attrs=[("feDiffuseLighting", None, {"surfaceScale": "2", "diffuseConstant": "1",
                                             "lighting-color": "white"}),
                ("fePointLight", None, {"x": "60", "y": "40", "z": "20"})], pixel="exact"),
    dict(name="A7 specular + point light", kind="u", teeth=0,
         body=f('<feSpecularLighting surfaceScale="2" specularConstant="1" specularExponent="20" '
                'lighting-color="white"><fePointLight x="30" y="20" z="10"/></feSpecularLighting>'),
         attrs=[("feSpecularLighting", None, {"specularExponent": "20"}),
                ("fePointLight", None, {"x": "60", "y": "40", "z": "20"})], pixel="exact"),
    dict(name="A8 feImage", kind="u", teeth=200,
         body=f('<feImage x="10" y="10" width="20" height="20" href="%s"/>' % TINY),
         attrs=[("feImage", None, {"x": "20", "y": "20", "width": "40", "height": "40"})],
         pixel="exact"),
    dict(name="A9 displacement (residual)", kind="u", teeth=200,
         body=f('<feTurbulence type="turbulence" baseFrequency="0.05" numOctaves="1" seed="3" '
                'result="t"/><feDisplacementMap in="SourceGraphic" in2="t" scale="10" '
                'xChannelSelector="R" yChannelSelector="G"/>'),
         # round-data/R60: the renderer applies `scale` quadratically
         # (offset = (c-0.5) * scale^2 * axis), so a uniform 2x cell takes
         # 10 * sqrt(2) = 14.142, not the linear 20 (measured: xor 0 either way
         # here, but the displacement is 2x too far with 20 on a solid map).
         attrs=[("feDisplacementMap", None, {"scale": "14.142"}),
                ("feTurbulence", None, {"baseFrequency": "0.025"})],
         pixel=("within", 200, 255), note="residual: the composed doc samples the noise at 1x"),
    dict(name="A10 convolve stays", kind="u", teeth=0,
         body=f('<feConvolveMatrix order="3" kernelMatrix="0 -1 0 -1 5 -1 0 -1 0" divisor="1" '
                'edgeMode="duplicate"/>'),
         attrs=[("feConvolveMatrix", None, {"order": "3", "divisor": "1", "edgeMode": "duplicate",
                                            "kernelMatrix": "0 -1 0 -1 5 -1 0 -1 0"})], pixel="exact"),
    dict(name="A11 colour/composite/flood/tile", kind="u", teeth=0,
         body=f('<feFlood flood-color="#336699"/><feComposite in2="SourceGraphic" operator="over"/>'
                '<feColorMatrix type="saturate" values="1"/><feTile/>'
                '<feBlend in2="SourceGraphic" mode="multiply"/>'),
         attrs=[("feFlood", None, {"flood-color": "#336699"}),
                ("feComposite", None, {"operator": "over"}),
                ("feBlend", None, {"mode": "multiply"}),
                ("feColorMatrix", None, {"values": "1"})], pixel="exact"),
    dict(name="A12 blur two-value", kind="u", teeth=2000,
         body=f('<feGaussianBlur stdDeviation="2 3"/>'),
         attrs=[("feGaussianBlur", None, {"stdDeviation": "4 6"})], pixel="exact"),
    dict(name="A13 primitiveUnits obb", kind="u", teeth=0,
         body='<defs><filter id="f" primitiveUnits="objectBoundingBox">'
              '<feGaussianBlur stdDeviation="0.05"/></filter></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("feGaussianBlur", None, {"stdDeviation": "0.05"})], pixel="exact"),
    # ---- N: anisotropy sx=4 sy=5 -------------------------------------------
    dict(name="N1 blur single", kind="a", teeth=20000,
         body=f('<feGaussianBlur stdDeviation="2"/>'),
         attrs=[("feGaussianBlur", None, {"stdDeviation": "8 10"})], pixel="exact"),
    dict(name="N2 blur two", kind="a", teeth=20000,
         body=f('<feGaussianBlur stdDeviation="2 3"/>'),
         attrs=[("feGaussianBlur", None, {"stdDeviation": "8 15"})], pixel="exact"),
    dict(name="N3 morphology", kind="a", teeth=2000,
         body=f('<feMorphology operator="dilate" radius="2"/>'),
         attrs=[("feMorphology", None, {"radius": "8 10"})], pixel="exact"),
    dict(name="N4 drop shadow", kind="a", teeth=2000,
         body=f('<feDropShadow dx="3" dy="5" stdDeviation="2"/>'),
         attrs=[("feDropShadow", None, {"dx": "12", "dy": "25", "stdDeviation": "8 10"})],
         pixel="exact"),
    dict(name="N5 turbulence two values", kind="a", teeth=2000,
         body=f('<feTurbulence baseFrequency="0.05" numOctaves="1" seed="3"/>'),
         attrs=[("feTurbulence", None, {"baseFrequency": "0.0125 0.01"})],
         pixel=("within", 20, 4), note="noise is not bit-reproducible; 2 px is the floor"),
    dict(name="N6 feImage (residual)", kind="a", teeth=2000,
         body=f('<feImage x="10" y="10" width="20" height="20" href="%s"/>' % TINY),
         attrs=[("feImage", None, {"x": "40", "y": "50", "width": "80", "height": "100"})],
         pixel=("within", 800, 255),
         note="residual: preserveAspectRatio fits in pre-transform user space"),
    dict(name="N7 offset", kind="a", teeth=2000,
         body=f('<feOffset dx="3" dy="5"/>'),
         attrs=[("feOffset", None, {"dx": "12", "dy": "25"})], pixel="exact"),
    dict(name="N8 diffuse + light (residual)", kind="a", teeth=20000,
         body=f('<feDiffuseLighting surfaceScale="2" diffuseConstant="1" lighting-color="white">'
                '<fePointLight x="30" y="20" z="10"/></feDiffuseLighting>'),
         # round-data/R60: z is the RMS of the axis factors, not the geometric
         # mean (44.72 was the old, wrong factor; 45.276928 is xor 0).
         attrs=[("fePointLight", None, {"x": "120", "y": "100", "z": 45.276928})],
         pixel=("within", 60000, 255),
         note="residual: the shading ramp still moves a little under 4x5"),
    # ---- B: definition-body font-size --------------------------------------
    dict(name="B1 pattern text", kind="u", teeth=200,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20">'
              '<text x="2" y="14" font-size="12" font-family="Arial">A</text></pattern></defs>'
              '<rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("text", None, {"font-size": "24"})], pixel="exact"),
    dict(name="B2 mask text", kind="u", teeth=50,
         body='<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
              '<text x="2" y="14" font-size="12" font-family="Arial" fill="white">A</text></mask></defs>'
              '<rect width="100" height="80" fill="black" mask="url(#m)"/>',
         attrs=[("text", None, {"font-size": "24"})], pixel="exact"),
    dict(name="B3 clipPath text", kind="u", teeth=50,
         body='<defs><clipPath id="c" clipPathUnits="userSpaceOnUse">'
              '<text x="2" y="14" font-size="12" font-family="Arial">A</text></clipPath></defs>'
              '<rect width="100" height="80" fill="black" clip-path="url(#c)"/>',
         attrs=[("text", None, {"font-size": "24"})], pixel="exact"),
    dict(name="B4 pattern style font-size", kind="u", teeth=200,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20">'
              '<text x="2" y="14" style="font-size: 12px" font-family="Arial">A</text></pattern></defs>'
              '<rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("text", None, {"style": ("contains", "font-size: 24px")})], pixel="exact"),
    dict(name="B5 pattern tspan", kind="u", teeth=200,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20">'
              '<text x="2" y="14" font-family="Arial"><tspan font-size="12">A</tspan></text>'
              '</pattern></defs><rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("tspan", None, {"font-size": "24"}),
                ("text", None, {"font-size": None})], pixel="exact"),
    dict(name="B6 plain text regression", kind="u1", teeth=0,
         body='<text x="10" y="40" font-size="12" font-family="Arial">Hello</text>',
         attrs=[("text", None, {"font-size": "12"})], pixel="exact"),
    dict(name="B7 viewBox pattern stays", kind="u", teeth=0,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20" '
              'viewBox="0 0 40 20"><text x="2" y="14" font-size="12" font-family="Arial">A</text>'
              '</pattern></defs><rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("text", None, {"font-size": "12"})], pixel="exact"),
    dict(name="B8 pattern text aniso (residual)", kind="a", teeth=2000,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20">'
              '<text x="2" y="14" font-size="12" font-family="Arial">A</text></pattern></defs>'
              '<rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("text", None, {"font-size": 53.67})], pixel=("within", 6500, 255),
         note="residual: one font-size cannot be two different scales"),
    dict(name="B9 position panel keeps 12", kind="p", teeth=0, pixel=None,
         body='<defs><pattern id="p" patternUnits="userSpaceOnUse" width="40" height="20">'
              '<text x="2" y="14" font-size="12" font-family="Arial">A</text></pattern></defs>'
              '<rect width="100" height="80" fill="url(#p)"/>',
         attrs=[("text", None, {"font-size": "12"})]),
    # ---- C: filter / mask regions ------------------------------------------
    dict(name="C1 filter href only", kind="u", teeth=2000,
         body='<defs><filter id="base" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
              'height="80"><feGaussianBlur stdDeviation="2"/></filter>'
              '<filter id="f" href="#base"/></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "f", {"x": None, "y": None, "width": None, "height": None}),
                ("filter", "base", {"width": "200", "height": "160"})],
         pixel="exact", ink_floor=1000, no_zero=True),
    dict(name="C2 filter without region", kind="u", teeth=2000,
         body='<defs><filter id="f" filterUnits="userSpaceOnUse">'
              '<feGaussianBlur stdDeviation="2"/></filter></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "f", {"x": None, "y": None, "width": None, "height": None})],
         pixel="exact", ink_floor=1000, no_zero=True),
    dict(name="C3 mask without region", kind="u", teeth=2000,
         body='<defs><mask id="m" maskUnits="userSpaceOnUse">'
              '<rect width="100" height="80" fill="white"/></mask></defs>'
              '<rect width="100" height="80" fill="black" mask="url(#m)"/>',
         attrs=[("mask", "m", {"x": None, "y": None, "width": None, "height": None})],
         pixel="exact", ink_floor=1000, no_zero=True, cairo_must_render=True),
    dict(name="C4 filter own region", kind="u", teeth=2000,
         body='<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
              'height="80"><feGaussianBlur stdDeviation="2"/></filter></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "f", {"x": "0", "y": "0", "width": "200", "height": "160"})],
         pixel="exact"),
    dict(name="C5 filter href + own region", kind="u", teeth=2000,
         body='<defs><filter id="base" filterUnits="userSpaceOnUse" x="0" y="0" width="100" '
              'height="80"><feGaussianBlur stdDeviation="2"/></filter>'
              '<filter id="f" href="#base" x="0" y="0" width="100" height="80"/></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "f", {"x": "0", "y": "0", "width": "200", "height": "160"})],
         pixel="exact"),
    dict(name="C6 filter obb no region", kind="u", teeth=2000,
         body='<defs><filter id="f"><feGaussianBlur stdDeviation="2"/></filter></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "f", {"x": None, "y": None, "width": None, "height": None})],
         pixel="exact", ink_floor=1000, no_zero=True),
    dict(name="C7 mask own region", kind="u", teeth=0,
         body='<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">'
              '<rect width="100" height="80" fill="white"/></mask></defs>'
              '<rect width="100" height="80" fill="black" mask="url(#m)"/>',
         attrs=[("mask", "m", {"x": "0", "y": "0", "width": "200", "height": "160"})],
         pixel="exact"),
    dict(name="C8 href obb region stays", kind="u", teeth=2000,
         body='<defs><filter id="base" filterUnits="objectBoundingBox" x="0" y="0" width="1" '
              'height="1"><feGaussianBlur stdDeviation="2"/></filter>'
              '<filter id="f" href="#base"/></defs>'
              '<rect x="20" y="20" width="60" height="40" fill="black" filter="url(#f)"/>',
         attrs=[("filter", "base", {"x": "0", "y": "0", "width": "1", "height": "1"}),
                ("filter", "f", {"width": None, "height": None})],
         pixel="exact"),
]


# --------------------------------------------------------------------------- #
# one case
# --------------------------------------------------------------------------- #
def run_case(case):
    name, kind = case["name"], case["kind"]
    cell, oracle_scale, oracle_mode = cfg(kind)
    src = tmp(name + "-src.svg")
    src.write_text(source_svg(kind, case["body"]))
    oracle_svg = tmp(name + "-oracle.svg")
    oracle_svg.write_text(aniso_oracle_svg(case["body"]) if oracle_mode == "aniso" else src.read_text())
    oracle_png = tmp(name + "-oracle.png")
    if not render(oracle_svg, oracle_png, oracle_scale):
        check(False, "%s: oracle rendered" % name)
        return

    out = tmp(name + "-out.svg")
    ok, err = compose(BIN, src, cell, out)
    if not check(ok, "%s: composed" % name, err):
        return
    product_svg = out.read_text()

    for tag, ident, want in case.get("attrs", []):
        expect(name, product_svg, tag, ident, want)
    if case.get("no_zero"):
        no_invented_box(name, product_svg)

    # cairo side (reference only)
    cairo_oracle = tmp(name + "-oracle-cairo.png")
    cairo_err_o = cairo(oracle_svg, cairo_oracle, oracle_scale)
    got = tmp(name + "-got.png")
    render(out, got, 1)
    cairo_got = tmp(name + "-got-cairo.png")
    cairo_err_g = cairo(out, cairo_got, 1)
    new_x, new_md = xor(oracle_png, got)
    new_ink, oracle_ink = ink(got), ink(oracle_png)
    if kind == "p":
        new_x = new_md = None  # a position panel is not a uniform scale: no pixel oracle
    cairo_old = cairo_new = None
    if not (cairo_err_o or cairo_err_g):
        cairo_new = xor(cairo_oracle, cairo_got)[0]

    # old binary: same pipeline, for the four-column table and the teeth check
    old_x = old_md = old_ink = None
    old_svg = tmp(name + "-old.svg")
    old_ok, _ = compose(OLD, src, cell, old_svg) if OLD.exists() else (False, "old binary missing")
    if old_ok:
        old_png = tmp(name + "-old.png")
        render(old_svg, old_png, 1)
        old_x, old_md = xor(oracle_png, old_png)
        old_ink = ink(old_png)
        old_cairo = tmp(name + "-old-cairo.png")
        if cairo(old_svg, old_cairo, 1) is None:
            cairo_old = xor(cairo_oracle, old_cairo)[0]

    if case.get("ink_floor"):
        check(new_ink >= case["ink_floor"], "%s: content is visible" % name,
              "ink %s < %s" % (new_ink, case["ink_floor"]))
        check(new_ink == oracle_ink, "%s: ink equals the oracle" % name,
              "%s vs %s" % (new_ink, oracle_ink))
    if case.get("cairo_must_render"):
        check(cairo_new is not None, "%s: cairosvg renders the product" % name,
              "cairo reported %r" % (cairo_err_g or cairo_err_o))

    pixel = case.get("pixel")
    if pixel == "exact":
        check(new_x == 0, "%s: d1 xor == 0" % name, "xor %s md %s" % (new_x, new_md))
    elif isinstance(pixel, tuple) and pixel[0] == "within":
        check(new_x is not None and new_x <= pixel[1] and new_md <= pixel[2],
              "%s: d1 xor <= %s (residual allowed)" % (name, pixel[1]),
              "xor %s md %s" % (new_x, new_md))

    teeth = case.get("teeth", 0)
    if OLD.exists() and teeth:
        check(old_x is not None and old_x >= teeth,
              "%s: old binary fails this case" % name,
              "old xor %s < %s" % (old_x, teeth))
    elif OLD.exists() and pixel and teeth == 0:
        check(old_x == new_x, "%s: unchanged by the fix" % name, "old %s new %s" % (old_x, new_x))

    ROWS.append(dict(name=name, old=(old_x, old_md), new=(new_x, new_md),
                     ink=(oracle_ink, new_ink), old_ink=old_ink,
                     cairo=(cairo_old, cairo_new), teeth=teeth,
                     note=case.get("note", "")))


# --------------------------------------------------------------------------- #
# pre-fix attribute teeth (independent of the old binary)
# --------------------------------------------------------------------------- #
def raw_teeth():
    """The old binary must still emit the pre-fix values; guard against a
    silently 'fixed' suite when SVG_GRID_BIN points at the pre-fix build."""
    if not OLD.exists():
        skip("teeth: pre-fix attributes", "old binary %s missing" % OLD)
        return
    probes = [
        ("A1", f('<feGaussianBlur stdDeviation="2"/>'), "feGaussianBlur", "stdDeviation", "2"),
        ("N1", f('<feGaussianBlur stdDeviation="2"/>'), "feGaussianBlur", "stdDeviation", "2"),
    ]
    for label, body, tag, attr, want in probes:
        kind = "a" if label == "N1" else "u"
        case = dict(name="teeth-" + label, kind=kind, body=body, pixel=None,
                    attrs=[(tag, None, {attr: want})])
        cell = cfg(kind)[0]
        src = tmp("teeth-" + label + ".svg")
        src.write_text(source_svg(kind, body))
        out = tmp("teeth-" + label + "-old.svg")
        ok, err = compose(OLD, src, cell, out)
        if check(ok, "teeth %s: old composes" % label, err):
            expect("teeth " + label, out.read_text(), tag, None, {attr: want})
    # C1: the pre-fix build invented width="0" height="0" on the href filter
    body = CASES[[c["name"] for c in CASES].index("C1 filter href only")]["body"]
    src = tmp("teeth-C1.svg")
    src.write_text(source_svg("u", body))
    out = tmp("teeth-C1-old.svg")
    ok, err = compose(OLD, src, cell=(200, None), out_svg=out)
    if check(ok, "teeth C1: old composes", err):
        got = element(out.read_text(), "filter", "f")
        check(got is not None and (got.get("width") == "0" or got.get("height") == "0"),
              "teeth C1: old binary invents a zero box",
              "filter f = %r" % (got,))
    # B1: the pre-fix build left the pattern-body font-size at 12
    body = CASES[[c["name"] for c in CASES].index("B1 pattern text")]["body"]
    src = tmp("teeth-B1.svg")
    src.write_text(source_svg("u", body))
    out = tmp("teeth-B1-old.svg")
    ok, err = compose(OLD, src, cell=(200, None), out_svg=out)
    if check(ok, "teeth B1: old composes", err):
        expect("teeth B1", out.read_text(), "text", None, {"font-size": "12"})


# --------------------------------------------------------------------------- #
# main
# --------------------------------------------------------------------------- #
def main():
    global ALLOW_SKIP
    ap = argparse.ArgumentParser()
    ap.add_argument("--allow-skip", action="store_true")
    args = ap.parse_args()
    ALLOW_SKIP = args.allow_skip

    print("R58 evidence — binary %s" % BIN)
    print("renderer %s" % RENDER)
    print("old      %s" % ("%s" % OLD if OLD.exists() else "missing (old columns n/a)"))

    if not check(BIN.is_file() and os.access(BIN, os.X_OK), "svg_grid binary is present", BIN):
        return finish()
    if not RENDER.is_file():
        skip("d1render is present", "missing %s" % RENDER)
        return finish()
    if not FONTDIR.is_dir():
        skip("fontlib is present", "missing %s" % FONTDIR)
        return finish()

    for case in CASES:
        run_case(case)
    raw_teeth()

    print("\n%-34s %13s %13s %15s %11s" % ("case", "d1 修前 xor/md", "d1 修后 xor/md",
                                           "oracle/修后 ink", "cairo 前/后"))
    for row in ROWS:
        old = "n/a" if row["old"][0] is None else "%s/%s" % row["old"]
        new = "n/a" if row["new"][0] is None else "%s/%s" % row["new"]
        ink_s = "%s/%s" % row["ink"]
        cairo = "%s/%s" % tuple("ERR" if v is None else v for v in row["cairo"])
        print("%-34s %13s %13s %15s %11s" % (row["name"], old, new, ink_s, cairo))
    for row in ROWS:
        if row["note"]:
            print("  note %-28s %s" % (row["name"], row["note"]))
    return finish()


def finish():
    print("\n%d passed, %d failed, %d skipped" % (len(PASSED), len(FAILED), len(SKIPPED)))
    if FAILED:
        print("FAILED: " + ", ".join(FAILED))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
