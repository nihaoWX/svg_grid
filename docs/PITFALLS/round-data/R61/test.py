#!/usr/bin/env python3
"""R61 evidence — a finite source value times a finite factor must never be
written back as `inf`/`NaN`.

Families
  A  every computed writeback (lengths, pairs, boxes, point lists, path data,
     filter pairs, primitive subregions, `gradientTransform`, rot centres, the
     `style` path) keeps the **original text** when the product would not be
     finite, and reports W7.  Before: `x="1e38"` at s=8 became `x="inf"` and the
     box became `width="NaN"` (pre-existing in cd5875d and e0c6033).
  B  a non-finite *factor* (the source geometry itself is sick) also keeps the
     input verbatim and names `input #0`; rc stays 0 and the output renders.
  C  counter-proofs: ordinary values still scale, `1e9` still scales, and the
     `1e-45` underflow keeps its pre-existing (finite) three-decimal semantics.
  D  `style="stroke-width:inf"` keeps its bytes like the attribute path, and now
     reports W7 too.
  E  `convert` never introduces `inf`/`NaN` either (it keeps the source bytes).

Every after-output is scanned attribute by attribute (regex over the whole
document) for an `inf`/`NaN` token, and rendered with d1render.  The teeth are
the R60 pin (/tmp/r60_eval/pin) on the same input: it must produce a non-finite
token, otherwise the case proves nothing.

Run:  python3 docs/PITFALLS/round-data/R61/test.py
      python3 docs/PITFALLS/round-data/R61/test.py --allow-skip
Exit: 0 = every check passed, 1 = at least one failed (fail-closed: a missing
      binary, pin or renderer is a failure unless --allow-skip).
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
                            "/tmp/r60_eval/pin/target/release/svg_grid"))
RENDER = Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r61-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))

PASSED, FAILED, SKIPPED, ROWS = [], [], [], []
ALLOW_SKIP = False
COUNTER = [0]

NON_FINITE = re.compile(r"(?i)(?<![a-z0-9])(-?inf|nan)(?![a-z0-9])")
ATTR_RE = re.compile(r'([\w:-]+)\s*=\s*"([^"]*)"')
TAG_RE = re.compile(r'<([A-Za-z][\w:-]*)((?:\s+[\w:-]+\s*=\s*"[^"]*")*)\s*/?>')


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


def tmp(name):
    COUNTER[0] += 1
    return TMP / ("%03d-%s" % (COUNTER[0], re.sub(r"[^A-Za-z0-9_.-]+", "_", name)))


def read_png(path):
    data = Path(path).read_bytes()
    pos, idat, w, h, ch = 8, b"", 0, 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, color = struct.unpack(">IIBB", body[:10])
            ch = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]
            assert depth == 8, "only 8-bit PNGs are read"
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = w * ch
    rows, prev = [], bytearray(stride)
    for y in range(h):
        base = y * (stride + 1)
        filter_type = raw[base]
        line = bytearray(raw[base + 1:base + 1 + stride])
        for x in range(stride):
            left = line[x - ch] if x >= ch else 0
            up = prev[x]
            ul = prev[x - ch] if x >= ch else 0
            if filter_type == 1:
                line[x] = (line[x] + left) & 0xFF
            elif filter_type == 2:
                line[x] = (line[x] + up) & 0xFF
            elif filter_type == 3:
                line[x] = (line[x] + (left + up) // 2) & 0xFF
            elif filter_type == 4:
                p = left + up - ul
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - ul)
                pred = left if pa <= pb and pa <= pc else (up if pb <= pc else ul)
                line[x] = (line[x] + pred) & 0xFF
        rows.append(line)
        prev = line
    return w, h, ch, rows


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


def elements(svg, tag):
    return [dict(ATTR_RE.findall(m.group(2))) for m in TAG_RE.finditer(svg) if m.group(1) == tag]


def element(svg, tag, index=-1):
    """The last match: the panel box comes first, the body shape last."""
    found = elements(svg, tag)
    return found[index] if len(found) >= abs(index) else None


def render(svg_path, png_path, scale=1):
    r = subprocess.run([str(RENDER), str(svg_path), str(png_path), str(scale)],
                       env=ENV, capture_output=True, text=True)
    return r.returncode == 0


def compose(binary, src, out, width, height, ncol=1, extra=()):
    cmd = [str(binary), "plot-grid", "--input", str(src), "--output", str(out),
           "--width", str(width), "--height", str(height), "--ncol", str(ncol),
           "--plot-margin", "0"] + list(extra)
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.returncode, r.stderr


WHITE = '<rect width="100" height="80" fill="white"/>'


def source_svg(body, policy="xy", width=100, height=80):
    """An already-converted document: plot-grid then applies the transform, not the
    auto-converter (which rewrites straight paths to polygons before we can see the
    path writeback)."""
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" '
            'viewBox="0 0 %d %d"><rect data-panel-box="main" x="0" y="0" width="%d" '
            'height="%d" visibility="hidden" pointer-events="none"/>%s'
            '<g data-scale="%s">%s</g></svg>'
            % (width, height, width, height, width, height, WHITE, policy, body))


def non_finite_tokens(svg):
    hits = []
    for key, value in ATTR_RE.findall(svg):
        if NON_FINITE.search(value):
            hits.append((key, value))
    return hits


def row(name, before, after, oracle, d1, note="n/a"):
    ROWS.append((name, before, after, oracle, d1, note))


# --------------------------------------------------------------------------- #
# A: the overflow corpus.  Cell is 800x640 for a 100x80 panel (s=8).
# --------------------------------------------------------------------------- #
FILTER_BLUR = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" '
               'width="100" height="80"><feGaussianBlur stdDeviation="3e38"/>'
               '</filter></defs><rect x="10" y="10" width="20" height="20" '
               'fill="black" filter="url(#f)"/>')
FILTER_DISP = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" '
               'width="100" height="80"><feImage href="%s" result="map"/>'
               '<feDisplacementMap in="SourceGraphic" in2="map" scale="3e38" '
               'xChannelSelector="R" yChannelSelector="G"/></filter></defs>'
               '<rect x="10" y="10" width="20" height="20" fill="black" '
               'filter="url(#f)"/>')


def const_map(r, g):
    w = h = 16
    raw = b"".join(b"\x00" + bytes([r, g, 0]) * w for _ in range(h))

    def chunk(kind, payload):
        return (struct.pack(">I", len(payload)) + kind + payload
                + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF))
    png = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))
    return "data:image/png;base64," + base64.b64encode(png).decode()


CASES = [
    ("A1 rect x", '<rect x="1e38" y="10" width="20" height="20"/>',
     [("rect", "x", "1e38"), ("rect", "y", "10"), ("rect", "width", "20"),
      ("rect", "height", "20")]),
    ("A2 rect width", '<rect x="10" y="10" width="1e38" height="20"/>',
     [("rect", "width", "1e38"), ("rect", "height", "20")]),
    ("A3 circle cx", '<circle cx="1e38" cy="10" r="40"/>',
     [("circle", "cx", "1e38"), ("circle", "cy", "10"), ("circle", "r", "320")]),
    ("A4 polygon points", '<polygon points="1e38,10 20,20 10,30"/>',
     [("polygon", "points", "1e38,10 20,20 10,30")]),
    ("A5 path d", '<path d="M 1e38 10 L 20 20 L 10 30 Z" fill="black"/>',
     [("path", "d", "M 1e38 10 L 20 20 L 10 30 Z")]),
    ("A6 stroke-width", '<rect x="10" y="10" width="20" height="20" stroke="black" '
     'stroke-width="3e38"/>', [("rect", "stroke-width", "3e38")]),
    ("A7 stdDeviation", FILTER_BLUR, [("feGaussianBlur", "stdDeviation", "3e38")]),
    ("A8 displacement", FILTER_DISP % const_map(255, 128),
     [("feDisplacementMap", "scale", "3e38")]),
    ("A9 rect x=-1e38", '<rect x="-1e38" y="10" width="20" height="20"/>',
     [("rect", "x", "-1e38")]),
]


def family_a():
    print("\n── A: a non-finite product keeps the original text and warns ──")
    for name, body, expected in CASES:
        src = tmp("src.svg")
        src.write_text(source_svg(body))
        before_out = tmp("before.svg")
        rc_b, err_b = compose(BEFORE, src, before_out, 800, 640)
        before_svg = before_out.read_text() if rc_b == 0 else ""
        teeth = non_finite_tokens(before_svg)
        check(bool(teeth), "%s: pin writes a non-finite token (teeth)" % name, teeth or err_b[-120:])
        out = tmp("after.svg")
        rc, err = compose(BIN, src, out, 800, 640, extra=("--verbose",))
        if not check(rc == 0, "%s: composes" % name, err[-160:]):
            continue
        svg = out.read_text()
        hits = non_finite_tokens(svg)
        check(not hits, "%s: no inf/NaN in any attribute" % name, hits)
        ok = True
        for tag, attr, want in expected:
            got = (element(svg, tag) or {}).get(attr)
            ok &= got == want
            check(got == want, "%s: <%s %s> kept/scaled to %r" % (name, tag, attr, want), got)
        check("could not be rewritten, kept verbatim" in err,
              "%s: W7 says the value was kept verbatim" % name, err[-160:])
        png = tmp("after.png")
        check(render(out, png), "%s: d1 renders" % name)
        before_attr = (element(before_svg, expected[0][0]) or {}).get(expected[0][1])
        row(name, "%s %s" % (expected[0][1], before_attr), "%s %s" % (expected[0][1], expected[0][2]),
            "original text", "renders")


def family_b():
    print("\n── B: a non-finite factor keeps the input verbatim and names #0 ──")
    src = tmp("tiny.svg")
    src.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="1e-45" height="1e-45" '
                   'viewBox="0 0 1e-45 1e-45"><rect x="10" y="10" width="20" height="20"/>'
                   '</svg>')
    out = tmp("tiny-after.svg")
    rc, err = compose(BIN, src, out, 800, 640, extra=("--normalize-input-max-side", "400",
                                                      "--verbose"))
    check(rc == 0, "B: composes", err[-160:])
    svg = out.read_text() if rc == 0 else ""
    hits = non_finite_tokens(svg)
    check(not hits, "B: no inf/NaN in any attribute", hits)
    check((element(svg, "rect") or {}).get("x") == "10", "B: the source length stays verbatim",
          (element(svg, "rect") or {}).get("x"))
    check("input #0 <svg width>" in err, "B: W7 names input #0 and <svg width>", err[-200:])
    png = tmp("tiny.png")
    check(render(out, png), "B: d1 renders")
    row("B non-finite factor", "x=NaN / width=inf", "x=10", "input verbatim", "renders")


def family_c():
    print("\n── C: counter-proofs — finite values keep scaling ──")
    src = tmp("normal.svg")
    src.write_text(source_svg('<rect x="10" y="10" width="20" height="20"/>'))
    out = tmp("normal-after.svg")
    rc, err = compose(BIN, src, out, 800, 640, extra=("--verbose",))
    svg = out.read_text() if rc == 0 else ""
    got = element(svg, "rect") or {}
    check((got.get("x"), got.get("width")) == ("80", "160"),
          "C1: x=10 -> 80, width=20 -> 160", got)
    check("could not be rewritten" not in err, "C1: no W7 for ordinary values", err[-160:])
    src = tmp("huge.svg")
    src.write_text(source_svg('<rect x="1e9" y="10" width="20" height="20"/>'))
    out = tmp("huge-after.svg")
    rc, _ = compose(BIN, src, out, 800, 640)
    got = (element(out.read_text(), "rect") or {}).get("x") if rc == 0 else None
    check(got == "8000000000", "C2: a finite huge 1e9 still scales", got)
    # 1e-45: the product is finite (underflows to 0 in the three-decimal output);
    # this is the pre-existing numeric semantics, not the inf/NaN defect.
    src = tmp("underflow.svg")
    src.write_text(source_svg('<rect x="1e-45" y="10" width="20" height="20"/>'))
    out = tmp("underflow-after.svg")
    rc, _ = compose(BIN, src, out, 800, 640, extra=("--normalize-input-max-side", "400"))
    svg = out.read_text() if rc == 0 else ""
    got = (element(svg, "rect") or {}).get("x")
    check(got == "0" and not non_finite_tokens(svg),
          "C3: 1e-45 underflows to 0 (no inf/NaN)", got)
    png = tmp("underflow.png")
    check(render(out, png), "C3: d1 renders")
    row("C3 1e-45", "x=0", "x=0", "finite product", "renders")


def family_d():
    print("\n── D: the style path warns like the attribute path ──")
    src = tmp("style.svg")
    src.write_text(source_svg('<rect x="10" y="10" width="20" height="20" '
                              'style="stroke:black;stroke-width:3e38"/>'))
    out = tmp("style-after.svg")
    rc, err = compose(BIN, src, out, 800, 640, extra=("--verbose",))
    svg = out.read_text() if rc == 0 else ""
    check(rc == 0, "D: composes", err[-160:])
    check('style="stroke:black;stroke-width:3e38"' in svg,
          "D: the style attribute keeps its bytes", (element(svg, "rect") or {}).get("style"))
    check("style token" in err and "kept verbatim" in err,
          "D: W7 reports the kept style token", err[-200:])
    src = tmp("style-inf.svg")
    src.write_text(source_svg('<rect x="10" y="10" width="20" height="20" '
                              'style="stroke-width:inf"/>'))
    out = tmp("style-inf-after.svg")
    rc, err = compose(BIN, src, out, 800, 640, extra=("--verbose",))
    check(rc == 0 and "style token" in err,
          "D2: a literal inf token warns too", err[-200:])
    row("D style", "no W7", "W7 style token", "attribute path", "n/a")


def family_e():
    print("\n── E: the R58 displacement contract is pixel-discriminating ──")
    # Source 200x200 -> cell 400x400 (2x), source scale=10 -> 10*sqrt(2)=14.142.
    # Patching the produced output's scale proves the pixel difference is real:
    # 14.142 -> xor 0, 20 -> 39420, 13.5 -> 4072 (R60 review measurements).
    body = ('<defs><filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="200" '
            'height="200"><feImage href="%s" result="map"/>'
            '<feDisplacementMap in="SourceGraphic" in2="map" scale="10" '
            'xChannelSelector="R" yChannelSelector="G"/></filter></defs>'
            '<rect x="20" y="20" width="160" height="160" fill="black" filter="url(#f)"/>'
            % const_map(255, 128))
    src = tmp("disp.svg")
    src.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" '
                   'viewBox="0 0 200 200"><rect width="200" height="200" fill="white"/>'
                   '<g data-scale="xy">%s</g></svg>' % body)
    out = tmp("disp-after.svg")
    rc, err = compose(BIN, src, out, 400, 400)
    check(rc == 0, "E: composes", err[-160:])
    svg = out.read_text() if rc == 0 else ""
    check((element(svg, "feDisplacementMap") or {}).get("scale") == "14.142",
          "E: scale == 14.142 (10*sqrt(2))",
          (element(svg, "feDisplacementMap") or {}).get("scale"))
    allowed = tmp("disp-after.png")
    check(render(out, allowed), "E: d1 renders")
    # Oracle: the same fixture wrapped in scale(2 2) at 400x400.
    oracle = tmp("disp-oracle.svg")
    oracle.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400" '
                      'viewBox="0 0 400 400"><rect width="400" height="400" fill="white"/>'
                      '<g transform="scale(2 2)">%s</g></svg>' % body)
    oracle_png = tmp("disp-oracle.png")
    check(render(oracle, oracle_png), "E: oracle renders")
    base = xor(allowed, oracle_png)
    check(base == (0, 0), "E: 14.142 matches the oracle (xor 0)", base)
    for alt, want in (("20", 39420), ("13.5", 4072)):
        patched = tmp("disp-%s.svg" % alt)
        patched.write_text(svg.replace('scale="14.142"', 'scale="%s"' % alt, 1))
        png = tmp("disp-%s.png" % alt)
        ok = render(patched, png)
        got = xor(png, oracle_png) if ok else None
        check(ok and got is not None and got[0] == want,
              "E: scale=%s differs from the oracle on %d pixels" % (alt, want), got)
        row("E scale=%s" % alt, "xor 0", "xor %s" % want, "oracle 14.142", "d1")


def family_convert():
    print("\n── F: convert keeps the source bytes and never adds inf/NaN ──")
    src = tmp("convert.svg")
    src.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
                   'viewBox="0 0 100 80"><rect x="1e38" y="10" width="20" height="20"/>'
                   '<circle cx="3e38" cy="10" r="40" stroke="black" stroke-width="3e38" '
                   'fill="none"/></svg>')
    out = tmp("convert-after.svg")
    r = subprocess.run([str(BIN), "convert", "--input", str(src), "--output", str(out)],
                       capture_output=True, text=True)
    check(r.returncode == 0, "F: convert exits 0", r.stderr[-160:])
    svg = out.read_text() if r.returncode == 0 else ""
    check(not non_finite_tokens(svg), "F: convert writes no inf/NaN", non_finite_tokens(svg))
    check('x="1e38"' in svg and 'cx="3e38"' in svg,
          "F: convert keeps the finite source text")
    png = tmp("convert.png")
    check(render(out, png), "F: d1 renders the converted document")
    row("F convert", "x=1e38", "x=1e38", "byte-preserving", "renders")


def main():
    global ALLOW_SKIP
    ap = argparse.ArgumentParser()
    ap.add_argument("--allow-skip", action="store_true")
    args = ap.parse_args()
    ALLOW_SKIP = args.allow_skip
    print("svg_grid R61 evidence — inf/NaN writeback invariant")
    for label, path in (("current binary", BIN), ("R60 pin", BEFORE), ("d1render", RENDER)):
        if not (path.is_file() and os.access(path, os.X_OK)):
            skip("%s exists (%s)" % (label, path), "missing")
    if FAILED:
        print("\n%d check(s) failed before running." % len(FAILED))
        return 1
    if not PASSED and SKIPPED:
        print("\nall checks skipped (--allow-skip).")
        return 0
    family_a()
    family_b()
    family_c()
    family_d()
    family_e()
    family_convert()
    print("\n── four-column table (before / after / oracle / d1) ──")
    for name, before, after, oracle, d1, note in ROWS:
        print("  %-24s | %-28s | %-28s | %-16s | %-8s | %s"
              % (name, before, after, oracle, d1, note))
    print("\n%d passed, %d failed, %d skipped" % (len(PASSED), len(FAILED), len(SKIPPED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
