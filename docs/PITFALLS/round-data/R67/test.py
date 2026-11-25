#!/usr/bin/env python3
"""R67 evidence: the non-finite gate covers every attribute the composer really
rewrites, and an `objectBoundingBox` clipPath body stays verbatim.

A  13 classes: pre-fix rc=0 (leak), post-fix rc=1 naming `attr="1e999"`,
   the composer's rewrite evidence (W7 or a silent pattern keep), write-mode
   sentinel; 5 legal controls stay rc=0; 3 href-inherited units cases rejected.
B  obb clip body: pre-fix scaled (d1 xor 32000/128000, ink 16000 vs 32000),
   post-fix verbatim (xor 0/0, ink 16000/16000) for solid and pattern paint;
   user-space clip still rewritten; the pattern still 8→16.
C  214 convert + 58 plot-grid byte-identical to /tmp/r50base @e0c6033.
D  gate_assertions.py strips strings/comments and catches cfg_attr(ignore);
   write-mode failure prints the findings.
T  teeth: SVG_GRID_PIN_BIN=/tmp/r67_before_bin makes A and B red.
Exit: 0 green, 1 red, 2 missing binary.
"""
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
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r67_before_bin"))
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path("/tmp/d1_eval/render/target/release/d1render")
FONTDIR = ROOT / "fontlib"
CAIRO_PY = None
TMP = Path(tempfile.mkdtemp(prefix="r67-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
PASSED, FAILED = [], []

HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="%s" height="%s" '
        'viewBox="0 0 %s %s">\n')
BOX = ('<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
       'fill="none" stroke="none"/>\n')

def grad(tag, attrs):
    return ('<defs><%s id="g" gradientUnits="userSpaceOnUse" %s>'
            '<stop offset="0" stop-color="red"/></%s></defs>'
            '<rect width="100" height="80" fill="url(#g)"/>' % (tag, attrs, tag))


def pat(attrs):
    return ('<defs><pattern id="p" patternUnits="userSpaceOnUse" %s>'
            '<rect width="4" height="4" fill="red"/></pattern></defs>'
            '<rect width="100" height="80" fill="url(#p)"/>' % attrs)


def mask(attrs):
    return ('<defs><mask id="m" maskUnits="userSpaceOnUse" %s>'
            '<rect width="100" height="80" fill="white"/></mask></defs>'
            '<rect width="100" height="80" fill="red" mask="url(#m)"/>' % attrs)


def filt(attrs, prim):
    return ('<defs><filter id="f" filterUnits="userSpaceOnUse" %s>%s</filter></defs>'
            '<rect width="100" height="80" fill="red" filter="url(#f)"/>' % (attrs, prim))


def marker(attrs):
    return ('<defs><marker id="m" markerUnits="userSpaceOnUse" %s>'
            '<rect width="2" height="2" fill="red"/></marker></defs>'
            '<path d="M10 10 L90 70" stroke="red" marker-end="url(#m)"/>' % attrs)


CLASSES = {
    "tspan-x": ('<text x="10" y="20" font-size="10">a<tspan x="1e999" y="30">b</tspan></text>', "x", True),
    "tspan-y": ('<text x="10" y="20" font-size="10">a<tspan x="12" y="1e999">b</tspan></text>', "y", True),
    "lg-x1": (grad("linearGradient", 'x1="1e999"'), "x1", True),
    "rg-cx": (grad("radialGradient", 'cx="1e999" cy="40" r="20"'), "cx", True),
    "rg-r": (grad("radialGradient", 'cx="50" cy="40" r="1e999"'), "r", True),
    "rg-fr": (grad("radialGradient", 'cx="50" cy="40" r="20" fr="1e999"'), "fr", True),
    "pattern-x": (pat('x="1e999" y="0" width="8" height="8"'), "x", False),
    "mask-x": (mask('x="1e999" y="0" width="100" height="80"'), "x", True),
    "filter-x": (filt('x="1e999" y="0" width="100" height="80"',
                      '<feGaussianBlur stdDeviation="1"/>'), "x", True),
    "marker-mw": (marker('markerWidth="1e999" markerHeight="3" refX="0" refY="0"'),
                  "markerWidth", True),
    "marker-refx": (marker('markerWidth="3" markerHeight="3" refX="1e999" refY="0"'),
                    "refX", True),
    "fetile-x": (filt('x="0" y="0" width="100" height="80"',
                      '<feTile x="1e999" y="0" width="10" height="10"/>'), "x", True),
    "fetile-width": (filt('x="0" y="0" width="100" height="80"',
                          '<feTile x="0" y="0" width="1e999" height="10"/>'), "width", True),
}
CONTROLS = {
    "rg-r-obb-pct": '<defs><radialGradient id="g" r="50%">'
                    '<stop offset="0" stop-color="red"/></radialGradient></defs>'
                    '<rect width="100" height="80" fill="url(#g)"/>',
    "pattern-x-obb-pct": '<defs><pattern id="p" x="50%" width="8" height="8">'
                         '<rect width="4" height="4" fill="red"/></pattern></defs>'
                         '<rect width="100" height="80" fill="url(#p)"/>',
    "pattern-width-user-pct": '<defs><pattern id="p" patternUnits="userSpaceOnUse" x="0" '
                              'y="0" width="50%" height="8"><rect width="4" height="4" '
                              'fill="red"/></pattern></defs>'
                              '<rect width="100" height="80" fill="url(#p)"/>',
    "marker-mw-obb": '<defs><marker id="m" markerWidth="1e999">'
                     '<rect width="2" height="2" fill="red"/></marker></defs>'
                     '<path d="M10 10 L90 70" stroke="red" marker-end="url(#m)"/>',
    "fetile-x-obb": '<defs><filter id="f" primitiveUnits="objectBoundingBox">'
                    '<feTile x="1e999"/></filter></defs>'
                    '<rect width="100" height="80" fill="red" filter="url(#f)"/>',
}
INHERITED = {
    "lg-href": ('<defs><linearGradient id="base" gradientUnits="userSpaceOnUse"/>'
                '<linearGradient id="g" href="#base" x1="1e999">'
                '<stop offset="0" stop-color="red"/></linearGradient></defs>'
                '<rect width="100" height="80" fill="url(#g)"/>', "x1"),
    "pattern-href": ('<defs><pattern id="base" patternUnits="userSpaceOnUse"/>'
                     '<pattern id="p" href="#base" x="1e999" width="8" height="8">'
                     '<rect width="4" height="4" fill="red"/></pattern></defs>'
                     '<rect width="100" height="80" fill="url(#p)"/>', "x"),
    "filter-href": ('<defs><filter id="base" filterUnits="userSpaceOnUse"/>'
                    '<filter id="f" href="#base" x="1e999" width="100" height="80">'
                    '<feGaussianBlur stdDeviation="1"/></filter></defs>'
                    '<rect width="100" height="80" fill="red" filter="url(#f)"/>', "x"),
}
CLIP = {
    "plain": '<defs><clipPath id="c" clipPathUnits="objectBoundingBox">'
             '<rect width="1" height="0.5" fill="white"/></clipPath></defs>'
             '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>',
    "pattern": '<defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8">'
               '<rect width="4" height="4" fill="white"/></pattern>'
               '<clipPath id="c" clipPathUnits="objectBoundingBox">'
               '<rect width="1" height="0.5" fill="url(#p)"/></clipPath></defs>'
               '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>',
}


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + (("  [%s]" % (detail,)) if not ok and detail else ""))


def blank(body):
    return HEAD % (100, 80, 100, 80) + BOX + '<g data-scale="xy">%s</g>' % body + "\n</svg>\n"


def write(name, text):
    path = TMP / (re.sub(r"[^A-Za-z0-9]+", "-", name) + ".svg")
    path.write_text(text, encoding="utf-8")
    return path


def run(binary, args):
    proc = subprocess.run([str(binary)] + args, capture_output=True, text=True)
    return proc.returncode, proc.stdout + proc.stderr


def convert_check(binary, src):
    return run(binary, ["convert", "--input", str(src), "--check"])


def convert_write(binary, src, out, keep=False):
    if not keep:
        out.unlink(missing_ok=True)
    rc, text = run(binary, ["convert", "--input", str(src), "--output", str(out)])
    return rc, text, (out.read_bytes() if out.exists() else b"")


def plot_grid(binary, src, out, extra=()):
    out.unlink(missing_ok=True)
    rc, text = run(binary, ["plot-grid", "--input", str(src), "--output", str(out),
                            "--width", "200", "--ncol", "1", "--verbose"] + list(extra))
    return rc, text, (out.read_bytes() if out.exists() else b"")


def ink_mask(path):
    data = Path(path).read_bytes()
    pos, idat, w, h, ch = 8, b"", 0, 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, color = struct.unpack(">IIBB", body[:10])
            if depth != 8 or color not in (0, 2, 4, 6):
                raise SystemExit("unsupported PNG")
            ch = {0: 1, 2: 3, 4: 2, 6: 4}[color]
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride, rows, prev = w * ch, [], bytearray(w * ch)
    for y in range(h):
        base = y * (stride + 1)
        f, line = raw[base], bytearray(raw[base + 1:base + 1 + stride])
        for x in range(stride):
            left = line[x - ch] if x >= ch else 0
            up = prev[x]
            ul = prev[x - ch] if x >= ch else 0
            if f == 1:
                add = left
            elif f == 2:
                add = up
            elif f == 3:
                add = (left + up) // 2
            elif f == 4:
                q = left + up - ul
                pa, pb, pc = abs(q - left), abs(q - up), abs(q - ul)
                add = left if pa <= pb and pa <= pc else (up if pb <= pc else ul)
            else:
                add = 0
            line[x] = (line[x] + add) & 0xFF
        rows.append(line)
        prev = line
    mask = []
    for y in range(h):
        row = rows[y]
        for x in range(w):
            px = row[x * ch:x * ch + ch]
            a = px[3] if ch == 4 else 255
            mask.append(0 if (a == 0 or (px[0] == 255 and px[1] == 255 and px[2] == 255)) else 1)
    return mask


def render(src, out, scale=1):
    proc = subprocess.run([str(RENDER), str(src), str(out), str(scale)], env=ENV,
                          capture_output=True, text=True)
    return proc.returncode, out.exists()


def cairo(src, out, scale=1):
    proc = subprocess.run([str(CAIRO_PY), "-c",
                           "import cairosvg,sys;cairosvg.svg2png(url=sys.argv[1],"
                           "write_to=sys.argv[2],scale=float(sys.argv[3]))",
                           str(src), str(out), str(scale)], capture_output=True, text=True)
    return proc.returncode, out.exists()


def family_a():
    print("\n== A ==")
    for name, (body, attr, rewrites) in CLASSES.items():
        src = write("a-" + name, blank(body))
        rc_pre, text_pre = convert_check(PIN, src)
        check(rc_pre == 0, "A/%s 修前 rc=0 放行" % name, "rc=%d %r" % (rc_pre, text_pre[-160:]))
        rc, text = convert_check(VIS, src)
        check(rc == 1 and ('%s="1e999"' % attr) in text,
              "A/%s 修后 rc=1 点名 %s=\"1e999\"" % (name, attr),
              "rc=%d %r" % (rc, text[-200:]))
        sentinel = TMP / "a-sentinel.svg"
        sentinel.write_text("SENTINEL", encoding="utf-8")
        rc_w, text_w, data = convert_write(VIS, src, sentinel, keep=True)
        check(rc_w == 1 and data == b"SENTINEL" and sentinel.read_text(encoding="utf-8") == "SENTINEL",
              "A/%s 写模式 rc=1 无产物 哨兵完整" % name,
              "rc=%d bytes=%d" % (rc_w, len(data)))
        rc_g, text_g, grid = plot_grid(VIS, src, TMP / "a-grid.svg")
        if rewrites:
            w7 = [l for l in text_g.splitlines() if "could not be rewritten" in l]
            check(rc_g == 0 and bool(w7),
                  "A/%s composer W7 保留 verbatim" % name, "rc=%d %r" % (rc_g, text_g[-200:]))
        else:
            check(rc_g == 0 and ('%s="1e999"' % attr).encode() in grid,
                  "A/%s composer 静默保留（无 W7）" % name, "rc=%d" % rc_g)
    for name, body in CONTROLS.items():
        src = write("a-" + name, blank(body))
        rc, text = convert_check(VIS, src)
        check(rc == 0, "A/%s 合法控制 rc=0" % name, "rc=%d %r" % (rc, text[-200:]))
    for name, (body, attr) in INHERITED.items():
        src = write("a-" + name, blank(body))
        rc, text = convert_check(VIS, src)
        check(rc == 1 and ('%s="1e999"' % attr) in text,
              "A/%s href 继承 units 也拦" % name, "rc=%d %r" % (rc, text[-200:]))


def clip_product(binary, src, out):
    rc, text, data = plot_grid(binary, src, out, ["--height", "160", "--plot-margin", "0"])
    return rc, text, data.decode("utf-8", "replace") if data else ""


def clip_body(svg):
    m = re.search(r"<clipPath[^>]*>(.*?)</clipPath>", svg, re.S)
    return m.group(1) if m else ""


def family_b():
    print("\n== B ==")
    for name, body in CLIP.items():
        src = write("b-" + name, blank(body))
        rc_pre, _, pre = clip_product(PIN, src, TMP / "b-pre.svg")
        rc, _, post = clip_product(VIS, src, TMP / "b-post.svg")
        pre_body, post_body = clip_body(pre), clip_body(post)
        check(rc_pre == 0 and 'width="2" height="1"' in pre_body,
              "B/%s 修前 body 缩放 2x1" % name, pre_body[-200:])
        check(rc == 0 and 'width="1" height="0.5"' in post_body and "x=" not in post_body,
              "B/%s 修后 body verbatim 无 x/y" % name, post_body[-200:])
        d1_src = TMP / "b-src.png"
        d1_pre = TMP / "b-pre.png"
        d1_post = TMP / "b-post.png"
        render(src, d1_src, 2)
        render(TMP / "b-pre.svg", d1_pre, 1)
        render(TMP / "b-post.svg", d1_post, 1)
        ms, mp, mq = ink_mask(d1_src), ink_mask(d1_pre), ink_mask(d1_post)
        xor_pre = sum(a != b for a, b in zip(ms, mp))
        xor_post = sum(a != b for a, b in zip(ms, mq))
        check(xor_pre > 0, "B/%s d1 修前 xor %d/%d" % (name, xor_pre, len(ms)), "")
        check(xor_post == 0 and sum(ms) == sum(mq) == 16000,
              "B/%s d1 修后 xor 0/%d ink %d/%d" % (name, len(ms), sum(ms), sum(mq)),
              "xor=%d" % xor_post)
        c_src, c_pre, c_post = TMP / "b-cs.png", TMP / "b-cp.png", TMP / "b-cq.png"
        cairo(src, c_src, 2)
        cairo(TMP / "b-pre.svg", c_pre, 1)
        cairo(TMP / "b-post.svg", c_post, 1)
        cs, cp, cq = ink_mask(c_src), ink_mask(c_pre), ink_mask(c_post)
        check(sum(a != b for a, b in zip(cs, cq)) == 0 and sum(cs) == sum(cq) == 16000,
              "B/%s cairo 修后 xor 0 ink %d/%d" % (name, sum(cs), sum(cq)), "")
    src = write("b-userspace", blank(
        '<defs><clipPath id="c"><rect x="10" y="10" width="20" height="20"/></clipPath></defs>'
        '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>'))
    rc, _, post = clip_product(VIS, src, TMP / "b-us.svg")
    check(rc == 0 and 'x="20" y="20" width="40" height="40"' in post,
          "B/userSpace clip 仍改写 x=20 y=20 40x40", post[-200:])
    rc, _, post = clip_product(VIS, write("b-pat-src", blank(CLIP["pattern"])), TMP / "b-pat-out.svg")
    check(rc == 0 and 'width="16" height="16"' in post,
          "B/obb clip 的 pattern 仍缩放 8→16", post[-200:])


def family_c():
    print("\n== C ==")
    listed = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--", "tests"],
                            capture_output=True, check=True)
    tracked = sorted(rel for rel in listed.stdout.decode().split("\n") if rel.endswith(".svg"))
    fixtures = sorted(str(p.relative_to(ROOT)) for p in ROOT.glob("tests/fixtures/**/*.svg"))
    check(len(tracked) == 214, "C/214 tracked tests/**/*.svg", len(tracked))
    check(len(fixtures) == 58, "C/58 tracked tests/fixtures/**/*.svg", len(fixtures))

    def one(binary, args, out):
        out.unlink(missing_ok=True)
        proc = subprocess.run([str(binary)] + args, capture_output=True)
        return (proc.returncode, out.read_bytes() if out.exists() else b"",
                proc.stdout, proc.stderr)

    def same(pre, post):
        return (pre[0] == post[0] and pre[1] == post[1] and pre[2] == post[2]
                and (pre[3] == post[3] or post[3].endswith(pre[3])))

    diffs = []
    for rel in tracked:
        out = TMP / "c.svg"
        args = ["convert", "--input", str(ROOT / rel), "--output", str(out)]
        if not same(one(PRE, args, out), one(BIN, args, out)):
            diffs.append(rel)
    check(not diffs, "C/214 convert rc+产物 0 差", diffs[:3])
    grid = []
    for rel in fixtures:
        out = TMP / "g.svg"
        args = ["plot-grid", "--input", str(ROOT / rel), "--output", str(out),
                "--width", "600", "--ncol", "1"]
        if not same(one(PRE, args, out), one(BIN, args, out)):
            grid.append(rel)
    check(not grid, "C/58 plot-grid rc+产物 0 差", grid[:3])


def family_d():
    print("\n== D ==")
    gate = ROOT / "docs/PITFALLS/round-data/R65/gate_assertions.py"
    proc = subprocess.run([sys.executable, str(gate), "--self-test"],
                          capture_output=True, text=True)
    check(proc.returncode == 0 and "PASS  4 bypass self-test(s) all red" in proc.stdout,
          "D/gate --self-test 4 绕过全红", proc.stdout[-200:] + proc.stderr[-200:])
    proc = subprocess.run([sys.executable, str(gate)], capture_output=True, text=True)
    check(proc.returncode == 0 and "PASS  789 `#[test]` function(s) all assert something" in proc.stdout,
          "D/gate 全树 PASS 789", proc.stdout[-200:] + proc.stderr[-200:])
    src = write("d-tspan", blank(CLASSES["tspan-x"][0]))
    rc, text, data = convert_write(VIS, src, TMP / "d.svg")
    check(rc == 1 and 'x="1e999"' in text and "1 problem(s)" in text,
          "D/写模式 stderr 逐条 findings", "rc=%d %r" % (rc, text[-300:]))


def family_t():
    print("\n== T ==")
    check(PIN.is_file() and PIN.read_bytes() != BIN.read_bytes(),
          "T/修前二进制存在", str(PIN))
    src = write("t-tspan", blank(CLASSES["tspan-x"][0]))
    rc, text = convert_check(PIN, src)
    check(rc == 0, "T/修前 tspan 仍 rc=0", "rc=%d %r" % (rc, text[-160:]))
    src = write("t-clip", blank(CLIP["plain"]))
    rc, _, pre = clip_product(PIN, src, TMP / "t.svg")
    check(rc == 0 and 'width="2" height="1"' in pre,
          "T/修前 obb clip 仍缩放", pre[-160:])


def cairo_python():
    candidates = []
    forced = os.environ.get("SVG_GRID_CAIRO_PYTHON")
    if forced:
        candidates.append(Path(forced))
    candidates.append(Path.home() / ".venv/bin/python")
    candidates.append(Path(sys.executable))
    seen = set()
    for one in candidates:
        if str(one) in seen or not one.exists():
            continue
        seen.add(str(one))
        probe = subprocess.run([str(one), "-c", "import cairosvg"],
                               capture_output=True, text=True)
        if probe.returncode == 0:
            return one
    return None


def main():
    for binary, label in ((BIN, "SVG_GRID_BIN"), (PIN, "pre-fix build"),
                          (PRE, "parity baseline"), (RENDER, "d1render")):
        if not binary.is_file():
            print("missing %s: %s" % (label, binary), file=sys.stderr)
            return 2
    global CAIRO_PY
    CAIRO_PY = cairo_python()
    if CAIRO_PY is None:
        print("cairosvg unavailable: set SVG_GRID_CAIRO_PYTHON to an interpreter "
              "that can `import cairosvg` (family B needs two renderers)",
              file=sys.stderr)
        return 2
    print("BIN=%s\nVIS=%s\nPIN=%s\nPRE=%s\nCAIRO_PY=%s\n"
          % (BIN, VIS, PIN, PRE, CAIRO_PY))
    family_a()
    family_b()
    family_c()
    family_d()
    family_t()
    print("\n%d passed, %d failed" % (len(PASSED), len(FAILED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
