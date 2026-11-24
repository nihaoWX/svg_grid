#!/usr/bin/env python3
"""R38 的可跑证据：`<pattern>` body 只缩放不平移 + `<defs>` 递归下钻。

契约（`tools/pitfalls-manager/references/02_consume.md` §4.3）：无参只跑快断言；退出码即结论
（0＝全过）；每族一行 PASS/FAIL；只有 FAIL 才给细节；跳过必须打出来并给原因；幂等、离线、不写仓库。

二进制定位（可用环境变量覆盖，便于对照 BASE）：
  SVG_GRID_BIN          默认 <repo>/target/debug/svg_grid
  SVG_GRID_CONVERT_BIN  默认 <repo>/target/debug/svg_grid（R42：转换器是它的 `convert` 子命令）
  SVG_GRID_RENDER_BIN   默认 /tmp/d1_eval/render/target/release/d1render （渲染对照，缺失即 FAIL）
  SVG_GRID_CAIRO_PYTHON 默认 ~/.venv/bin/python（能 import cairosvg 才启用第二渲染器对照）
渲染器缺失时渲染对照类判据 **FAIL**；确实要跳过请显式传 `--allow-skip`（R38 修的就是
「缺渲染器 ⇒ 静默 SKIP ⇒ 仍打印全部成立、rc=0」）。
"""
import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R38 → PITFALLS → docs → svg_grid
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN", SVG_GRID / "target/debug/svg_grid"))
CONV = pathlib.Path(os.environ.get("SVG_GRID_CONVERT_BIN", SVG_GRID / "target/debug/svg_grid"))
CONVERT_ARGS = ["convert"]   # R42：转换器是主二进制的子命令，不再是独立二进制
RENDER = pathlib.Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO_PYTHON = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(pathlib.Path.home() / ".venv/bin/python")))
ALLOW_SKIP = "--allow-skip" in sys.argv

TMP = pathlib.Path(tempfile.mkdtemp(prefix="r38ev-"))
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


def compose(src, out_name, *extra):
    run([BIN, "plot-grid", "--input", src, "--output", TMP / out_name, *extra])
    return (TMP / out_name).read_text()


def convert(src, out_name):
    run([CONV, *CONVERT_ARGS, "--input", src, "--output", TMP / out_name])
    return (TMP / out_name).read_text()


def bare(w, h):
    return '<svg width="%d" height="%d" viewBox="0 0 %d %d" %s>' % (w, h, w, h, NS)


def panel(w, h):
    return ('<svg width="%d" height="%d" viewBox="0 0 %d %d" %s>' % (w, h, w, h, NS)) \
        + PANEL.format(w=w, h=h)


# ── A. pattern 相位：body 是 tile 局部坐标 ⇒ 只缩放、不平移 ────────────────────────
def family_a():
    # A1 userSpaceOnUse + s≠1：tile 全映射，body 停在 tile 原点。
    src = write_svg("a1_us.svg", panel(120, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="h" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20">'
        '<rect x="0" y="0" width="20" height="20" fill="#000000"/></pattern></defs>'
        '<rect x="0" y="0" width="120" height="80" fill="url(#h)"/></g></svg>'))
    out = TMP / "a1_us_grid.svg"
    svg = compose(src, "a1_us_grid.svg", "--width", "148", "--ncol", "1", "--plot-margin", "24")
    check('x="24" y="24" width="16.667" height="16.667"' in svg and
          '<rect x="0" y="0" width="16.667" height="16.667"' in svg,
          "A1 userSpaceOnUse：tile 平移、body 停在 tile 原点", svg[:900] if len(svg) < 200000 else "")

    # A2 objectBoundingBox 内容：分数字面量以**引用元素的 bbox** 为参照，而 bbox 已随 s
    # 缩放 ⇒ 再乘 (sx,sy) 就是双重缩放。正确行为是**原样保留**。
    #
    # 旧形态是空洞的：tile 4×6、body x=0.1（0.1×40=4 恰好落在 tile 右缘外）⇒ 源图在
    # resvg 下 ink=0，三张图全空，diff 0 什么也证明不了（R39 的 blocker）。现在 tile
    # 与引用矩形同尺寸（40×30），body 完整落在 tile 内；且判据同时跑 s=1 与 s=2——
    # 若 body 被缩放，0.1 会变成 0.200（s=2）或 0.050（s=0.5），两次读数不可能同时原样。
    src2 = write_svg("a2_obb.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox" '
        'x="10" y="10" width="40" height="30">'
        '<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>'))
    a2_outputs = {}
    for w, h, tag in ((100, 80, "s1"), (200, 160, "s2")):
        name = f"a2_obb_grid_{tag}.svg"
        svg2 = compose(src2, name, "--width", str(w), "--height", str(h), "--ncol", "1",
                       "--plot-margin", "0")
        a2_outputs[tag] = TMP / name
        check('<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>' in svg2,
              f"A2 obb 内容 @{tag}：分数字面量逐字保留（不被 (sx,sy) 二次缩放）",
              svg2[:900] if len(svg2) < 200000 else "")
    a2_tile = 'x="10" y="10" width="40" height="30"' in a2_outputs["s1"].read_text()
    a2_tile = a2_tile and 'x="20" y="20" width="80" height="60"' in a2_outputs["s2"].read_text()
    check(a2_tile, "A2b obb tile（userSpaceOnUse）仍随 s 平移缩放", "")

    # A3 非对称 body + s≠1：每个数字都按 s 缩放，x/y 保持 tile 局部。
    src3 = write_svg("a3_asym.svg", panel(120, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="q" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        '<rect x="7" y="3" width="6" height="4" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="100" height="60" fill="url(#q)"/></g></svg>'))
    svg3 = compose(src3, "a3_asym_grid.svg", "--width", "148", "--ncol", "1", "--plot-margin", "24")
    check('<rect x="5.833" y="2.500" width="5" height="3.333" fill="#000000"/>' in svg3,
          "A3 非对称 body：缩放 (7,3,6,4)×0.8333 且不平移", svg3[:900])

    # A4 s=1：body 逐字节原样。
    src4 = write_svg("a4_identity.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="r" patternUnits="userSpaceOnUse" x="10" y="10" width="4" height="6">'
        '<rect x="1" y="2" width="2" height="3" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#r)"/></g></svg>'))
    svg4 = compose(src4, "a4_identity_grid.svg", "--width", "100", "--height", "80", "--ncol", "1",
            "--plot-margin", "0")
    check('<rect x="1" y="2" width="2" height="3" fill="#000000"/>' in svg4,
          "A4 s=1：pattern body 原样（不缩放不平移）", svg4[:900])

    # A5 **R55 修正**：`patternTransform` 写在面板的 user space 里，必须跟着面板
    # 比例共轭（`T' = S·T·S⁻¹`），等向下 translate 乘 s、rotate 角度不变。R38 原来
    # 这条断言「原样保留（要写就必须累积）」是错的：R55 用 d1render 逐像素实证
    # `translate(6 8)` 才 xor 0，而嵌套 pattern 各自缩、不累积。见
    # `docs/PITFALLS/round-data/R55/`。
    src5 = write_svg("a5_transform.svg", panel(100, 80) + (
        '<g data-scale="xy"><defs>'
        '<pattern id="t" patternUnits="userSpaceOnUse" patternTransform="translate(3 4) rotate(30)" '
        'x="10" y="10" width="4" height="6">'
        '<rect x="1" y="2" width="2" height="3" fill="#000000"/></pattern></defs>'
        '<rect x="10" y="10" width="40" height="30" fill="url(#t)"/></g></svg>'))
    svg5 = compose(src5, "a5_transform_grid.svg", "--width", "200", "--height", "160", "--ncol", "1",
            "--plot-margin", "0")
    check('patternTransform="translate(6, 8) rotate(30)"' in svg5,
          "A5 既有 patternTransform 按面板比例共轭（translate 乘 2、rotate 不变）", svg5[:900])

    # 渲染对照：源图按 s 缩放后贴到面板原点，应与产物逐像素相同。
    if not RENDER.exists():
        skip_render("A6 pattern 相位渲染对照（resvg 0 / cairosvg ink）", f"渲染器不存在：{RENDER}")
        return
    cases = [("A1", src, out, 100 / 120, 24, 24),
             ("A2s1", src2, a2_outputs["s1"], 1.0, 0, 0),
             ("A2s2", src2, a2_outputs["s2"], 2.0, 0, 0),
             ("A3", src3, TMP / "a3_asym_grid.svg", 100 / 120, 24, 24)]
    # `ink == 0` 是 R38 自己的空洞：三张全空同样 xor 0。渲染对照必须同时要求
    # 源图与产物**真的有墨**，否则「diff 0」只证明了两块白布相同。
    for tag, s, o, scale, dx, dy in cases:
        src_img = render(s, f"{tag}_src_resvg", scale)
        out_img = render(o, f"{tag}_out_resvg", 1.0)
        pasted = paste_at(src_img, dx, dy, (out_img[0], out_img[1]))
        xor, md = diff(pasted, out_img)
        check(xor == 0 and md == 0 and ink(src_img) > 0 and ink(out_img) > 0,
              f"A6{tag} resvg：源图×s 贴到面板原点 ⇒ diff 0（且非空洞）",
              f"xor={xor} maxdiff={md} ink_src={ink(src_img)}/ink_out={ink(out_img)}")

    if CAIRO_PYTHON.exists() and subprocess.run(
            [str(CAIRO_PYTHON), "-c", "import cairosvg"], capture_output=True).returncode == 0:
        # cairosvg 的 hatch 相位与 resvg 有系统差（源图就存在），所以这里不判逐像素，
        # 只判「hatch 没有整块消失」（ink 同量级）。objectBoundingBox 内容两渲染器的
        # **相位**语义本身不一致，所以 obb 的逐像素判据只认 resvg（A6）；cairo 只判
        # ink 量级，不再用 SKIP 掩盖（R39）。
        for tag, s, o, scale, dx, dy in cases:
            out_img = render(o, f"{tag}_out_cairo", 1.0, cairo=True)
            if tag.startswith("A2"):
                # cairosvg 没有实现 `patternContentUnits="objectBoundingBox"`：它把
                # 分数字面量当 user unit（实测 body 只有 0.5×0.25 用户单位 ⇒ 亚像素
                # 点，ink 恒为 1），相位与像素都不可判。obb 的逐像素判据以 resvg
                # 为准（A6A2s1/A6A2s2）；cairo 侧只判 ink 量级——源图按 1:1 渲染
                # （与产物的 user unit 对齐）后必须与产物同量级、且两边都有墨。
                src_img = render(s, f"{tag}_src_cairo1", 1.0, cairo=True)
                si, oi = ink(src_img), ink(out_img)
                check(si > 0 and oi > 0 and oi <= 4 * si and si <= 4 * oi,
                      f"A7{tag} cairosvg：obb 只判 ink 量级（亚像素点，判定以 resvg 为准）",
                      f"ink_src_1to1={si} ink_out={oi}")
                continue
            src_img = render(s, f"{tag}_src_cairo", scale, cairo=True)
            pasted = paste_at(src_img, dx, dy, (out_img[0], out_img[1]))
            si = ink(pasted)
            oi = ink(out_img)
            check(si > 0 and oi > 0 and 0.3 * si <= oi <= 3.0 * si,
                  f"A7{tag} cairosvg：hatch 未整块消失（ink 同量级，相位差已知）",
                  f"ink_src={si} ink_out={oi}")
    else:
        skip("A7 cairosvg 渲染对照", f"没有可用的 cairosvg 解释器：{CAIRO_PYTHON}")


# ── B. `<defs>` 递归下钻 ───────────────────────────────────────────────────────────
NESTED = {
    "d1_flat": "<defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
               "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></defs>",
    "d2_g": "<defs><g><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
            "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></g></defs>",
    "d3_g_id": "<defs><g id=\"grp\"><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
               "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></g></defs>",
    "d4_g_defs": "<g><defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
                 "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></defs></g>",
    "d5_defs_defs": "<defs><defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
                    "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></defs></defs>",
    "d6_symbol": "<defs><symbol id=\"s\"><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#f00\"/>"
                 "<stop offset=\"1\" stop-color=\"#00f\"/></linearGradient></symbol></defs>",
    "d7_g_mask": "<defs><g><mask id=\"m\"><rect x=\"0\" y=\"0\" width=\"100\" height=\"100\" fill=\"#fff\"/>"
                 "</mask></g></defs>",
}


def family_b():
    if not CONV.exists():
        check(False, "B0 转换器存在", f"缺 {CONV}（先 cargo build --offline）")
        return
    render_ok = RENDER.exists()
    if not render_ok:
        skip_render("B1 defs 嵌套渲染对照（resvg xor 0 / cairosvg ink）", f"渲染器不存在：{RENDER}")
    for name, defs in NESTED.items():
        ref = 'mask="url(#m)"' if name == "d7_g_mask" else 'fill="url(#g)"'
        src = write_svg(f"b_{name}.svg", bare(100, 100) + defs +
                        f'<rect x="10" y="10" width="80" height="80" {ref}/></svg>')
        out_path = TMP / f"b_{name}_out.svg"
        p = subprocess.run([str(CONV), *CONVERT_ARGS, "--input", str(src), "--output", str(out_path)],
                            capture_output=True, text=True)
        out = out_path.read_text() if out_path.exists() else ""
        want = "<mask" if name == "d7_g_mask" else '<linearGradient id="g"'
        check(p.returncode == 0 and want in out and "<symbol" not in out,
              f"B1 {name}：rc=0 且定义仍在产物里",
              f"rc={p.returncode} stderr={p.stderr[-300:]} out={out[:600]}")
        if not render_ok or p.returncode != 0:
            continue
        xor, md = diff(render(src, f"b_{name}_src", 1.0), render(out_path, f"b_{name}_out", 1.0))
        check(xor == 0 and md == 0, f"B2 {name} resvg：源图 vs 产物 diff 0", f"xor={xor} maxdiff={md}")
        if CAIRO_PYTHON.exists():
            si = ink(render(src, f"b_{name}_src_c", 1.0, cairo=True))
            oi = ink(render(out_path, f"b_{name}_out_c", 1.0, cairo=True))
            check(si == oi and si > 0, f"B3 {name} cairosvg：ink 相同", f"ink_src={si} ink_out={oi}")


# ── C. 真实语料：hatch 不再整块消失 ───────────────────────────────────────────────
def family_c():
    bars = SVG_GRID / "tests/fixtures/matplotlib/bars_grouped.svg"
    if not bars.exists():
        skip("C1 bars_grouped 语料渲染对照", f"缺 {bars}")
        return
    if not RENDER.exists():
        skip_render("C1 bars_grouped 语料渲染对照（resvg maxdiff ≤ 8）", f"渲染器不存在：{RENDER}")
        return
    out = TMP / "c1_bars_grid.svg"
    run([BIN, "plot-grid", "--input", bars, "--output", out, "--width", "288", "--ncol", "1",
         "--plot-margin", "24"])
    scale = 240 / 288.0
    src_img = render(bars, "c1_bars_src", scale)
    out_img = render(out, "c1_bars_out", 1.0)
    xor, md = diff(paste_at(src_img, 24, 24, (out_img[0], out_img[1])), out_img)
    check(md <= 8 and abs(ink(src_img) - ink(out_img)) <= max(8, ink(src_img) // 100),
          "C1 bars_grouped：resvg 源图×s 贴 (24,24) ⇒ maxdiff ≤ 8 且 ink 相同",
          f"xor={xor} maxdiff={md} ink_src={ink(src_img)} ink_out={ink(out_img)}")

    s36 = pathlib.Path("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/09_上游转录因子/出图/"
                       "S36_S37/figure_panels/S36_panelA_matrix.svg")
    if not s36.exists():
        skip("C2 S36 真实语料渲染对照", "research/ 语料不在本仓（monorepo）")
        return
    out2 = TMP / "c2_s36_grid.svg"
    run([BIN, "plot-grid", "--input", s36, "--output", out2, "--width", "288", "--ncol", "1",
         "--plot-margin", "24"])
    scale2 = 240 / (533.76 * 4 / 3)
    a = render(s36, "c2_s36_src", scale2)
    b = render(out2, "c2_s36_out", 1.0)
    xor2, md2 = diff(paste_at(a, 24, 24, (b[0], b[1])), b)
    check(md2 <= 8 and abs(ink(a) - ink(b)) <= max(8, ink(a) // 100),
          "C2 S36：resvg 源图×s 贴 (24,24) ⇒ maxdiff ≤ 8 且 ink 相同",
          f"xor={xor2} maxdiff={md2} ink_src={ink(a)} ink_out={ink(b)}")


# ── D. R37 证据脚本 fail-closed ────────────────────────────────────────────────────
def family_d():
    r37 = SVG_GRID / "docs/PITFALLS/round-data/R37/test.py"
    if not r37.exists():
        check(False, "D1 R37 证据脚本存在", str(r37))
        return
    env = dict(os.environ, SVG_GRID_RENDER_BIN="/nonexistent/d1render")
    p = subprocess.run([sys.executable, str(r37)], capture_output=True, text=True, env=env,
                       timeout=1800)
    check(p.returncode != 0,
          "D1 渲染器缺失 ⇒ R37 证据 rc≠0（不再 skip-to-green）",
          f"rc={p.returncode}\n{p.stdout[-500:]}")
    env2 = dict(os.environ, SVG_GRID_BIN="/nonexistent/svg_grid")
    p2 = subprocess.run([sys.executable, str(r37)], capture_output=True, text=True, env=env2,
                        timeout=600)
    check(p2.returncode != 0, "D2 SVG_GRID_BIN 被采纳（指向不存在的二进制 ⇒ rc≠0）",
          f"rc={p2.returncode}\n{p2.stderr[-300:]}")


def main():
    print(f"== R38 证据（bin={BIN}） ==")
    if not BIN.exists():
        check(False, "B0 svg_grid 二进制存在", f"缺 {BIN}（先 cargo build --offline）")
        return finish()
    family_a()
    family_b()
    family_c()
    family_d()
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
