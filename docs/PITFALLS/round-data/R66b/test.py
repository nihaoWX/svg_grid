#!/usr/bin/env python3
"""R66b evidence — W6 must follow the layer that paints (`<text>/<tspan>`, a
`<pattern>` href chain, `currentColor`), and a 0-px cell must be a usage error.

A tspan decides the text (P1-1); B pattern href + child paint (P1-2/P2-1);
C currentColor (P2-2). Four columns: pre W6, post W6, d1render ink, cairosvg ink
(300x80 probe, x>=100). D `--rel-widths 1e-9,1` (P3-4): pre rc=0 / `cell 0x60.800`;
post rc=2 naming flag + token from run and `--plan`, writing nothing. E parity:
214 convert + 58 plot-grid vs /tmp/r50base. F teeth: pointing PIN at the pre-fix
build reddens A/B/C/D; a missing dependency is rc=2.

Run: python3 docs/PITFALLS/round-data/R66b/test.py   (PIN set -> red)
Exit: 0 green, 1 a check red, 2 a missing binary or no cairosvg.
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
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r66b_before_bin"))
# A/B/C/D's "after" column runs on VIS: point SVG_GRID_PIN_BIN at the pre-fix
# build and every "after" assertion MUST go red (that is the teeth).
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path("/tmp/d1_eval/render/target/release/d1render")
FONTDIR = ROOT / "fontlib"
CAIRO_PY = None                                    # resolved in main()
TMP = Path(tempfile.mkdtemp(prefix="r66b-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
PASSED, FAILED = [], []

HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="%s" height="%s" '
        'viewBox="0 0 %s %s" xmlns:xlink="http://www.w3.org/1999/xlink">\n')
BOX = ('<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
       'fill="none" stroke="none"/>\n')
TEXT = '<text x="200" y="40" font-size="20" %s>%s</text>'
RECT = '<rect x="200" y="10" width="20" height="20" %s/>'
DEFS = ('<defs><pattern id="b" patternUnits="userSpaceOnUse" width="4" height="4">'
        '<rect width="4" height="4" fill="#000"/></pattern>%s</defs>')
# (body, pre_warns, post_warns) — post must equal "d1render really inks it".
A_CASES = [
    ("tspan_fill", TEXT % ('fill="none"', '<tspan fill="#000">A</tspan>'), False, True),
    ("tspan_stroke", '<text x="200" y="40" font-size="20" fill="none">'
     '<tspan stroke="#000">A</tspan></text>', False, True),
    ("hidden_text_visible_tspan", TEXT % ('fill="#000" visibility="hidden"',
                                          '<tspan visibility="visible">A</tspan>'), False, True),
    ("group_fill_none", '<g fill="none">' + TEXT % ('', '<tspan fill="#000">A</tspan>')
     + '</g>', False, True),
    ("use_text", '<defs><text id="t" x="200" y="40" font-size="20" fill="none">'
     '<tspan fill="#000">A</tspan></text></defs><use href="#t"/>', False, True),
    ("tspan_gradient", '<defs><linearGradient id="g"><stop stop-color="#000"/>'
     '</linearGradient></defs>' + TEXT % ('fill="none"', '<tspan fill="url(#g)">A</tspan>'),
     False, True),
    ("only_tspan_none", TEXT % ('fill="#000"', '<tspan fill="none">A</tspan>'), True, False),
    ("partial_tspan_none", TEXT % ('fill="#000"', 'A<tspan fill="none">BB</tspan>'), True, True),
    ("tspan_fill_opacity_0", TEXT % ('fill="#000"',
                                     '<tspan fill-opacity="0">A</tspan>'), True, False),
    ("tspan_visibility_hidden", TEXT % ('fill="#000"',
                                        '<tspan visibility="hidden">A</tspan>'), True, False),
    ("tspan_display_none", TEXT % ('fill="#000"',
                                   '<tspan display="none">A</tspan>'), True, False),
    # d1render ignores a tspan's own `opacity` (ink 92); cairosvg drops it (0).
    # resvg is the oracle (R51/R54), so the judge keeps warning: see CAIRO_SPLIT.
    ("tspan_opacity_0", TEXT % ('fill="#000"', '<tspan opacity="0">A</tspan>'), True, True),
]
B_CASES = [
    ("href", DEFS % '<pattern id="p" href="#b"/>' + RECT % 'fill="url(#p)"', False, True),
    ("xlink_href", DEFS % '<pattern id="p" xlink:href="#b"/>' + RECT % 'fill="url(#p)"',
     False, True),
    ("href_chain", DEFS % ('<pattern id="c" href="#b"/><pattern id="p" href="#c"/>')
     + RECT % 'fill="url(#p)"', False, True),
    ("title_then_href", '<defs><pattern id="b" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="#000"/></pattern>'
     '<pattern id="p" href="#b"><title>t</title></pattern></defs>'
     + RECT % 'fill="url(#p)"', False, True),
    ("child_fill_none", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="none"/></pattern>'
     + RECT % 'fill="url(#p)"', True, False),
    ("child_transparent", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="transparent"/></pattern>'
     + RECT % 'fill="url(#p)"', True, False),
    ("child_fill_opacity_0", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="#000" fill-opacity="0"/></pattern>'
     + RECT % 'fill="url(#p)"', True, False),
    ("child_opacity_0", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="#000" opacity="0"/></pattern>'
     + RECT % 'fill="url(#p)"', True, False),
    ("child_visibility_hidden", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" '
     'width="4" height="4"><rect width="4" height="4" fill="#000" '
     'visibility="hidden"/></pattern>' + RECT % 'fill="url(#p)"', True, False),
    ("group_fill_none_black_child", DEFS % '<pattern id="p" patternUnits="userSpaceOnUse" '
     'width="4" height="4"><g fill="none"><rect width="4" height="4" fill="#000"/></g>'
     '</pattern>' + RECT % 'fill="url(#p)"', True, True),
    # Self-painting pattern must terminate; cairosvg exits 1 on it (d1 inks 0).
    ("self_reference", '<defs><pattern id="p" patternUnits="userSpaceOnUse" width="4" '
     'height="4"><rect width="4" height="4" fill="url(#p)"/></pattern></defs>'
     + RECT % 'fill="url(#p)"', True, False),
    ("cycle", DEFS % ('<pattern id="p" href="#q"/><pattern id="q" href="#p"/>')
     + RECT % 'fill="url(#p)"', False, False),
]
C_CASES = [
    ("transparent_color", RECT % 'fill="currentColor" color="transparent"', True, False),
    ("black_color", RECT % 'fill="currentColor" color="#000"', True, True),
    ("style_transparent", RECT % 'fill="currentColor" style="color: transparent"', True, False),
    # Case-sensitive: an unknown colour keyword is invalid -> black (ink 400).
    ("lowercase_invalid", RECT % 'fill="currentcolor" color="transparent"', True, True),
    ("group_transparent", '<g color="transparent">' + (RECT % 'fill="currentColor"') + '</g>',
     True, False),
    ("stroke_transparent", '<rect x="200" y="10" width="20" height="20" fill="none" '
     'stroke="currentColor" color="transparent"/>', True, False),
    ("important_color", RECT % 'fill="currentColor" color="transparent" '
     'style="color: #000 !important"', True, True),
]
# cairosvg 2.9.1 diverges from resvg/d1 here by design (or exits 1); follow d1.
CAIRO_SPLIT = {"tspan_opacity_0", "title_then_href", "self_reference"}


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
                           "--width", "100", "--height", "80", "--ncol", "1",
                           "--plot-margin", "0", "--verbose"], capture_output=True, text=True)
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


def ink_pixels(path, x0=0):
    """Count non-white opaque pixels right of x0 (the canvas edge at 100)."""
    w, h, ch, rows = png_pixels(path)
    count = 0
    for y in range(h):
        row = rows[y]
        for x in range(max(0, x0), w):
            px = row[x * ch:x * ch + ch]
            a = px[3] if ch == 4 else 255
            if a > 0 and not (px[0] == 255 and px[1] == 255 and px[2] == 255):
                count += 1
    return count


def render(src, out):
    proc = subprocess.run([str(RENDER), str(src), str(out), "1"], env=ENV,
                          capture_output=True, text=True)
    return proc.returncode, out.exists()


def cairo(src, out):
    proc = subprocess.run([str(CAIRO_PY), "-c",
                           "import cairosvg,sys;cairosvg.svg2png(url=sys.argv[1],"
                           "write_to=sys.argv[2],scale=1)", str(src), str(out)],
                          capture_output=True, text=True)
    return proc.returncode, out.exists()


def four_columns(title, cases, tag):
    print("\n== %s ==" % title)
    for name, body, pre, post in cases:
        src = write("%s-%s" % (tag, name), blank(body))
        rc_pre, line_pre, bytes_pre = w6(PIN, src, TMP / "pre.svg")
        rc_post, line_post, bytes_post = w6(VIS, src, TMP / "post.svg")
        check((bool(line_pre) == pre) and rc_pre == 0,
              "%s/%s 修前 W6 %s（rc=0）" % (tag, name, "报" if pre else "不报"),
              "rc=%d line=%r" % (rc_pre, line_pre))
        check((bool(line_post) == post) and rc_post == 0,
              "%s/%s 修后 W6 %s" % (tag, name, "报" if post else "不报"),
              "rc=%d line=%r" % (rc_post, line_post))
        check(bytes_pre == bytes_post,
              "%s/%s 合成产物修前=修后（逐字节）" % (tag, name),
              "%d vs %d bytes" % (len(bytes_pre), len(bytes_post)))
        probe = write("%s-%s-probe" % (tag, name), HEAD % (300, 80, 300, 80) + body + "\n</svg>\n")
        d1_png = TMP / "d1.png"
        rc, made = render(probe, d1_png)
        d1_ink = ink_pixels(d1_png, 100) if (rc == 0 and made) else -1
        check(d1_ink >= 0 and (d1_ink > 0) == post,
              "%s/%s d1render 实测 %s（x>=100 像素 %d）" % (tag, name, "有墨" if post else "无墨",
                                                           d1_ink), "rc=%d ink=%d" % (rc, d1_ink))
        c_png = TMP / "cairo.png"
        crc, cmade = cairo(probe, c_png)
        c_ink = ink_pixels(c_png, 100) if (crc == 0 and cmade) else -1
        if name == "self_reference":
            check(crc != 0, "%s/%s cairosvg 拒绝递归 pattern（rc=%d）" % (tag, name, crc))
        elif name in CAIRO_SPLIT:
            check(c_ink == 0 and d1_ink > 0,
                  "%s/%s cairosvg 与 d1 分叉已登记（d1=%d cairo=%d）" % (tag, name, d1_ink, c_ink))
        else:
            check(c_ink >= 0 and (c_ink > 0) == post,
                  "%s/%s cairosvg 实测 %s（x>=100 像素 %d）" % (tag, name, "有墨" if post else "无墨",
                                                               c_ink), "rc=%d ink=%d" % (crc, c_ink))


def family_d():
    print("\n== D 0 宽格：修前 rc=0 · 修后 rc=2 点名 · --plan 同步 ==")
    srcs = [write("d-%d" % i, blank(RECT % 'fill="#000"')) for i in range(2)]

    def run(binary, extra, plan=False):
        out = TMP / "d.svg"
        out.unlink(missing_ok=True)
        args = [str(binary), "plot-grid"] + sum((["--input", str(s)] for s in srcs), [])
        args += ["--width", "200", "--ncol", "2"] + extra
        if plan:
            args.append("--plan")
        else:
            args += ["--output", str(out)]
        proc = subprocess.run(args, capture_output=True, text=True)
        return proc.returncode, proc.stderr + proc.stdout, (out.read_bytes() if out.exists() else b"")

    rc, err, data = run(PIN, ["--rel-widths", "1e-9,1"])
    check(rc == 0 and bool(data), "D/修前 --rel-widths 1e-9,1 rc=0 且写出 0 宽格",
          "rc=%d err=%r" % (rc, err.strip()[-120:]))
    plan = subprocess.run([str(PIN), "plot-grid"] + sum((["--input", str(s)] for s in srcs), [])
                          + ["--width", "200", "--ncol", "2", "--rel-widths", "1e-9,1", "--plan"],
                          capture_output=True, text=True).stdout
    cells = [(float(a), float(b)) for a, b in re.findall(r"cell ([0-9.]+)x([0-9.]+)", plan)]
    check(any(w == 0 or h == 0 for w, h in cells), "D/修前 --plan 里出现 0 尺寸格子", cells)

    want = 'must leave a cell at least 1 px wide, got "1e-9"'
    rc, err, data = run(VIS, ["--rel-widths", "1e-9,1"])
    check(rc == 2 and want in err and not data, "D/修后 rc=2、点名 flag+原始 token、不写产物",
          "rc=%d wrote=%s err=%r" % (rc, bool(data), err.strip()[-200:]))
    prc, perr, _ = run(VIS, ["--rel-widths", "1e-9,1"], plan=True)
    check(prc == 2 and want in perr, "D/修后 --plan 同样 rc=2 并点名", "rc=%d err=%r" % (prc, perr))
    check(perr == err, "D/--plan 与真实运行报同一条", "%r vs %r" % (perr, err))

    big = [write("d-big-%d" % i, blank(RECT % 'fill="#000"')) for i in range(2)]

    def run_big(binary, weights, width, out_name):
        out = TMP / out_name
        out.unlink(missing_ok=True)
        proc = subprocess.run([str(binary), "plot-grid"]
                              + sum((["--input", str(s)] for s in big), [])
                              + ["--output", str(out), "--width", str(width), "--ncol", "2",
                                 "--rel-widths", weights], capture_output=True, text=True)
        return proc.returncode, (out.read_bytes() if out.exists() else b"")

    pre_rc, pre_data = run_big(PIN, "1e-3,1", 1000, "d-pre.svg")
    rc, data = run_big(VIS, "1e-3,1", 1000, "d-vis.svg")
    check(pre_rc == 0 and rc == 0 and data == pre_data and bool(data),
          "D/修后 1000 宽上 ≈1 px 照旧 rc=0，产物逐字节不变",
          "pre=%d vis=%d" % (pre_rc, rc))


def family_e():
    print("\n== E 语料逐字节不变（对照 /tmp/r50base @e0c6033）==")
    listed = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--", "tests"],
                            capture_output=True, check=True)
    tracked = sorted(rel for rel in listed.stdout.decode().split("\n") if rel.endswith(".svg"))

    def one(binary, args):
        out = TMP / "e.svg"
        out.unlink(missing_ok=True)
        proc = subprocess.run([str(binary)] + args + ["--output", str(out)], capture_output=True)
        return proc.returncode, (out.read_bytes() if out.exists() else b""), proc.stderr

    def same(a, b):  # R67: findings precede the old error line on stderr
        return a[:2] == b[:2] and b[2].endswith(a[2])

    diffs = [rel for rel in tracked
             if not same(one(PRE, ["convert", "--input", str(ROOT / rel)]),
                         one(BIN, ["convert", "--input", str(ROOT / rel)]))]
    check(not diffs, "E/214 convert rc+产物逐字节相同", diffs[:3])
    grid = [str(path.relative_to(ROOT)) for path in sorted(ROOT.glob("tests/fixtures/**/*.svg"))
            if one(PRE, ["plot-grid", "--input", str(path), "--width", "600", "--ncol", "1"])
            != one(BIN, ["plot-grid", "--input", str(path), "--width", "600", "--ncol", "1"])]
    check(not grid, "E/58 plot-grid rc+产物逐字节相同", grid[:3])


def family_f():
    print("\n== F 有牙：修前二进制在 A/B/C/D 上必须红 ==")
    check(PIN.is_file() and PIN.read_bytes() != BIN.read_bytes(),
          "F/修前二进制存在且与修后不同：%s" % PIN, str(PIN))
    # The two blocking false negatives: the pre-fix build is SILENT on both, which
    # is exactly why pointing VIS at it reddens the after columns.
    for tag, body in (("tspan", TEXT % ('fill="none"', '<tspan fill="#000">A</tspan>')),
                      ("pattern", DEFS % '<pattern id="p" href="#b"/>'
                       + RECT % 'fill="url(#p)"')):
        _, line, _ = w6(PIN, write("f-" + tag, blank(body)), TMP / "f.svg")
        check(not line, "F/修前对 %s 上色的越界内容保持静默（假阴性）" % tag, line)
    if os.environ.get("SVG_GRID_PIN_BIN"):
        print("     (VIS is the pin: A/B/C/D's after columns are red)")
    else:
        print("     (unset: after columns run on SVG_GRID_BIN)")


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
              "that can `import cairosvg` (families A-C need two renderers)",
              file=sys.stderr)
        return 2
    print("BIN=%s\nVIS=%s\nPIN=%s\nPRE=%s\nCAIRO_PY=%s\n"
          % (BIN, VIS, PIN, PRE, CAIRO_PY))
    four_columns("A <text>/<tspan> 的上色层：修前 · 修后 · d1render · cairosvg", A_CASES, "A")
    four_columns("B pattern 的 href 链与子元素上色", B_CASES, "B")
    four_columns("C currentColor 解析", C_CASES, "C")
    family_d()
    family_e()
    family_f()
    print("\n%d passed, %d failed" % (len(PASSED), len(FAILED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
