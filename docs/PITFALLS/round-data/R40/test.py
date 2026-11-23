#!/usr/bin/env python3
"""R40 的可跑证据：`patternContentUnits` 只认严格 token，`viewBox` 内容原样，二者沿 href 继承。

R39 把「任何显式非 userSpaceOnUse 的值」都当成 `objectBoundingBox`，于是
`ObjectBoundingBox` / 带空白 / 未知值 / 空串 都让 body 原样不动，而 resvg 按类型化解析
把它们全部回退到 `userSpaceOnUse` 并缩放 body（s=2 实测 xor 448）。同一个错误还有第二面：
`<pattern viewBox="…">` 的内容坐标在 **viewBox 单位**里，viewBox→tile 的映射已经随 s 跟上，
再乘 (sx,sy) 就是双重缩放（s=2 实测 xor 1792，body `2 2 6 6` 被写成 `4 4 12 12`）。
usvg 0.47 的权威实现：`convert_units` = `attribute(...).unwrap_or(default)`（区分大小写、
不 trim），`resolve_pattern_attr` 沿 `href` 取第一个显式值，`viewBox` 解析成功即
`push_pattern_transform` 且内容单位强制为 user space。

契约（`pitfalls-manager/references/02_consume.md` §4.3）：无参只跑快断言；退出码即结论
（0＝全过）；每族一行 PASS/FAIL；只有 FAIL 才给细节；跳过必须打出来并给原因；幂等、离线、
不写仓库。渲染对照缺失即 FAIL，确实要跳过请显式传 `--allow-skip`。

二进制定位（可用环境变量覆盖）：
  SVG_GRID_BIN          默认 <repo>/target/debug/svg_grid
  SVG_GRID_RENDER_BIN   默认 /tmp/d1_eval/render/target/release/d1render
  SVG_GRID_CAIRO_PYTHON 默认 ~/.venv/bin/python（能 import cairosvg 才启用第二渲染器）
  SVG_GRID_OLD_BIN      可选：上一轮（R39）的二进制；给了就必须 **红**（fail-closed 段）
"""
import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R40 → PITFALLS → docs → svg_grid
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN", SVG_GRID / "target/debug/svg_grid"))
RENDER = pathlib.Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO_PYTHON = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(pathlib.Path.home() / ".venv/bin/python")))
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None
ALLOW_SKIP = "--allow-skip" in sys.argv

TMP = pathlib.Path(tempfile.mkdtemp(prefix="r40ev-"))
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
    if len(img) == 3:
        return img
    w, h, _bpp, rows = img
    return w, h, [[pixel(img, x, y) for x in range(w)] for y in range(h)]


def ink(img):
    w, h, rows = to_rows(img)
    return sum(1 for y in range(h) for x in range(w) if rows[y][x] != (255, 255, 255))


def render(svg, tag, scale=1.0):
    png = TMP / f"{tag}.png"
    run([RENDER, svg, png, scale])
    return read_png(png)


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


def compose(bin_path, src, out_name, *extra):
    run([bin_path, "plot-grid", "--input", src, "--output", TMP / out_name, *extra])
    return (TMP / out_name).read_text()


def body_of(svg, pattern_id):
    start = svg.find(f'<pattern id="{pattern_id}"')
    assert start >= 0, f"pattern {pattern_id} missing"
    end = start + svg[start:].find("</pattern>")
    markup = svg[start:end]
    body_start = markup.find("<rect x=")
    body_end = body_start + markup[body_start:].find("/>") + 2
    return markup[body_start:body_end]


USER_BODY = '<rect x="2" y="3" width="6" height="4" fill="#000000"/>'
OBB_BODY = '<rect x="0.1" y="0.2" width="0.5" height="0.25" fill="#000000"/>'
SCALED_BODY = '<rect x="4" y="6" width="12" height="8" fill="#000000"/>'


def pattern_doc(defs, use_id="p"):
    return panel(100, 80) + (
        '<g data-scale="xy"><defs>' + defs + '</defs>'
        f'<rect x="10" y="10" width="40" height="30" fill="url(#{use_id})"/></g></svg>')


def compose_doubled(src, out_name):
    return compose(BIN, src, out_name, "--width", "200", "--height", "160", "--ncol", "1",
                   "--plot-margin", "0")


def tile_units(attrs):
    return (f'<pattern id="p" patternUnits="userSpaceOnUse" {attrs}'
            ' x="10" y="10" width="20" height="20">')


# ── A. `patternContentUnits` 的类型化读取：非严格 token 一律回退 userSpaceOnUse ────────
INVALID_UNITS = ["ObjectBoundingBox", "objectBoundingBox ", " objectBoundingBox", "foo", ""]


def family_a():
    for value in INVALID_UNITS:
        src = write_svg(f"a_inv_{abs(hash(value))}.svg",
                        pattern_doc(tile_units(f'patternContentUnits="{value}"') + USER_BODY + '</pattern>'))
        svg = compose_doubled(src, f"a_inv_{abs(hash(value))}_out.svg")
        check(SCALED_BODY in svg,
              f"A1 patternContentUnits={value!r} 不是 bbox token ⇒ body 按 user 单位缩放",
              svg[:900])
    # 严格 token 必须仍然原样（R39 的成果不回退）。
    src = write_svg("a_obb.svg", pattern_doc(
        tile_units('patternContentUnits="objectBoundingBox"') + OBB_BODY + '</pattern>'))
    svg = compose_doubled(src, "a_obb_out.svg")
    check(OBB_BODY in svg and 'x="0.200"' not in svg,
          "A2 严格 objectBoundingBox 仍逐字保留（0.1 不变成 0.2）", svg[:900])
    return (src, "a_obb_out.svg")


def family_b(obb_case):
    """viewBox 内容原样；不合法的 viewBox 等于没有 viewBox。"""
    src = write_svg("b_vb.svg", pattern_doc(
        tile_units('viewBox="0 0 10 10"') + USER_BODY + '</pattern>'))
    svg = compose_doubled(src, "b_vb_out.svg")
    check(USER_BODY in svg,
          "B1 viewBox 内容在 viewBox 单位里 ⇒ 逐字保留（2 3 6 4 不变成 4 6 12 8）", svg[:900])
    for value in ("0 0 20 0", "0 0 20", "0 0 20 20e", "a b c d"):
        bad = write_svg(f"b_bad_{abs(hash(value))}.svg", pattern_doc(
            tile_units(f'viewBox="{value}"') + USER_BODY + '</pattern>'))
        out = compose_doubled(bad, f"b_bad_{abs(hash(value))}_out.svg")
        check(SCALED_BODY in out,
              f"B2 viewBox={value!r} 解析失败 ⇒ 视同没有 viewBox，body 仍缩放", out[:900])

    if not RENDER.exists():
        for label in ("B3 viewBox 内容 resvg 逐像素对照（源图×2 vs 产物）",
                      "B4 obb 内容 resvg 逐像素对照（源图×2 vs 产物）"):
            skip_render(label, f"渲染器不存在：{RENDER}")
        return
    for tag, (case_src, out_name) in {
        "B3 viewBox": (src, "b_vb_out.svg"),
        "B4 obb": obb_case,
    }.items():
        src_img = render(case_src, f"{tag.split()[1]}_src", 2.0)
        out_img = render(TMP / out_name, f"{tag.split()[1]}_out", 1.0)
        xor, md = diff(src_img, out_img)
        si, oi = ink(src_img), ink(out_img)
        check(xor == 0 and md == 0 and si > 0 and oi > 0,
              f"{tag} resvg：源图×2 vs 产物 ⇒ diff 0（且两边真有墨）",
              f"xor={xor} maxdiff={md} ink_src={si} ink_out={oi}")


# ── C. href 继承：patternContentUnits / viewBox 沿链取第一个显式值 ─────────────────────
def family_c():
    src = write_svg("c_href_obb.svg", pattern_doc(
        '<pattern id="base" patternContentUnits="objectBoundingBox"/>'
        '<pattern id="p" href="#base" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + OBB_BODY + '</pattern>'))
    svg = compose_doubled(src, "c_href_obb_out.svg")
    check(OBB_BODY in svg,
          "C1 objectBoundingBox 经 href 继承 ⇒ body 原样（不被当成 user 单位缩放）", svg[:900])

    for link in ("href", "xlink:href"):
        tag = link.replace(":", "_")
        doc = write_svg(f"c_href_vb_{tag}.svg", pattern_doc(
            '<pattern id="base" viewBox="0 0 10 10"/>'
            f'<pattern id="p" {link}="#base" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
            + USER_BODY + '</pattern>'))
        out = compose_doubled(doc, f"c_href_vb_{tag}_out.svg")
        check(USER_BODY in out,
              f"C2 viewBox 经 {link} 继承 ⇒ body 原样", out[:900])

    # 环：必须停机，且不 panic（两个 pattern 都没有单位/ viewBox ⇒ user space 缩放）。
    doc = write_svg("c_cycle.svg", pattern_doc(
        '<pattern id="a" href="#b"/>'
        '<pattern id="p" href="#a" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + USER_BODY + '</pattern>'
        '<pattern id="b" href="#a"/>', use_id="p"))
    try:
        out = compose_doubled(doc, "c_cycle_out.svg")
        check(SCALED_BODY in out, "C3 href 环：停机且按 user 单位处理（不 panic）", out[:900])
    except RuntimeError as exc:
        check(False, "C3 href 环：停机且按 user 单位处理（不 panic）", str(exc))

    # 悬空 id / 外部链接 / 非 pattern 目标：fail-safe 与原样保留的取舍。
    for link, want, label in (
        ("#missing", USER_BODY, "C4 悬空 href ⇒ fail-safe 原样（不额外加缩放）"),
        ("other.svg#p", USER_BODY, "C5 外部 href ⇒ fail-safe 原样"),
        ("#notpattern", SCALED_BODY, "C6 链指向非 pattern ⇒ 按 usvg 截断，body 仍缩放"),
    ):
        doc = write_svg(f"c_link_{abs(hash(link))}.svg", pattern_doc(
            '<rect id="notpattern" x="0" y="0" width="1" height="1"/>'
            f'<pattern id="p" href="{link}" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
            + USER_BODY + '</pattern>'))
        out = compose_doubled(doc, f"c_link_{abs(hash(link))}_out.svg")
        check(want in out, f"{label}（{link!r}）", out[:900])


# ── D. userSpaceOnUse 内容不回退（R38 的修法原样保留） ────────────────────────────────
def family_d():
    src = write_svg("d_user.svg", pattern_doc(
        tile_units("") + '<rect x="7" y="3" width="6" height="4" fill="#000000"/></pattern>'))
    svg = compose_doubled(src, "d_user_out.svg")
    check('<rect x="14" y="6" width="12" height="8" fill="#000000"/>' in svg,
          "D1 userSpaceOnUse 内容：每个数字 ×2 且不平移（R38/R39 行为不回退）", svg[:900])
    if not RENDER.exists():
        skip_render("D2 userSpaceOnUse resvg 逐像素对照", f"渲染器不存在：{RENDER}")
        return
    src_img = render(src, "d_user_src", 2.0)
    out_img = render(TMP / "d_user_out.svg", "d_user_out", 1.0)
    xor, md = diff(src_img, out_img)
    si, oi = ink(src_img), ink(out_img)
    check(xor == 0 and md == 0 and si > 0 and oi > 0,
          "D2 userSpaceOnUse @s=2 resvg：源图×2 vs 产物 ⇒ diff 0（两边真有墨）",
          f"xor={xor} maxdiff={md} ink_src={si} ink_out={oi}")


# ── E. fail-closed：上一轮的二进制必须红（证明断言真的有牙） ──────────────────────────
def family_e():
    if OLD_BIN is None:
        skip("E1 上一轮二进制必须红", "设 SVG_GRID_OLD_BIN=<R39 二进制> 启用（fail-closed 段）")
        return
    if not OLD_BIN.exists():
        check(False, "E1 上一轮二进制必须红", f"SVG_GRID_OLD_BIN 不存在：{OLD_BIN}")
        return
    red = []
    for value in INVALID_UNITS:
        src = write_svg(f"e_inv_{abs(hash(value))}.svg",
                        pattern_doc(tile_units(f'patternContentUnits="{value}"') + USER_BODY + '</pattern>'))
        svg = compose(OLD_BIN, src, f"e_inv_{abs(hash(value))}_out.svg", "--width", "200",
                      "--height", "160", "--ncol", "1", "--plot-margin", "0")
        if SCALED_BODY not in svg:
            red.append(f"patternContentUnits={value!r}")
    src = write_svg("e_vb.svg", pattern_doc(tile_units('viewBox="0 0 10 10"') + USER_BODY + '</pattern>'))
    svg = compose(OLD_BIN, src, "e_vb_out.svg", "--width", "200", "--height", "160",
                  "--ncol", "1", "--plot-margin", "0")
    if USER_BODY not in svg:
        red.append("viewBox 内容原样")
    check(len(red) >= 6, "E1 上一轮二进制在 B1 五种非法值 + viewBox 上全部为红（断言有牙）",
          f"只红了 {len(red)} 项：{red}")


def main():
    print(f"== R40 证据（bin={BIN}） ==")
    if not BIN.exists():
        check(False, "A0 svg_grid 二进制存在", f"缺 {BIN}（先 cargo build --offline）")
        return finish()
    try:
        obb_case = family_a()
    except RuntimeError as exc:
        check(False, "A0 组合 pattern 失败", str(exc))
        obb_case = (write_svg("a_obb.svg", pattern_doc(
            tile_units('patternContentUnits="objectBoundingBox"') + OBB_BODY + '</pattern>')),
            "a_obb_out.svg")
    family_b(obb_case)
    family_c()
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
