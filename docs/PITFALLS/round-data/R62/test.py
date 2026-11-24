#!/usr/bin/env python3
"""R62 evidence — the content domain of a definition, and `<ellipse>`.

A  content domain (the new axis).  A definition whose *body* is verbatim (a
   pattern with a `viewBox` or `patternContentUnits="objectBoundingBox"`, a
   marker with a `viewBox`) is instantiated in body units: the definitions it
   references keep their tile/coordinates/body unscaled.  A definition shared by
   a user-space and a verbatim referrer is copied per domain.  Eight measured
   cases: each must write the exact attributes and pixel-match the source
   rendered at s=2 (cell 200x160, panel 100x80).  Two controls (a plain nested
   user-space chain, an all-user-space graph) must keep their pre-R62 bytes.
B  `<ellipse>` was never rewritten at all (no dispatch arm).  `cx/cy` scale as a
   pair, `rx` on x, `ry` on y, and a single radius is materialised on the other
   axis (s=8: 20/20/10/5 -> 160/160/80/40).  A malformed radius fails closed in
   both `plot-grid` and `convert --check`.
C  registrations (no code change): a clipPath/mask body that is
   `*Units="objectBoundingBox"` and references a pattern keeps the pre-R62
   numbers, including one that did not converge.
D  teeth: the same A/B assertions run against the pre-fix binary
   (`SVG_GRID_BEFORE_BIN`, default /tmp/r62_before_bin) must go red — every A
   case and every B case, otherwise the evidence proves nothing.

Run:  python3 docs/PITFALLS/round-data/R62/test.py
      python3 docs/PITFALLS/round-data/R62/test.py --allow-skip
Exit: 0 = all passed, 1 = at least one failed (fail-closed: a missing binary,
      renderer or pre-fix binary is a failure unless --allow-skip).
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
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
BEFORE = Path(os.environ.get("SVG_GRID_BEFORE_BIN", "/tmp/r62_before_bin"))
# The binary of the first R62 cut (cc66d4b): the pre-fix build for the P1-b group E.
PREV = Path(os.environ.get("SVG_GRID_PREV_BIN", "/tmp/r62_cc66_bin"))
RENDER = Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r62-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
PASSED, FAILED, SKIPPED, ROWS = [], [], [], []
ALLOW_SKIP = False
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
    return TMP / re.sub(r"[^A-Za-z0-9_.-]+", "_", name)


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
        f = raw[base]
        line = bytearray(raw[base + 1:base + 1 + stride])
        for x in range(stride):
            left = line[x - ch] if x >= ch else 0
            up = prev[x]
            ul = prev[x - ch] if x >= ch else 0
            if f == 1:
                line[x] = (line[x] + left) & 0xFF
            elif f == 2:
                line[x] = (line[x] + up) & 0xFF
            elif f == 3:
                line[x] = (line[x] + (left + up) // 2) & 0xFF
            elif f == 4:
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


def render(svg_path, png_path, scale=1):
    r = subprocess.run([str(RENDER), str(svg_path), str(png_path), str(scale)],
                       env=ENV, capture_output=True, text=True)
    return r.returncode == 0


def compose(binary, src, out, width=200, height=160):
    r = subprocess.run([str(binary), "plot-grid", "--input", str(src), "--output", str(out),
                        "--width", str(width), "--height", str(height), "--ncol", "1",
                        "--plot-margin", "0"], capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def elements(svg, tag):
    return [dict(ATTR_RE.findall(m.group(2))) for m in TAG_RE.finditer(svg)
            if m.group(1) == tag]


def defs_of(svg):
    m = re.search(r"<defs>(.*?)</defs>", svg, re.S)
    return m.group(1) if m else ""


def attr(element, name):
    return element.get(name)


def widths(svg, tag, name):
    return sorted(e[name] for e in elements(defs_of(svg), tag) if name in e)


def gradient_ids(svg):
    return {e["id"]: e for e in elements(defs_of(svg), "linearGradient") if "id" in e}


def fills(svg):
    return [e["fill"][5:-1] for e in elements(defs_of(svg), "rect")
            if e.get("fill", "").startswith("url(#") and e["fill"].endswith(")")]


def body_gradient_x2(svg):
    """The `x2` of every gradient a definition body paints with."""
    grads = gradient_ids(svg)
    return sorted(grads[f].get("x2", "?") for f in fills(svg) if f in grads)


PANEL_HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
              'viewBox="0 0 100 80">')
PANEL_TAIL = "</svg>"


def doc(body):
    return PANEL_HEAD + body + PANEL_TAIL


def gradient(gid, x2):
    return ('<linearGradient id="%s" gradientUnits="userSpaceOnUse" x1="0" y1="0" '
            'x2="%s" y2="0"><stop offset="0" stop-color="red"/>'
            '<stop offset="1" stop-color="blue"/></linearGradient>' % (gid, x2))


CASE_V1 = doc('<defs>'
              '<pattern id="inner" patternUnits="userSpaceOnUse" width="8" height="8">'
              '<rect width="4" height="4" fill="red"/></pattern>'
              '<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16" '
              'viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#inner)"/>'
              '</pattern></defs><rect width="100" height="80" fill="url(#outer)"/>')

CASE_V2 = doc('<defs>' + gradient("g", "16") +
              '<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16" '
              'viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#g)"/>'
              '</pattern></defs><rect width="100" height="80" fill="url(#outer)"/>')

CASE_V3 = doc('<defs>' + gradient("g", "0.5") +
              '<pattern id="outer" patternUnits="objectBoundingBox" '
              'patternContentUnits="objectBoundingBox" width="0.2" height="0.2">'
              '<rect width="1" height="1" fill="url(#g)"/></pattern></defs>'
              '<rect width="100" height="80" fill="url(#outer)"/>')

CASE_M1 = doc('<defs>' + gradient("g", "10") +
              '<marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" '
              'markerHeight="10" refX="0" refY="0" viewBox="0 0 10 10">'
              '<rect width="10" height="10" fill="url(#g)"/></marker></defs>'
              '<path d="M10 10 L90 10" stroke="black" stroke-width="2" '
              'marker-end="url(#m)"/>')

CASE_N3 = doc('<defs>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="4" height="4">'
              '<rect width="2" height="2" fill="red"/></pattern>'
              '<marker id="m" markerUnits="userSpaceOnUse" markerWidth="20" '
              'markerHeight="20" refX="0" refY="0" viewBox="0 0 20 20">'
              '<rect width="20" height="20" fill="url(#p)"/></marker></defs>'
              '<path d="M10 10 L60 10" stroke="black" stroke-width="1" '
              'marker-end="url(#m)"/>')

CASE_V4 = doc('<defs>' + gradient("g", "10") +
              '<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16" '
              'viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#g)"/>'
              '</pattern></defs>'
              '<rect x="0" y="0" width="100" height="40" fill="url(#g)"/>'
              '<rect x="0" y="40" width="100" height="40" fill="url(#outer)"/>')

CASE_N4 = doc('<defs>' + gradient("g", "10") +
              '<marker id="m" markerUnits="userSpaceOnUse" markerWidth="20" '
              'markerHeight="20" refX="0" refY="0" viewBox="0 0 20 20">'
              '<rect width="20" height="20" fill="url(#g)"/></marker></defs>'
              '<rect x="0" y="0" width="100" height="30" fill="url(#g)"/>'
              '<path d="M10 60 L90 60" stroke="black" stroke-width="1" '
              'marker-end="url(#m)"/>')

CASE_M3 = doc('<defs>' + gradient("g", "10") +
              '<pattern id="pv" patternUnits="userSpaceOnUse" width="16" height="16" '
              'viewBox="0 0 16 16"><rect width="16" height="16" fill="url(#g)"/>'
              '</pattern>'
              '<pattern id="pb" patternUnits="objectBoundingBox" '
              'patternContentUnits="objectBoundingBox" width="0.2" height="0.2">'
              '<rect width="1" height="1" fill="url(#g)"/></pattern></defs>'
              '<rect x="0" y="0" width="100" height="26" fill="url(#g)"/>'
              '<rect x="0" y="27" width="100" height="26" fill="url(#pv)"/>'
              '<rect x="0" y="54" width="100" height="26" fill="url(#pb)"/>')

CASE_C1 = doc('<defs>'
              '<pattern id="inner" patternUnits="userSpaceOnUse" width="8" height="8">'
              '<rect width="4" height="4" fill="red"/></pattern>'
              '<pattern id="outer" patternUnits="userSpaceOnUse" width="16" height="16">'
              '<rect width="16" height="16" fill="url(#inner)"/></pattern></defs>'
              '<rect width="100" height="80" fill="url(#outer)"/>')

CASE_C2 = doc('<defs>' + gradient("g", "10") +
              '<pattern id="p" patternUnits="userSpaceOnUse" width="16" height="16">'
              '<rect width="16" height="16" fill="url(#g)"/></pattern></defs>'
              '<rect width="100" height="80" fill="url(#p)"/>')

CASE_M4 = doc('<defs>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8">'
              '<rect width="4" height="4" fill="white"/></pattern>'
              '<clipPath id="c" clipPathUnits="objectBoundingBox">'
              '<rect width="1" height="0.5" fill="url(#p)"/></clipPath></defs>'
              '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>')

CASE_M2 = doc('<defs>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8">'
              '<rect width="4" height="4" fill="white"/></pattern>'
              '<mask id="mk" maskUnits="objectBoundingBox" '
              'maskContentUnits="objectBoundingBox">'
              '<rect width="1" height="1" fill="url(#p)"/></mask></defs>'
              '<rect width="100" height="80" fill="red" mask="url(#mk)"/>')

CASE_N1 = doc('<defs>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10">'
              '<rect width="10" height="5" fill="white"/></pattern>'
              '<mask id="mk" maskUnits="objectBoundingBox" '
              'maskContentUnits="objectBoundingBox">'
              '<rect width="1" height="1" fill="url(#p)"/></mask></defs>'
              '<rect width="100" height="80" fill="red" mask="url(#mk)"/>')

CASE_N2 = doc('<defs>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10">'
              '<rect width="10" height="5" fill="white"/></pattern>'
              '<mask id="mk" maskUnits="userSpaceOnUse" x="0" y="0" width="100" '
              'height="80" maskContentUnits="userSpaceOnUse">'
              '<rect width="100" height="80" fill="url(#p)"/></mask></defs>'
              '<rect width="100" height="80" fill="red" mask="url(#mk)"/>')

B1 = doc('<ellipse cx="20" cy="20" rx="10" ry="5" fill="red"/>')
B2 = doc('<ellipse cx="20" cy="20" rx="10" fill="red"/>')
B3 = doc('<ellipse cx="20" cy="20" ry="10" fill="red"/>')
B4 = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
      'viewBox="0 0 100 80"><rect data-panel-box="main" x="0" y="0" width="100" '
      'height="80" fill="none" stroke="none"/><g data-scale="x">'
      '<ellipse cx="20" cy="20" rx="10" fill="red"/></g></svg>')
B5 = doc('<ellipse cx="20" cy="20" rx="abc" fill="red"/>')
B4_ORACLE = ('<svg xmlns="http://www.w3.org/2000/svg" width="800" height="640" '
             'viewBox="0 0 800 640"><rect width="800" height="640" fill="#FFFFFF"/>'
             '<ellipse cx="160" cy="20" rx="80" ry="10" fill="red"/></svg>')


# E: a definition-to-definition `href` is an out-edge of the referrer's **body**,
# so it follows the body's content domain — even when the referrer itself was
# instantiated in panel user space (R62 review P1-b).
CASE_E1 = doc('<defs>'
              '<pattern id="q" patternUnits="userSpaceOnUse" width="10" height="10">'
              '<rect width="10" height="10" fill="#0a0"/></pattern>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20" '
              'viewBox="0 0 20 20" href="#q"/></defs>'
              '<g data-scale="xy"><rect width="50" height="80" fill="url(#q)"/>'
              '<rect x="50" width="50" height="80" fill="url(#p)"/></g>')

CASE_E2 = doc('<defs>' + gradient("g", "10") +
              '<linearGradient id="h" href="#g"/>'
              '<pattern id="pv" patternUnits="userSpaceOnUse" width="20" height="20" '
              'viewBox="0 0 10 10"><rect width="10" height="10" fill="url(#h)"/>'
              '</pattern></defs>'
              '<g data-scale="xy"><rect width="40" height="80" fill="url(#g)"/>'
              '<rect x="40" width="60" height="80" fill="url(#pv)"/></g>')

CASE_E3B = doc('<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" '
               'x2="0.5" y2="0"><stop offset="0" stop-color="#000"/>'
               '<stop offset="1" stop-color="#fff"/></linearGradient>'
               '<linearGradient id="h" gradientUnits="objectBoundingBox" href="#g"/>'
               '</defs><g data-scale="xy"><rect width="50" height="80" fill="url(#g)"/>'
               '<rect x="50" width="50" height="80" fill="url(#h)"/></g>')

CASE_E4 = doc('<defs>'
              '<pattern id="r" patternUnits="userSpaceOnUse" width="4" height="4">'
              '<rect width="4" height="4" fill="#0a0"/></pattern>'
              '<pattern id="q" patternUnits="userSpaceOnUse" width="8" height="8" href="#r"/>'
              '<pattern id="p" patternUnits="userSpaceOnUse" width="20" height="20" '
              'viewBox="0 0 20 20" href="#q"/></defs>'
              '<g data-scale="xy"><rect width="40" height="80" fill="url(#q)"/>'
              '<rect x="40" width="60" height="80" fill="url(#p)"/></g>')


def hrefs(svg):
    """`href` targets with the composer's cell prefix stripped."""
    return sorted(h.replace("svg-grid-cell0-", "")
                  for h in re.findall(r'href="#([^"]+)"', svg))


def ellipse_attrs(svg):
    found = elements(svg, "ellipse")
    if len(found) != 1:
        return {}
    return found[0]


def attrs_are(svg, tag, index, wanted):
    found = elements(svg, tag)
    if len(found) <= index:
        return False, "missing <%s> #%d in %s" % (tag, index, svg[-200:])
    got = {name: found[index].get(name) for name in wanted}
    return got == wanted, "got %s want %s" % (got, wanted)


def pixel_case(name, svg, binary, width=200, height=160, scale=2):
    src = tmp(name + ".svg")
    src.write_text(svg)
    out = tmp(name + ".cell.svg")
    if out.exists():
        out.unlink()
    rc, message = compose(binary, src, out, width, height)
    if rc != 0 or not out.exists():
        return None, "rc=%d %s" % (rc, message.strip()[:160])
    ours, ref = tmp(name + ".ours.png"), tmp(name + ".ref.png")
    if not render(out, ours, 1) or not render(src, ref, scale):
        return None, "renderer failed"
    return out.read_text(), xor(ours, ref)


def case_a(binary, name, svg, assert_after, expect_ok=True):
    text, diff = pixel_case(name, svg, binary)
    ok = text is not None and diff == (0, 0)
    label = "A/%s pixel-matches the source at s=2" % name
    if text is not None and assert_after:
        ok = ok and assert_after(text)[0]
    return ok, "%s xor=%s" % ("", diff) if text is None else "xor=%s" % (diff,)


def run_a(binary, tag):
    results = {}

    def record(name, svg, assertion):
        text, diff = pixel_case(name + "-" + tag, svg, binary)
        good = text is not None
        detail = "xor=%s" % (diff,)
        if good and assertion:
            passed, why = assertion(text)
            good = good and passed
            detail += " " + str(why) if not passed else ""
        results[name] = (good, detail)
        return good

    record("v1_inner_pattern", CASE_V1, lambda s: (
        widths(s, "pattern", "width") == ["32", "8"]
        and widths(s, "rect", "width") == ["16", "4"], widths(s, "pattern", "width")))
    record("v2_inner_gradient", CASE_V2, lambda s: (
        widths(s, "linearGradient", "x2") == ["16"], widths(s, "linearGradient", "x2")))
    record("v3_obb_parent_grad", CASE_V3, lambda s: (
        widths(s, "linearGradient", "x2") == ["0.5"], widths(s, "linearGradient", "x2")))
    record("m1_marker_grad", CASE_M1, lambda s: (
        widths(s, "linearGradient", "x2") == ["10"]
        and widths(s, "marker", "markerWidth") == ["20"], widths(s, "linearGradient", "x2")))
    record("n3_marker_pat", CASE_N3, lambda s: (
        widths(s, "pattern", "width") == ["4"] and "2" in widths(s, "rect", "width"),
        widths(s, "pattern", "width")))
    record("v4_shared_grad", CASE_V4, lambda s: (
        widths(s, "linearGradient", "x2") == ["10", "20"]
        and body_gradient_x2(s) == ["10"], body_gradient_x2(s)))
    record("n4_shared_marker", CASE_N4, lambda s: (
        widths(s, "linearGradient", "x2") == ["10", "20"]
        and body_gradient_x2(s) == ["10"], body_gradient_x2(s)))
    record("m3_three_domains", CASE_M3, lambda s: (
        widths(s, "linearGradient", "x2") == ["10", "20"]
        and body_gradient_x2(s) == ["10", "10"], body_gradient_x2(s)))
    return results


def run_b(binary, tag):
    results = {}

    def record(name, svg, assertion, expect):
        text, diff = pixel_case(name + "-" + tag, svg, binary)
        good = text is not None and diff == expect
        results[name] = (good, "xor=%s want %s" % (diff, expect))
        return good

    record("b1_both", B1, None, (0, 0))
    record("b2_rx_only", B2, None, (0, 0))
    record("b3_ry_only", B3, None, (0, 0))
    record("b4_scale_x", B4, None, (0, 0))
    return results


def main():
    global ALLOW_SKIP
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-skip", action="store_true")
    args = parser.parse_args()
    ALLOW_SKIP = args.allow_skip

    if not BIN.is_file():
        return 1 if check(False, "binary under test exists", str(BIN)) else 1
    if not RENDER.is_file():
        skip("renderer exists", str(RENDER))
    print("binary under test: %s" % BIN)

    # ── A: content domain ────────────────────────────────────────────────────
    print("\n── A: a verbatim body does not pass the panel policy inward ──")
    for name, svg, wanted in [
        ("v1_inner_pattern", CASE_V1, None),
        ("v2_inner_gradient", CASE_V2, None),
        ("v3_obb_parent_grad", CASE_V3, None),
        ("m1_marker_grad", CASE_M1, None),
        ("n3_marker_pat", CASE_N3, None),
        ("v4_shared_grad", CASE_V4, None),
        ("n4_shared_marker", CASE_N4, None),
        ("m3_three_domains", CASE_M3, None),
    ]:
        text, diff = pixel_case(name, svg, BIN)
        ok = text is not None and diff == (0, 0)
        check(ok, "A/%s pixel-matches the source at s=2" % name, "xor=%s" % (diff,))
        ROWS.append((name, "composed", "composed", "source@s=2", diff))

    # exact attributes, separately so a failure names what is wrong.
    text, _ = pixel_case("a_attrs", CASE_V1, BIN) or ("", None)
    check(widths(text, "pattern", "width") == ["32", "8"]
          and widths(text, "rect", "width") == ["16", "4"],
          "A/v1 inner tile and body stay 8/8+4/4", widths(text, "pattern", "width"))
    text, _ = pixel_case("a_v2", CASE_V2, BIN) or ("", None)
    check(widths(text, "linearGradient", "x2") == ["16"], "A/v2 gradient x2 stays 16",
          widths(text, "linearGradient", "x2"))
    text, _ = pixel_case("a_v3", CASE_V3, BIN) or ("", None)
    check(widths(text, "linearGradient", "x2") == ["0.5"], "A/v3 gradient x2 stays 0.5",
          widths(text, "linearGradient", "x2"))
    text, _ = pixel_case("a_v4", CASE_V4, BIN) or ("", None)
    check(widths(text, "linearGradient", "x2") == ["10", "20"]
          and body_gradient_x2(text) == ["10"],
          "A/v4 shared gradient is copied per domain", body_gradient_x2(text))
    text, _ = pixel_case("a_m3", CASE_M3, BIN) or ("", None)
    check(widths(text, "linearGradient", "x2") == ["10", "20"]
          and body_gradient_x2(text) == ["10", "10"],
          "A/m3 three domains, one verbatim copy", body_gradient_x2(text))

    # controls: both the pixels and the pre-R62 bytes.
    for name, svg in [("c1_nested_plain", CASE_C1), ("c2_all_user", CASE_C2)]:
        text, diff = pixel_case("ctrl-" + name, svg, BIN)
        check(text is not None and diff == (0, 0), "A/%s control still pixel-clean" % name,
              "xor=%s" % (diff,))
        if BEFORE.is_file():
            old, _ = pixel_case("ctrl-old-" + name, svg, BEFORE)
            check(old is not None and old == text, "A/%s control keeps its old bytes" % name)
        else:
            skip("A/%s control keeps its old bytes" % name, "no pre-fix binary")

    # ── E: href follows the referrer body's domain ─────────────────────────
    print("\n── E: a definition's `href` edge follows its own content domain ──")
    for name, svg, before, assertion in [
        ("e1_pattern_href", CASE_E1, (12800, 255), lambda s: (
            widths(s, "pattern", "width") == ["10", "20", "40"]
            and "q--svg-grid-verbatim" in hrefs(s), hrefs(s))),
        ("e3b_gradient_href", CASE_E3B, (16000, 126), lambda s: (
            widths(s, "linearGradient", "x2") == ["0.5", "1"]
            and "g--svg-grid-verbatim" in hrefs(s), hrefs(s))),
        ("e4_three_layers", CASE_E4, (2304, 255), lambda s: (
            widths(s, "pattern", "width") == ["16", "4", "40", "8", "8"]
            and hrefs(s) == ["q--svg-grid-verbatim", "r", "r--svg-grid-verbatim"],
            hrefs(s))),
    ]:
        text, diff = pixel_case(name, svg, BIN)
        ok = text is not None and diff == (0, 0)
        if text is not None:
            passed, why = assertion(text)
            ok = ok and passed
            if not passed:
                print("      %s: %s" % (name, why))
        check(ok, "E/%s pixel-matches the source at s=2" % name, "xor=%s" % (diff,))
        ROWS.append((name, "xor=%s" % (before,), "xor=%s" % (diff,), "source@s=2", diff))

    # the two controls that were already right must not move: one link is only
    # reached from a verbatim body, the other chain resolves to user space.
    for name, svg in [("e2_only_verbatim", CASE_E2)]:
        text, diff = pixel_case(name, svg, BIN)
        check(text is not None and diff == (0, 0), "E/%s control still pixel-clean" % name,
              "xor=%s" % (diff,))
        if PREV.is_file():
            old, _ = pixel_case("ctrl-old-" + name, svg, PREV)
            check(old is not None and old == text,
                  "E/%s control keeps the first cut's bytes" % name)
        else:
            skip("E/%s control keeps the first cut's bytes" % name, "no cc66d4b binary")

    # ── B: ellipse ──────────────────────────────────────────────────────────
    print("\n── B: <ellipse> scales centre and both radii ──")
    for name, svg, wanted in [
        ("b1_both", B1, {"cx": "160", "cy": "160", "rx": "80", "ry": "40"}),
        ("b2_rx_only", B2, {"cx": "160", "cy": "160", "rx": "80", "ry": "80"}),
        ("b3_ry_only", B3, {"cx": "160", "cy": "160", "rx": "80", "ry": "80"}),
        ("b4_scale_x", B4, {"cx": "160", "cy": "20", "rx": "80", "ry": "10"}),
    ]:
        text, diff = pixel_case(name, svg, BIN, 800, 640, 8)
        # b4 scales x only: the source-at-s oracle does not exist for it, its
        # oracle is the hand-built document below.
        ok = text is not None and (diff == (0, 0) or name == "b4_scale_x")
        if text is not None:
            got = ellipse_attrs(text)
            ok = ok and {k: got.get(k) for k in wanted} == wanted
        check(ok, "B/%s scales cx/cy/rx/ry and pixel-matches" % name,
              "xor=%s attrs=%s" % (diff, ellipse_attrs(text or "")))
        ROWS.append((name, "unscaled", "160/160/80/40", "source@s=8", diff))

    # b4 also has a hand-built oracle: the x-only policy translates y.
    oracle = tmp("b4-oracle.svg")
    oracle.write_text(B4_ORACLE)
    assert render(oracle, tmp("b4-oracle.png"), 1)
    product, _ = pixel_case("b4-oracle-product", B4, BIN, 800, 640, 8)
    if product is not None:
        diff = xor(tmp("b4-oracle-product.ours.png"), tmp("b4-oracle.png"))
        check(diff == (0, 0), "B/b4 matches the hand oracle (y translated, x scaled)",
              "xor=%s" % (diff,))
    else:
        check(False, "B/b4 hand oracle product composes")

    src = tmp("b5.svg")
    src.write_text(B5)
    out = tmp("b5.cell.svg")
    rc, message = compose(BIN, src, out)
    check(rc != 0 and not out.exists(),
          "B/b5 a malformed rx fails closed in plot-grid", "rc=%d %s" % (rc, message.strip()))
    r = subprocess.run([str(BIN), "convert", "--input", str(src), "--check"],
                       capture_output=True, text=True)
    report = r.stdout + r.stderr
    check(r.returncode == 1 and "not a plain number" in report and "ellipse" in report,
          "B/b5 convert --check rejects the same input", "rc=%d %s" % (r.returncode, report.strip()[:160]))

    # ── C: registrations (no code change) ───────────────────────────────────
    print("\n── C: registered corners (numbers pinned, nothing changed) ──")
    for name, svg, want in [
        ("m4_clip_pattern", CASE_M4, (16000, 255)),
        ("m2_mask_pattern", CASE_M2, (0, 0)),
        ("n1_mask_obb_pat", CASE_N1, (0, 0)),
        ("n2_mask_user_pat", CASE_N2, (0, 0)),
    ]:
        text, diff = pixel_case(name, svg, BIN)
        check(diff == want, "C/%s keeps its registered numbers" % name,
              "xor=%s want %s" % (diff, want))

    # ── D: teeth ─────────────────────────────────────────────────────────────
    print("\n── D: the pre-fix binary must go red ──")
    if not BEFORE.is_file():
        skip("D/pre-fix binary exists", str(BEFORE))
        return 0 if not FAILED else 1
    for name, svg in [("v1_inner_pattern", CASE_V1), ("v2_inner_gradient", CASE_V2),
                      ("v3_obb_parent_grad", CASE_V3), ("m1_marker_grad", CASE_M1),
                      ("n3_marker_pat", CASE_N3), ("v4_shared_grad", CASE_V4),
                      ("n4_shared_marker", CASE_N4), ("m3_three_domains", CASE_M3)]:
        text, diff = pixel_case("tooth-" + name, svg, BEFORE)
        check(text is None or diff != (0, 0), "D/A %s is red before the fix" % name,
              "xor=%s" % (diff,))
    for name, svg, want in [("b1_both", B1, {"cx": "160", "cy": "160", "rx": "80", "ry": "40"}),
                            ("b2_rx_only", B2, {"cx": "160", "cy": "160", "rx": "80", "ry": "80"}),
                            ("b3_ry_only", B3, {"cx": "160", "cy": "160", "rx": "80", "ry": "80"}),
                            ("b4_scale_x", B4, {"cx": "160", "cy": "20", "rx": "80", "ry": "10"})]:
        text, diff = pixel_case("tooth-" + name, svg, BEFORE, 800, 640, 8)
        got = ellipse_attrs(text or "")
        check(text is None or got != want, "D/B %s is red before the fix" % name, str(got))
    # Group E is red on both earlier cuts: the first R62 cut (cc66d4b, `PREV`) and
    # the round's own base (`BEFORE`, the pre-R62 binary).
    for label, binary in [("the first R62 cut", PREV), ("the pre-R62 binary", BEFORE)]:
        if not binary.is_file():
            skip("D/E binary of %s exists" % label, str(binary))
            continue
        for name, svg in [("e1_pattern_href", CASE_E1), ("e3b_gradient_href", CASE_E3B),
                          ("e4_three_layers", CASE_E4)]:
            text, diff = pixel_case("tooth-" + label.replace(" ", "-") + "-" + name, svg, binary)
            check(text is None or diff != (0, 0),
                  "D/E %s is red on %s" % (name, label), "xor=%s" % (diff,))
    src = tmp("tooth-b5.svg")
    src.write_text(B5)
    out = tmp("tooth-b5.cell.svg")
    rc, _ = compose(BEFORE, src, out)
    check(rc == 0 and out.exists(), "D/B malformed rx was accepted before the fix",
          "rc=%d wrote=%s" % (rc, out.exists()))

    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
