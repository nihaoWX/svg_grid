#!/usr/bin/env python3
"""R39 的可跑证据：`patternContentUnits="objectBoundingBox"` 的 body 不被二次缩放。

R38 把 pattern 的 **所有** children 一律送进 `ScalePolicy::Scale`；但 obb 内容的分数
字面量以**引用元素的 bbox** 为参照，而 bbox 已经随 s 缩放 ⇒ 再乘 (sx,sy) 就是双重
缩放（s=0.5 时 ink 4 而非 16，xor 16）。R39 按 `patternContentUnits` 分流：未写或
`userSpaceOnUse` 照旧 `ScalePolicy::Scale`，显式其它值（obb）原样跳过、不递归。

R40 追加（本脚本保留这条修法的证据，不另开轮次）：`patternContentUnits` 必须**类型化**
读取——只有严格 token `objectBoundingBox` 才算 bbox，其它写法（大小写不符、带空白、未知值、
空串）一律按 usvg 回退 `userSpaceOnUse` 并缩放；并且带 `viewBox` 的 pattern 内容在 viewBox
单位里，也必须原样（R39 引入的 B1 与既有 P1-B 由 R40 修复）。F 族就是这两条，把它们放进
R39 的证据而不是 R38，是因为 R38 的证据只谈相位/平移，单位语义是 R39 才写进代码的。

契约（`pitfalls-manager/references/02_consume.md` §4.3）：无参只跑快断言；退出码即结论
（0＝全过）；每族一行 PASS/FAIL；只有 FAIL 才给细节；跳过必须打出来并给原因；幂等、离线、
不写仓库。`--with-old` 额外复跑 R38 证据（数分钟），默认不打。

二进制定位（可用环境变量覆盖）：
  SVG_GRID_BIN          默认 <repo>/target/debug/svg_grid
  SVG_GRID_CONVERT_BIN  默认 <repo>/target/debug/svg_grid（R42：转换器是它的 `convert` 子命令）
  SVG_GRID_RENDER_BIN   默认 /tmp/d1_eval/render/target/release/d1render （渲染对照，缺失即 FAIL）
  SVG_GRID_CAIRO_PYTHON 默认 ~/.venv/bin/python（能 import cairosvg 才启用第二渲染器）
渲染器缺失时渲染对照类判据 **FAIL**；确实要跳过请显式传 `--allow-skip`。
"""
import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R39 → PITFALLS → docs → svg_grid
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN", SVG_GRID / "target/debug/svg_grid"))
CONV = pathlib.Path(os.environ.get("SVG_GRID_CONVERT_BIN", SVG_GRID / "target/debug/svg_grid"))
CONVERT_ARGS = ["convert"]   # R42：转换器是主二进制的 `convert` 子命令
RENDER = pathlib.Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO_PYTHON = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(pathlib.Path.home() / ".venv/bin/python")))
ALLOW_SKIP = "--allow-skip" in sys.argv
WITH_OLD = "--with-old" in sys.argv

TMP = pathlib.Path(tempfile.mkdtemp(prefix="r39ev-"))
NS = 'xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"'
PANEL = '<rect data-panel-box="main" x="0" y="0" width="{w}" height="{h}" fill="none" stroke="none"/>'

FAILS = []
SKIPS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        if detail:
            print(("      " + str(detail).replace("\n", "\n      "))[:1200])
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label}  （{reason}）")
    SKIPS.append(label)


def skip_render(label, reason):
    if ALLOW_SKIP:
        skip(label, reason + "；显式 --allow-skip")
    else:
        check(False, label,
              reason + "（渲染对照缺失 ⇒ FAIL；确实要跳过请显式传 --allow-skip）")


def run(cmd):
    p = subprocess.run([str(c) for c in cmd], capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"rc={p.returncode} {' '.join(map(str, cmd))}\n{p.stderr[-600:]}")
    return p


# ── 最小 PNG 读入（d1render / cairosvg 都是 8-bit RGBA 或 RGB） ──────────────────────
def read_png(path):
    data = pathlib.Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat, w, h, ct = 8, b"", 0, 0, 6
    while pos < len(data):
        (ln,) = struct.unpack(">I", data[pos:pos + 4])
        typ = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + ln]
        pos += 12 + ln
        if typ == b"IHDR":
            w, h, _bd, ct = struct.unpack(">IIBB", chunk[:10])
        elif typ == b"IDAT":
            idat += chunk
        elif typ == b"IEND":
            break
    raw = zlib.decompress(idat)
    bpp = {0: 1, 2: 3, 4: 2, 6: 4}[ct]
    stride = w * bpp
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f = raw[i]
        i += 1
        line = bytearray(raw[i:i + stride])
        i += stride
        if f == 1:
            for x in range(bpp, stride):
                line[x] = (line[x] + line[x - bpp]) & 255
        elif f == 2:
            for x in range(stride):
                line[x] = (line[x] + prev[x]) & 255
        elif f == 3:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                line[x] = (line[x] + ((a + prev[x]) >> 1)) & 255
        elif f == 4:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                b = prev[x]
                c = prev[x - bpp] if x >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        rows.append(bytes(line))
        prev = line
    return w, h, bpp, rows


def pixel(img, x, y):
    w, h, bpp, rows = img
    if x < 0 or y < 0 or x >= w or y >= h:
        return (255, 255, 255)
    row = rows[y]
    if bpp == 1:
        return (row[x],) * 3
    r, g, b = row[x * bpp:x * bpp + 3]
    if bpp == 4:
        a = row[x * bpp + 3]
        return tuple((c * a + 255 * (255 - a)) // 255 for c in (r, g, b))
    return (r, g, b)


def to_rows(img):
    """归一化成 (w, h, rows)，rows[y][x] 是 RGB 三元组（alpha 已合成到白底）。"""
    if len(img) == 3:
        return img
    w, h, _bpp, rows = img
    return w, h, [[pixel(img, x, y) for x in range(w)] for y in range(h)]


def ink(img):
    w, h, rows = to_rows(img)
    return sum(1 for y in range(h) for x in range(w) if rows[y][x] != (255, 255, 255))


def render(svg, tag, scale=1.0, cairo=False):
    png = TMP / f"{tag}.png"
    if cairo:
        code = ("import sys, cairosvg; cairosvg.svg2png(url=sys.argv[1], write_to=sys.argv[2], "
                "scale=float(sys.argv[3]), background_color='white')")
        p = subprocess.run([str(CAIRO_PYTHON), "-c", code, str(svg), str(png), str(scale)],
                           capture_output=True, text=True)
        if p.returncode != 0:
            raise RuntimeError(p.stderr[-600:])
    else:
        run([RENDER, svg, png, scale])
    return read_png(png)


def paste_at(img, dx, dy, size):
    w, h = size
    iw, ih, rows = to_rows(img)
    canvas = []
    for y in range(h):
        out = []
        for x in range(w):
            sx, sy = x - dx, y - dy
            out.append(rows[sy][sx] if (0 <= sx < iw and 0 <= sy < ih) else (255, 255, 255))
        canvas.append(out)
    return w, h, canvas


def diff(a, b):
    aw, ah, arows = to_rows(a)
    bw, bh, brows = to_rows(b)
    xor = maxdiff = 0
    for y in range(min(ah, bh)):
        for x in range(min(aw, bw)):
            ca, cb = arows[y][x], brows[y][x]
            if ca != cb:
                xor += 1
                maxdiff = max(maxdiff, max(abs(ca[i] - cb[i]) for i in range(3)))
    return xor, maxdiff


def write_svg(name, body):
    p = TMP / name
    p.write_text(body)
    return p


def panel(w, h):
    return ('<svg width="%d" height="%d" viewBox="0 0 %d %d" %s>' % (w, h, w, h, NS)) \
        + PANEL.format(w=w, h=h)


def compose(src, out_name, *extra):
    run([BIN, "plot-grid", "--input", src, "--output", TMP / out_name, *extra])
    return (TMP / out_name).read_text()


# ── A. `objectBoundingBox` 内容：分数字面量必须逐字保留，且 s=1/s=2 同时成立 ──────────
# 旧形态（R38 A2）是空洞的：tile 4×6、body x=0.1（0.1×40=4 正好落在 tile 右缘外）⇒ 源图
# 本身 ink=0，三张全空 ⇒ diff 0 什么都没证明。这里 tile 与引用矩形同尺寸（40×30），body
# 完整落在 tile 内；tile 的 x/y 在 s1/s2 分别是 10/10 与 20/20，缩放若没发生也逃不掉。
OBB_BODY = '<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>'
US_BODY = '<rect x="7" y="3" width="6" height="4" fill="#000000"/>'


def family_a():
    src = write_svg("a_obb.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" '
        'x="10" y="10" width="40" height="30">' + OBB_BODY + '</pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>'))
    outs = {}
    for w, h, tag in ((100, 80, "s1"), (200, 160, "s2")):
        name = f"a_obb_grid_{tag}.svg"
        svg = compose(src, name, "--width", str(w), "--height", str(h), "--ncol", "1",
                      "--plot-margin", "0")
        outs[tag] = TMP / name
        # 双重缩放的签名：s2 下会变成 0.200/0.400/0.500，s0.5 下会变成 0.050/0.100/0.250。
        doubled = 'x="0.200"' in svg or 'x="0.050"' in svg
        check(OBB_BODY in svg and not doubled,
              f"A1 obb 内容 @{tag}：分数字面量逐字保留（无 (sx,sy) 二次缩放）",
              svg[:900] if len(svg) < 200000 else "")
    check('x="10" y="10" width="40" height="30"' in outs["s1"].read_text() and
          'x="20" y="20" width="80" height="60"' in outs["s2"].read_text(),
          "A2 obb tile（userSpaceOnUse）仍随 s 平移缩放（s1≠s2）", "")
    return src, outs


def family_b(src, outs):
    if not RENDER.exists():
        skip_render("B1 obb 内容 resvg 逐像素对照（s=1/s=2）", f"渲染器不存在：{RENDER}")
        return
    for tag, scale in (("s1", 1.0), ("s2", 2.0)):
        # 源图按 s 渲染后应与产物（已含 s 倍画布）逐像素一致：obb 内容若被二次缩放，
        # body 会缩到 tile 左上角 ⇒ xor 立刻非 0。
        src_img = render(src, f"b_obb_src_{tag}", scale)
        out_img = render(outs[tag], f"b_obb_out_{tag}", 1.0)
        xor, md = diff(src_img, out_img)
        si, oi = ink(src_img), ink(out_img)
        check(xor == 0 and md == 0 and si > 0 and oi > 0,
              f"B1 obb @{tag} resvg：源图×s vs 产物 ⇒ diff 0（且两边真有墨）",
              f"xor={xor} maxdiff={md} ink_src={si} ink_out={oi}")
    # cairosvg 没有实现 patternContentUnits=objectBoundingBox：它把分数当 user unit
    # （实测 body 亚像素、ink 恒为 1），因此像素判据只认 resvg，cairo 只判「没有整块消失」。
    if CAIRO_PYTHON.exists() and subprocess.run(
            [str(CAIRO_PYTHON), "-c", "import cairosvg"], capture_output=True).returncode == 0:
        for tag in ("s1", "s2"):
            si = ink(render(src, f"b_obb_src_c_{tag}", 1.0, cairo=True))
            oi = ink(render(outs[tag], f"b_obb_out_c_{tag}", 1.0, cairo=True))
            check(si > 0 and oi > 0 and oi <= 4 * si and si <= 4 * oi,
                  f"B2 obb @{tag} cairosvg：只判 ink 量级（obb 语义以 resvg 为准）",
                  f"ink_src={si} ink_out={oi}")
    else:
        skip("B2 obb cairosvg ink 量级", f"没有可用的 cairosvg 解释器：{CAIRO_PYTHON}")


# ── C. `userSpaceOnUse` 内容不回退（R38 的修法必须原样保留） ────────────────────────
def family_c():
    src = write_svg("c_us.svg", panel(120, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="q" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + US_BODY + '</pattern></defs>'
        '<rect x="10" y="10" width="100" height="60" fill="url(#q)"/></g></svg>'))
    svg = compose(src, "c_us_grid.svg", "--width", "240", "--height", "160", "--ncol", "1",
                  "--plot-margin", "0")
    check('<rect x="14" y="6" width="12" height="8" fill="#000000"/>' in svg,
          "C1 userSpaceOnUse 内容：body 每个数字 ×2 且不平移（R38 行为不回退）",
          svg[:900])
    if not RENDER.exists():
        skip_render("C2 userSpaceOnUse resvg 逐像素对照", f"渲染器不存在：{RENDER}")
        return
    src_img = render(src, "c_us_src", 2.0)
    out_img = render(TMP / "c_us_grid.svg", "c_us_out", 1.0)
    xor, md = diff(src_img, out_img)
    si, oi = ink(src_img), ink(out_img)
    check(xor == 0 and md == 0 and si > 0 and oi > 0,
          "C2 userSpaceOnUse @s=2 resvg：源图×2 vs 产物 ⇒ diff 0（且两边真有墨）",
          f"xor={xor} maxdiff={md} ink_src={si} ink_out={oi}")


# ── D. 账本更正所依据的事实（Blocker 2）：R42 之后独立二进制真的没了 ────────────────
def family_d():
    check(not (SVG_GRID / "convert").exists() and (SVG_GRID / "src/convert/mod.rs").is_file(),
          "D1 R42：独立 `convert/` crate 已删除，转换器是 `src/convert/` 模块",
          f"convert/ exists={(SVG_GRID / 'convert').exists()}")
    check(CONV.exists() and os.access(CONV, os.X_OK),
          "D2 转换器仍可用（同一二进制的 `convert` 子命令）",
          f"缺 {CONV}（先 cargo build --offline）")
    if CONV.exists():
        p = subprocess.run([str(CONV), *CONVERT_ARGS, "--help"], capture_output=True, text=True)
        help_text = p.stdout + p.stderr
        check(p.returncode == 0 and "svg_grid_convert" not in help_text,
              "D3 `svg_grid convert --help` 不再自称 svg_grid_convert", help_text[:400])
    install = (SVG_GRID / "install.sh").read_text()
    check('stale_convert="svg_grid_convert"' in install and
          'rm -f -- "$prefix/$stale_convert"' in install,
          "D4 install.sh 只清理 stale svg_grid_convert，不再安装它（真实失效依据是安装面）",
          install[1200:1800])
    # R38 证据的 A2 已改非退化；这条静态守卫防止有人把空洞形态改回去。
    r38 = (SVG_GRID / "docs/PITFALLS/round-data/R38/test.py").read_text()
    check('a2_outputs["s2"]' in r38 and "obb 内容语义两渲染器不一致" not in r38 and
          '"A2s2"' in r38,
          "D5 R38 证据保持非退化（A2 跑 s1+s2，不再用 obb SKIP 掩盖）", "")


# ── F. R40 追加：类型化 token + viewBox 内容原样（B1 / P1-B） ──────────────────────
def family_f():
    """R39 把「任何非 userSpaceOnUse」当 bbox；resvg 只认精确 token。"""
    for value in ["ObjectBoundingBox", "objectBoundingBox ", " objectBoundingBox", "foo", ""]:
        src = write_svg(f"f_inv_{abs(hash(value))}.svg", panel(100, 80) + (
            '<g data-scale="xy"><defs>'
            f'<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="{value}" '
            'x="10" y="10" width="20" height="20">'
            '<rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern></defs>'
            '<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>'))
        svg = compose(src, f"f_inv_{abs(hash(value))}_out.svg", "--width", "200", "--height",
                      "160", "--ncol", "1", "--plot-margin", "0")
        check('<rect x="4" y="6" width="12" height="8" fill="#000000"/>' in svg,
              f"F1 patternContentUnits={value!r} 不是 bbox token ⇒ body 按 user 单位缩放",
              svg[:900])

    src = write_svg("f_vb.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="p" patternUnits="userSpaceOnUse" viewBox="0 0 10 10" '
        'x="10" y="10" width="20" height="20">'
        '<rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>'))
    svg = compose(src, "f_vb_out.svg", "--width", "200", "--height", "160", "--ncol", "1",
                  "--plot-margin", "0")
    check('<rect x="2" y="3" width="6" height="4" fill="#000000"/>' in svg,
          "F2 viewBox 内容在 viewBox 单位里 ⇒ 逐字保留（不乘 (sx,sy)）", svg[:900])

    src = write_svg("f_vb_href.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="base" viewBox="0 0 10 10"/>'
        '<pattern id="p" href="#base" patternUnits="userSpaceOnUse" '
        'x="10" y="10" width="20" height="20">'
        '<rect x="2" y="3" width="6" height="4" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>'))
    svg = compose(src, "f_vb_href_out.svg", "--width", "200", "--height", "160", "--ncol", "1",
                  "--plot-margin", "0")
    check('<rect x="2" y="3" width="6" height="4" fill="#000000"/>' in svg,
          "F3 viewBox 经 href 继承 ⇒ 内容逐字保留", svg[:900])


# —— E. 旧证据复跑（可选，数分钟） ──────────────────────────────────────────────────
def family_e():
    if not WITH_OLD:
        skip("E1 R38 证据复跑", "传 --with-old 启用（数分钟；R39 已单独复跑过 rc=0）")
        return
    p = subprocess.run([sys.executable, str(SVG_GRID / "docs/PITFALLS/round-data/R38/test.py")],
                       capture_output=True, text=True, timeout=1800)
    check(p.returncode == 0, "E1 R38 证据 rc=0（A2 obb 非退化后仍全绿）",
          f"rc={p.returncode}\n{p.stdout[-800:]}")


def main():
    print(f"== R39 证据（bin={BIN}） ==")
    if not BIN.exists():
        check(False, "A0 svg_grid 二进制存在", f"缺 {BIN}（先 cargo build --offline）")
        return finish()
    try:
        src, outs = family_a()
    except RuntimeError as exc:
        check(False, "A0 组合 obb pattern 失败", str(exc))
        src, outs = None, None
    if src is not None:
        family_b(src, outs)
    family_c()
    family_f()
    family_d()
    family_e()
    return finish()


def finish():
    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print(" · " + f)
        return 1
    if SKIPS:
        print(f"\n== 全部成立（另有 {len(SKIPS)} 条 SKIP，原因见上） ==")
    else:
        print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
