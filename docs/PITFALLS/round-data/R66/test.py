#!/usr/bin/env python3
"""R66 evidence — the W6/W5 ink judge must only count content that paints, and
`--rel-widths` / `--rel-heights` must reject a non-positive weight.

A  ink paint awareness, four columns per case: pre-fix W6 warning, post-fix W6
   warning, what d1render/cairosvg actually ink (a 300x80 probe viewport holding
   the out-of-bounds element), and the composed product, which must be
   byte-identical. Cases span fill / fill-opacity / stroke-opacity /
   stroke-width / transparent / visibility (attr and style) / opacity / display /
   image / use / line, with painting controls that must keep warning.
B  `--rel-*`: a zero weight is rc=2 naming the flag and the raw token, writes no
   output, and the pre-fix 0-size cell is gone; positive weights keep rc=0 and
   identical bytes.
C  parity: 214 tracked `tests/**/*.svg` through `convert` and 58
   `tests/fixtures/**/*.svg` through `plot-grid --width 600 --ncol 1` — rc and
   bytes equal the base build (/tmp/r50base @e0c6033).
D  the six corpus W6 texts that do change: each is the `visibility="hidden"`
   contribution, not a lost true positive — adding `display="none"` to every
   hidden element makes the PRE-FIX build reproduce the POST-FIX text exactly.
T  teeth: the pre-fix build still warns for `fill="none"` and still accepts
   `--rel-widths 0,1`, so setting SVG_GRID_PIN_BIN makes A and B red.

Run:  python3 docs/PITFALLS/round-data/R66/test.py
      SVG_GRID_PIN_BIN=/tmp/r66_before_bin python3 .../R66/test.py   # A/B red
Exit: 0 all green, 1 at least one check red, 2 a missing binary or no cairosvg.
Env:  SVG_GRID_BIN, SVG_GRID_PIN_BIN (default /tmp/r66_before_bin),
      SVG_GRID_PRE_BIN (default /tmp/r50base/target/release/svg_grid),
      SVG_GRID_CAIRO_PYTHON (interpreter that can `import cairosvg`).
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
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r66_before_bin"))
# A and B's "after" column runs on VIS: point SVG_GRID_PIN_BIN at the pre-fix
# build and every "after" assertion MUST go red (that is the teeth).
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path("/tmp/d1_eval/render/target/release/d1render")
FONTDIR = ROOT / "fontlib"
CAIRO_PY = None                                    # resolved in main()
TMP = Path(tempfile.mkdtemp(prefix="r66-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
PASSED, FAILED = [], []

HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="%s" height="%s" '
        'viewBox="0 0 %s %s">\n')
BOX = ('<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
       'fill="none" stroke="none"/>\n')
BLACK_PNG = ("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ"
             "AAAADUlEQVR4nGNgYGD4DwABBAEAX+XDSwAAAABJRU5ErkJggg==")
R200 = '<rect x="200" y="10" width="20" height="20"'
CASES = {   # name: (body, paints, pre_warns, post_warns)
    "visible_rect": (R200 + ' fill="#000"/>', True, True, True),
    "fill_none": (R200 + ' fill="none"/>', False, True, False),
    "fill_none_stroke_w0": (R200 + ' fill="none" stroke="#000" stroke-width="0"/>', False, True, False),
    "fill_transparent": (R200 + ' fill="transparent"/>', False, True, False),
    "fill_opacity_half": (R200 + ' fill="#000" fill-opacity="0.5"/>', True, True, True),
    "stroke_opacity_0": (R200 + ' fill="none" stroke="#000" stroke-opacity="0"/>', False, True, False),
    "stroke_opacity_half": (R200 + ' fill="none" stroke="#000" stroke-opacity="0.5"/>', True, True, True),
    "stroke_w_tiny": (R200 + ' fill="none" stroke="#000" stroke-width="1e-9"/>', False, True, True),  # fail-open
    "vis_hidden_attr": (R200 + ' fill="#000" visibility="hidden"/>', False, True, False),
    "vis_hidden_style": (R200 + ' fill="#000" style="visibility: hidden"/>', False, True, False),
    "g_vis_hidden": ('<g visibility="hidden">' + R200 + ' fill="#000"/></g>', False, True, False),
    "g_vis_hidden_child_visible": ('<g visibility="hidden">' + R200 + ' fill="#000" '
                                   'visibility="visible"/></g>', True, True, True),
    "g_opacity_0": ('<g opacity="0">' + R200 + ' fill="#000"/></g>', False, True, False),
    "g_opacity_half": ('<g opacity="0.5">' + R200 + ' fill="#000"/></g>', True, True, True),
    "display_none": ('<g display="none">' + R200 + ' fill="#000"/></g>', False, False, False),
    "image": ('<image x="200" y="10" width="20" height="20" href="%s"/>' % BLACK_PNG, True, True, True),
    "image_opacity_0": ('<image x="200" y="10" width="20" height="20" opacity="0" href="%s"/>'
                        % BLACK_PNG, False, True, False),
    "use_visible": ('<defs><rect id="r" x="200" y="10" width="20" height="20" fill="#000"/></defs>'
                    '<use href="#r"/>', True, True, True),
    "use_fill_none": ('<defs><rect id="r" x="200" y="10" width="20" height="20" fill="none"/></defs>'
                      '<use href="#r"/>', False, True, False),
    "line_stroke": ('<line x1="200" y1="10" x2="220" y2="30" stroke="#000"/>', True, True, True),
    "line_fill_only": ('<line x1="200" y1="10" x2="220" y2="30" fill="#000"/>', False, True, False),
}
REL = [  # (flag, weights, panels, ncol, pre_rc, pre_text, post_rc, post_text)
    ("--rel-widths", "0,1", 2, 2, 0, "", 2, 'must be positive, got "0"'),
    ("--rel-widths", "1,0", 2, 2, 0, "", 2, 'must be positive, got "0"'),
    ("--rel-widths", "0.0,1", 2, 2, 0, "", 2, 'must be positive, got "0.0"'),
    ("--rel-widths", "-1,1", 2, 2, 2, "missing value for --rel-widths", 2, 'must be positive, got "-1"'),
    ("--rel-widths", "1,-2", 2, 2, 2, "list of non-negative numbers", 2, 'must be positive, got "-2"'),
    ("--rel-widths", "0", 2, 1, 2, "at least one positive", 2, 'must be positive, got "0"'),
    ("--rel-widths", "abc,1", 2, 2, 2, 'must be a number, got "abc"', 2, 'must be a number, got "abc"'),
    ("--rel-heights", "0,1", 2, 1, 0, "", 2, 'must be positive, got "0"'),
    ("--rel-widths", "1,1", 2, 2, 0, "", 0, ""),
    ("--rel-widths", "1e-9,1", 2, 2, 0, "", 0, ""),
    ("--rel-widths", "1,2", 2, 2, 0, "", 0, ""),
    ("--rel-heights", "1,2", 2, 1, 0, "", 0, ""),
]
D_FILES = ["tests/fig2_mixed/out/probe/g.svg",
           "tests/s22_repro/out/skilltest2/Figure.svg",
           "tests/s22_repro/out/skilltest2/rows/row2.svg",
           "tests/s22_repro/out/skilltest3/Figure.svg",
           "tests/s22_repro/out/skilltest3/rows/row2.svg",
           "tests/user_tests/2026-11-20/ABCDE.svg"]


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + (("  [%s]" % (detail,)) if not ok and detail != "" else ""))


def blank(name):
    return HEAD % (100, 80, 100, 80) + BOX + name + "\n</svg>\n"


def write(name, text):
    path = TMP / (re.sub(r"[^A-Za-z0-9]+", "-", name) + ".svg")
    path.write_text(text, encoding="utf-8")
    return path


def w6(binary, src, out):
    out.unlink(missing_ok=True)
    proc = subprocess.run([str(binary), "plot-grid", "--input", str(src), "--output", str(out),
                           "--width", "200", "--ncol", "1", "--verbose"],
                          capture_output=True, text=True)
    text = proc.stderr + proc.stdout
    line = [l for l in text.splitlines() if "ink outside" in l]
    return proc.returncode, (line[0] if line else ""), (out.read_bytes() if out.exists() else b"")


def png_pixels(path):
    """(width, height, channels, rows) of an 8-bit PNG — no third-party import."""
    data = Path(path).read_bytes()
    pos, idat, w, h, ch = 8, b"", 0, 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, color = struct.unpack(">IIBB", body[:10])
            if depth != 8 or color not in (0, 2, 4, 6):
                raise SystemExit("unsupported PNG: depth=%d color=%d" % (depth, color))
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
    return w, h, ch, rows


def ink_pixels(path, x0=0, y0=0, x1=None, y1=None):
    """Count non-white opaque pixels inside [x0,x1) x [y0,y1)."""
    w, h, ch, rows = png_pixels(path)
    x1 = w if x1 is None else x1
    y1 = h if y1 is None else y1
    count = 0
    for y in range(max(0, y0), min(h, y1)):
        row = rows[y]
        for x in range(max(0, x0), min(w, x1)):
            px = row[x * ch:x * ch + ch]
            a = px[3] if ch == 4 else 255
            if a > 0 and not (px[0] == 255 and px[1] == 255 and px[2] == 255):
                count += 1
    return count


def render(src, out, binary=None):
    proc = subprocess.run([str(binary or RENDER), str(src), str(out), "1"], env=ENV,
                          capture_output=True, text=True)
    return proc.returncode, out.exists()


def cairo(src, out):
    proc = subprocess.run([str(CAIRO_PY), "-c",
                           "import cairosvg,sys;cairosvg.svg2png(url=sys.argv[1],"
                           "write_to=sys.argv[2],scale=1)", str(src), str(out)],
                          capture_output=True, text=True)
    return proc.returncode, out.exists()


def family_a():
    print("\n== A 只数真正上色的内容：修前 · 修后 · d1render 实测 · 产物对照 ==")
    for name, (body, paints, pre, post) in CASES.items():
        src = write(name, blank(body))
        rc_pre, line_pre, bytes_pre = w6(PIN, src, TMP / "a-pre.svg")
        rc_post, line_post, bytes_post = w6(VIS, src, TMP / "a-post.svg")
        check((bool(line_pre) == pre) and rc_pre == 0,
              "A/%s 修前 W6 %s（rc=0）" % (name, "报" if pre else "不报"),
              "rc=%d line=%r" % (rc_pre, line_pre))
        check((bool(line_post) == post) and rc_post == 0,
              "A/%s 修后 W6 %s" % (name, "报" if post else "不报"),
              "rc=%d line=%r" % (rc_post, line_post))
        if pre and post:
            check(line_pre == line_post, "A/%s 修后文案与修前逐字节相同" % name,
                  "%r vs %r" % (line_pre, line_post))
        check(bytes_pre == bytes_post, "A/%s 合成产物修前=修后（逐字节）" % name,
              "%d vs %d bytes" % (len(bytes_pre), len(bytes_post)))
        # Reference-renderer column: a 300x80 viewport that contains the
        # out-of-bounds element; ink at x>=100 means it really paints.
        probe = write(name + "-probe", HEAD % (300, 80, 300, 80) + body + "\n</svg>\n")
        d1_png = TMP / "a-d1.png"
        rc, made = render(probe, d1_png)
        d1_ink = ink_pixels(d1_png, 100) if (rc == 0 and made) else -1
        c_png = TMP / "a-cairo.png"
        crc, cmade = cairo(probe, c_png)
        c_ink = ink_pixels(c_png, 100) if (crc == 0 and cmade) else -1
        check(d1_ink >= 0 and (d1_ink > 0) == paints,
              "A/%s d1render 实测 %s（x>=100 像素 %d）" % (name, "有墨" if paints else "无墨", d1_ink),
              "rc=%d ink=%d" % (rc, d1_ink))
        check(c_ink >= 0 and (c_ink > 0) == paints,
              "A/%s cairosvg 实测 %s（x>=100 像素 %d）" % (name, "有墨" if paints else "无墨", c_ink),
              "rc=%d ink=%d" % (crc, c_ink))


def family_b():
    print("\n== B --rel-weights 非正数：修前 0 宽格 · 修后 rc=2 点名 · 不写产物 ==")
    for flag, weights, panels, ncol, pre_rc, pre_text, post_rc, post_text in REL:
        srcs = [write("b-%s-%d" % (flag, i), blank(R200 + ' fill="#000"/>')) for i in range(panels)]

        def run(binary):
            out = TMP / "b.svg"
            out.unlink(missing_ok=True)
            proc = subprocess.run([str(binary), "plot-grid"]
                                  + sum((["--input", str(s)] for s in srcs), [])
                                  + ["--output", str(out), "--width", "200", "--ncol", str(ncol),
                                     flag, weights], capture_output=True, text=True)
            return proc.returncode, proc.stderr + proc.stdout, out.read_bytes() if out.exists() else b""

        rc, err, data = run(PIN)
        if post_rc == 0:
            rc_vis, _, data_vis = run(VIS)
            check(rc == 0 and rc_vis == 0 and data == data_vis and bool(data),
                  "B/%s %s 正数权照旧 rc=0 且产物逐字节不变" % (flag, weights),
                  "pin=%d vis=%d" % (rc, rc_vis))
            continue
        check(rc == pre_rc and pre_text in err and (rc != 0 or bool(data)),
              "B/%s %s 修前 rc=%d%s" % (flag, weights, pre_rc,
                                        "（漏洞：写出产物）" if pre_rc == 0 else ""),
              "rc=%d err=%r" % (rc, err.strip()[-160:]))
        if pre_rc == 0:
            plan = subprocess.run([str(PIN), "plot-grid"]
                                  + sum((["--input", str(s)] for s in srcs), [])
                                  + ["--width", "200", "--ncol", str(ncol), flag, weights, "--plan"],
                                  capture_output=True, text=True).stdout
            cells = [(float(a), float(b)) for a, b in re.findall(r"cell ([0-9.]+)x([0-9.]+)", plan)]
            check(any(w == 0 or h == 0 for w, h in cells),
                  "B/%s %s 修前 --plan 里出现 0 尺寸格子" % (flag, weights), cells)
        rc, err, data = run(VIS)
        check(rc == 2 and post_text in err and not data,
              "B/%s %s 修后 rc=2、点名值、不写产物" % (flag, weights),
              "rc=%d wrote=%s err=%r" % (rc, bool(data), err.strip()[-200:]))


def family_c():
    print("\n== C 语料逐字节不变（对照 /tmp/r50base @e0c6033）==")
    listed = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--", "tests"],
                            capture_output=True, check=True)
    tracked = sorted(rel for rel in listed.stdout.decode().split("\n") if rel.endswith(".svg"))
    fixtures = sorted(str(p.relative_to(ROOT)) for p in ROOT.glob("tests/fixtures/**/*.svg"))
    check(len(tracked) == 214, "C/214 tracked tests/**/*.svg", len(tracked))
    check(len(fixtures) == 58, "C/58 tracked tests/fixtures/**/*.svg", len(fixtures))

    def run(binary, args, out):
        out.unlink(missing_ok=True)
        proc = subprocess.run([str(binary)] + args, capture_output=True)
        return (proc.returncode, out.read_bytes() if out.exists() else b"",
                proc.stdout, proc.stderr)

    diffs = []
    for rel in tracked:
        out = TMP / "c.svg"
        args = ["convert", "--input", str(ROOT / rel), "--output", str(out)]
        if run(PRE, args, out) != run(BIN, args, out):
            diffs.append(rel)
    check(not diffs, "C/214 convert rc+产物逐字节相同", diffs[:3])
    grid = []
    for rel in fixtures:
        out = TMP / "g.svg"
        args = ["plot-grid", "--input", str(ROOT / rel), "--output", str(out),
                "--width", "600", "--ncol", "1"]
        if run(PRE, args, out) != run(BIN, args, out):
            grid.append(rel)
    check(not grid, "C/58 plot-grid rc+产物逐字节相同", grid[:3])


def family_d():
    print("\n== D 语料上 6 处 W6 文案变化 = visibility:hidden 的贡献（非丢真阳性）==")
    for rel in D_FILES:
        src = ROOT / rel
        _, old, _ = w6(PIN, src, TMP / "d-old.svg")
        _, new, _ = w6(BIN, src, TMP / "d-new.svg")
        text = src.read_text(encoding="utf-8")
        stripped = re.sub(r'<([A-Za-z][\w:.-]*)([^>]*?)visibility="hidden"',
                          lambda m: '<%s%sdisplay="none" visibility="hidden"'
                          % (m.group(1), m.group(2)), text)
        hidden = len(re.findall(r'visibility="hidden"', text))
        after = w6(PIN, write("d-" + rel, stripped), TMP / "d-strip.svg")[1]
        check(bool(old) and old != new,
              "D/%s 修前文案与修后不同" % rel, "old=%r new=%r" % (old, new))
        check(after == new,
              "D/%s 把 %d 个 hidden 元素改成 display:none 后，修前二进制复现修后文案"
              % (rel, hidden), "%r vs %r" % (after, new))


def family_t():
    print("\n== T 有牙：修前二进制在 A/B 上必须红 ==")
    check(PIN.is_file() and PIN.read_bytes() != BIN.read_bytes(),
          "T/修前二进制存在且与修后不同：%s" % PIN, str(PIN))
    src = write("t-fill-none", blank(R200 + ' fill="none"/>'))
    _, line, _ = w6(PIN, src, TMP / "t.svg")
    check(bool(line), "T/修前对 fill=\"none\" 越界元素仍报 W6", line)
    srcs = [write("t-1", blank(R200 + ' fill="#000"/>')), write("t-2", blank(R200 + ' fill="#000"/>'))]
    out = TMP / "t2.svg"
    out.unlink(missing_ok=True)
    proc = subprocess.run([str(PIN), "plot-grid"] + sum((["--input", str(s)] for s in srcs), [])
                          + ["--output", str(out), "--width", "200", "--ncol", "2",
                             "--rel-widths", "0,1"], capture_output=True, text=True)
    check(proc.returncode == 0 and out.exists(),
          "T/修前 --rel-widths 0,1 仍 rc=0 且写出 0 宽格", "rc=%d" % proc.returncode)
    print("     (T: SVG_GRID_PIN_BIN is %s; with it set, A and B go red)"
          % ("set -> A/B's after column runs on the pin"
             if os.environ.get("SVG_GRID_PIN_BIN") else "unset, A/B's after column runs on SVG_GRID_BIN"))


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
              "that can `import cairosvg` (family A needs two renderers)",
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
