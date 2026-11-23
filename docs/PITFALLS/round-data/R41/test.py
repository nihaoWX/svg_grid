#!/usr/bin/env python3
"""R41 的可跑证据：`href` 链决定「缩放多少」——断链回退默认、重复 id 后者胜、`patternUnits` 继承。

R40 引入两个方向写反的语义分支：

1. 链解析不出**本文件内元素**（`#missing`、`other.svg#q`、空串、裸 URL）时返回 Verbatim，
   注释称「不加缩放最安全」。usvg 的 `HrefIter` 只是停止继承，`convert_units` 于是落到
   默认 `userSpaceOnUse` ⇒ body **必须**缩放（实测 s=2：xor 448、ink 96 vs 参考 384）。
2. 重复 id 取「首个」，而 usvg 的 id map 是 `parser/svgtree/parse.rs` 里
   `links.insert(id, node.id)` 顺序覆盖 ⇒ **最后**定义胜。

同一机制上还缺 `patternUnits`：它也沿 `href` 继承（`resolve_pattern_attr`），默认
`objectBoundingBox`；未继承时 tile 的分数被当成 user 单位重写，三版实测 xor 均 896。

旁证（usvg 0.47）：`parse.rs:117-121` 建 id map；`svgtree/mod.rs:619-648` `HrefIter`
解析不出本文件元素就 warn + 结束；`paint_server.rs:230-233` `convert_units` =
`attribute(...).unwrap_or(default)`；`:498-518` `resolve_pattern_attr` 沿链取第一个显式值、
遇到非 `<pattern>` 即停。

契约（`pitfalls-manager/references/02_consume.md` §4.3）：无参只跑快断言；退出码即结论；
每族一行 PASS/FAIL；只有 FAIL 才给细节；跳过必须打出来并给原因；幂等、离线、不写仓库。
渲染对照缺失即 FAIL，确实要跳过请显式传 `--allow-skip`。

二进制定位（可用环境变量覆盖）：
  SVG_GRID_BIN          默认 <repo>/target/debug/svg_grid
  SVG_GRID_RENDER_BIN   默认 /tmp/d1_eval/render/target/release/d1render
  SVG_GRID_OLD_BIN      可选：R40 的二进制（715ce48）；给了就必须 **红**（fail-closed 段）
"""
import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R41 → PITFALLS → docs → svg_grid
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN", SVG_GRID / "target/debug/svg_grid"))
RENDER = pathlib.Path(os.environ.get("SVG_GRID_RENDER_BIN", "/tmp/d1_eval/render/target/release/d1render"))
CAIRO_PYTHON = pathlib.Path(os.environ.get("SVG_GRID_CAIRO_PYTHON", str(pathlib.Path.home() / ".venv/bin/python")))
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None
ALLOW_SKIP = "--allow-skip" in sys.argv

TMP = pathlib.Path(tempfile.mkdtemp(prefix="r41ev-"))
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


# ── A. B-1：链解析不出本文件元素 ⇒ HrefIter 停止继承 ⇒ 默认 userSpaceOnUse，body 缩放 ──
BROKEN_LINKS = ["#missing", "other.svg#q", "", "http://example.com/#p"]


def render_matches(src, out_name, label, tag):
    if not RENDER.exists():
        skip_render(label, f"渲染器不存在：{RENDER}")
        return
    src_img = render(src, f"{tag}_src", 2.0)
    out_img = render(TMP / out_name, f"{tag}_out", 1.0)
    xor, md = diff(src_img, out_img)
    si, oi = ink(src_img), ink(out_img)
    check(xor == 0 and md == 0 and si > 0 and oi > 0,
          label, f"xor={xor} maxdiff={md} ink_src={si} ink_out={oi}")


def family_a():
    for link in BROKEN_LINKS:
        tag = f"a_link_{abs(hash(link))}"
        doc = write_svg(f"{tag}.svg", pattern_doc(
            f'<pattern id="p" href="{link}" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
            + USER_BODY + '</pattern>'))
        out = compose_doubled(doc, f"{tag}_out.svg")
        check(SCALED_BODY in out,
              f"A1 不可解析 href {link!r} ⇒ 回退 userSpaceOnUse，body ×2（不再原样）", out[:900])
        render_matches(doc, f"{tag}_out.svg",
                       f"A1 不可解析 href {link!r} ⇒ resvg 源图×2 vs 产物 diff 0", tag)


# ── B. B-2：重复 id 由 usvg 的 id map 顺序覆盖 ⇒ 最后定义胜 ───────────────────────────
def family_b():
    # 后者是 userSpaceOnUse ⇒ 继承方按 user 单位缩放。
    doc = write_svg("b_dup_user.svg", pattern_doc(
        '<pattern id="q" patternContentUnits="objectBoundingBox"/>'
        '<pattern id="q" patternContentUnits="userSpaceOnUse"/>'
        '<pattern id="p" href="#q" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + USER_BODY + '</pattern>'))
    out = compose_doubled(doc, "b_dup_user_out.svg")
    check(SCALED_BODY in out, "B1 重复 id 后者胜（后一个声明的 user 单位 ⇒ body 缩放）", out[:900])
    render_matches(doc, "b_dup_user_out.svg", "B1 重复 id ⇒ resvg 源图×2 vs 产物 diff 0", "b_dup_user")

    # 反过来：后者声明 bbox ⇒ 继承方按 bbox 单位，body 分数原样。
    doc = write_svg("b_dup_obb.svg", pattern_doc(
        '<pattern id="q" patternContentUnits="userSpaceOnUse"/>'
        '<pattern id="q" patternContentUnits="objectBoundingBox"/>'
        '<pattern id="p" href="#q" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + OBB_BODY + '</pattern>'))
    out = compose_doubled(doc, "b_dup_obb_out.svg")
    check(OBB_BODY in out, "B2 重复 id 的另一个方向（后者 bbox ⇒ body 分数原样）", out[:900])
    render_matches(doc, "b_dup_obb_out.svg", "B2 重复 id（bbox）⇒ resvg 源图×2 vs 产物 diff 0", "b_dup_obb")


# ── C. `patternUnits` 沿 href 继承，且按类型化解析（只认严格 token） ──────────────────
def family_c():
    doc = write_svg("c_units_user.svg", pattern_doc(
        '<pattern id="base" patternUnits="userSpaceOnUse"/>'
        '<pattern id="p" href="#base" x="12" y="14" width="16" height="18">'
        + USER_BODY + '</pattern>'))
    out = compose_doubled(doc, "c_units_user_out.svg")
    check('x="24" y="28" width="32" height="36"' in out,
          "C1 patternUnits 经 href 继承为 userSpaceOnUse ⇒ tile 随面板缩放", out[:900])
    check(SCALED_BODY in out, "C1b 继承来的 user 单位同样让 body 缩放", out[:900])
    render_matches(doc, "c_units_user_out.svg", "C1 patternUnits 继承 ⇒ resvg 源图×2 vs 产物 diff 0", "c_units_user")

    doc = write_svg("c_units_obb.svg", pattern_doc(
        '<pattern id="base" patternUnits="objectBoundingBox"/>'
        '<pattern id="p" href="#base" x="0.1" y="0.2" width="0.5" height="0.5">'
        + USER_BODY + '</pattern>'))
    out = compose_doubled(doc, "c_units_obb_out.svg")
    check('x="0.1" y="0.2" width="0.5" height="0.5"' in out,
          "C2 patternUnits 继承为 objectBoundingBox ⇒ tile 分数原样（默认值）", out[:900])
    render_matches(doc, "c_units_obb_out.svg", "C2 patternUnits（bbox）⇒ resvg 源图×2 vs 产物 diff 0", "c_units_obb")

    # 类型化解析：大小写不符的 `UserSpaceOnUse` 解析失败 ⇒ 回退默认 bbox ⇒ 分数原样。
    doc = write_svg("c_units_loose.svg", pattern_doc(
        '<pattern id="p" patternUnits="UserSpaceOnUse" x="0.1" y="0.2" width="0.5" height="0.5">'
        + USER_BODY + '</pattern>'))
    out = compose_doubled(doc, "c_units_loose_out.svg")
    check('x="0.1" y="0.2" width="0.5" height="0.5"' in out,
          "C3 patternUnits 只认严格 token：'UserSpaceOnUse' 回退默认 bbox，分数原样", out[:900])
    render_matches(doc, "c_units_loose_out.svg", "C3 patternUnits 非严格 token ⇒ resvg 源图×2 vs 产物 diff 0",
                   "c_units_loose")


# ── D. R40 语义不回退：严格 bbox 逐字保留、五种非法值仍缩放 ───────────────────────────
def family_d():
    src = write_svg("d_obb.svg", pattern_doc(
        '<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="objectBoundingBox"'
        ' x="10" y="10" width="20" height="20">' + OBB_BODY + '</pattern>'))
    out = compose_doubled(src, "d_obb_out.svg")
    check(OBB_BODY in out, "D1 严格 objectBoundingBox 仍逐字保留（R39/R40 修复不回退）", out[:900])
    for value in ["ObjectBoundingBox", "objectBoundingBox ", " objectBoundingBox", "foo", ""]:
        src = write_svg(f"d_inv_{abs(hash(value))}.svg", pattern_doc(
            f'<pattern id="p" patternUnits="userSpaceOnUse" patternContentUnits="{value}"'
            ' x="10" y="10" width="20" height="20">' + USER_BODY + '</pattern>'))
        out = compose_doubled(src, f"d_inv_{abs(hash(value))}_out.svg")
        check(SCALED_BODY in out, f"D2 patternContentUnits={value!r} 仍回退 user 单位并缩放", out[:900])


# ── E. fail-closed：R40 的二进制（715ce48）必须红 ───────────────────────────────────
def family_e():
    if OLD_BIN is None:
        skip("E1 R40 二进制必须红", "设 SVG_GRID_OLD_BIN=<715ce48 的 svg_grid> 启用（fail-closed 段）")
        return
    if not OLD_BIN.exists():
        check(False, "E1 R40 二进制必须红", f"SVG_GRID_OLD_BIN 不存在：{OLD_BIN}")
        return
    red = []
    for link in BROKEN_LINKS:
        doc = write_svg(f"e_link_{abs(hash(link))}.svg", pattern_doc(
            f'<pattern id="p" href="{link}" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
            + USER_BODY + '</pattern>'))
        out = compose(OLD_BIN, doc, f"e_link_{abs(hash(link))}_out.svg", "--width", "200",
                      "--height", "160", "--ncol", "1", "--plot-margin", "0")
        if SCALED_BODY not in out:
            red.append(f"不可解析 href {link!r}")
    doc = write_svg("e_dup.svg", pattern_doc(
        '<pattern id="q" patternContentUnits="objectBoundingBox"/>'
        '<pattern id="q" patternContentUnits="userSpaceOnUse"/>'
        '<pattern id="p" href="#q" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20">'
        + USER_BODY + '</pattern>'))
    out = compose(OLD_BIN, doc, "e_dup_out.svg", "--width", "200", "--height", "160",
                  "--ncol", "1", "--plot-margin", "0")
    if SCALED_BODY not in out:
        red.append("重复 id 后者胜")
    doc = write_svg("e_units.svg", pattern_doc(
        '<pattern id="base" patternUnits="userSpaceOnUse"/>'
        '<pattern id="p" href="#base" x="12" y="14" width="16" height="18">' + USER_BODY + '</pattern>'))
    out = compose(OLD_BIN, doc, "e_units_out.svg", "--width", "200", "--height", "160",
                  "--ncol", "1", "--plot-margin", "0")
    if 'x="24" y="28" width="32" height="36"' not in out:
        red.append("patternUnits 继承")
    check(len(red) >= 3, "E1 R40 二进制在不可解析 href / 重复 id / patternUnits 继承上必须红（断言有牙）",
          f"只红了 {len(red)} 项：{red}")


def main():
    print(f"== R41 证据（bin={BIN}） ==")
    if not BIN.exists():
        check(False, "A0 svg_grid 二进制存在", f"缺 {BIN}（先 cargo build --offline）")
        return finish()
    try:
        family_a()
        family_b()
        family_c()
    except RuntimeError as exc:
        check(False, "A0 组合失败", str(exc))
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
