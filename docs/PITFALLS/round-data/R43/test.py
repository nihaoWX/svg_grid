#!/usr/bin/env python3
"""R43 证据：单位属性的类型化解析、marker/mask 的内容域、pattern tile 的继承。

`*Units` 不是"带空白的字符串"：usvg-0.47.0/src/parser/paint_server.rs:230-233 的
`convert_units` 只接受两个精确 token（不 trim、区分大小写），其余回退该属性的默认值：

* `maskContentUnits="foo"` / `" objectBoundingBox "` ⇒ 默认 userSpaceOnUse，内容要缩放；
  只有严格 `objectBoundingBox` 才逐字保留；`maskUnits=" userSpaceOnUse "` ⇒ 默认 bbox 区域；
* `markerUnits="userSpaceOnUse"` ⇒ 视口/参考点/内容都是用户长度，必须随面板缩放；
* `gradientUnits=" userSpaceOnUse "` ⇒ 默认 objectBoundingBox，坐标是 bbox 分数，不能缩放；
* pattern tile 的 x/y/width/height 逐个沿 href 链继承，按解析后的 patternUnits 解释；
* IRI 语法：`" #base"`/`"#base "` 合法（跳过字面空格），`"#base extra"` 非法。

每族都用参考渲染器（d1render = resvg）做像素对照，s=1 与 s=2 各测一次。
退出码 0 = 全部成立。无参数、离线、不写仓库。

fail-closed 段：设置 SVG_GRID_OLD_BIN=<755ad5b 的 svg_grid> 时，同一批用例在旧二进制上
必须至少 5 条为红；否则 FAIL。不给则打印 SKIP。
"""

import os
import struct
import subprocess
import sys
import tempfile
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(HERE))))
NEW = os.environ.get("SVG_GRID_BIN") or os.path.join(ROOT, "target", "debug", "svg_grid")
OLD = os.environ.get("SVG_GRID_OLD_BIN")
RENDER = os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
TMP = tempfile.mkdtemp(prefix="svg-grid-r43-")
PANEL_W, PANEL_H = 100, 80
FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        if detail:
            print("      " + str(detail).replace("\n", "\n      ")[:1200])
        FAILS.append(label)


def skip(label, why):
    print(f"SKIP  {label}  （{why}）")


def run(args):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True)


def read_png(path):
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"not a PNG: {path}")
    pos, width, height, depth, color, rows = 8, 0, 0, 0, 0, b""
    while pos < len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, color = struct.unpack(">IIBB", body[:10])
        elif kind == b"IDAT":
            rows += body
        elif kind == b"IEND":
            break
    raw = zlib.decompress(rows)
    if depth != 8 or color not in (2, 6):
        raise SystemExit(f"unsupported PNG: depth={depth} color={color}")
    channels = 3 if color == 2 else 4
    stride = width * channels
    pixels, previous, offset = [], bytearray(stride), 0
    for _ in range(height):
        filt = raw[offset]
        line = bytearray(raw[offset + 1:offset + 1 + stride])
        offset += 1 + stride
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            up_left = previous[i - channels] if i >= channels else 0
            if filt == 1:
                line[i] = (line[i] + left) & 0xFF
            elif filt == 2:
                line[i] = (line[i] + up) & 0xFF
            elif filt == 3:
                line[i] = (line[i] + ((left + up) >> 1)) & 0xFF
            elif filt == 4:
                p = left + up - up_left
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - up_left)
                pred = left if pa <= pb and pa <= pc else (up if pb <= pc else up_left)
                line[i] = (line[i] + pred) & 0xFF
        pixels.append(bytes(line))
        previous = line
    return width, height, channels, pixels


def ink(path):
    width, _, channels, rows = read_png(path)
    count = 0
    for row in rows:
        for x in range(width):
            if row[x * channels:x * channels + 3] != b"\xff\xff\xff":
                count += 1
    return count


def diff(left, right):
    lw, lh, lc, lrows = read_png(left)
    rw, rh, rc, rrows = read_png(right)
    if (lw, lh, lc) != (rw, rh, rc):
        return (10 ** 9, 255)
    xor = maxdiff = 0
    for row_l, row_r in zip(lrows, rrows):
        for x in range(lw):
            a = row_l[x * lc:x * lc + 3]
            b = row_r[x * lc:x * lc + 3]
            if a != b:
                xor += 1
                maxdiff = max(maxdiff, max(abs(a[i] - b[i]) for i in range(3)))
    return xor, maxdiff


def write(path, text):
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)
    return path


def panel(inner):
    return (
        f'<svg width="{PANEL_W}" height="{PANEL_H}" viewBox="0 0 {PANEL_W} {PANEL_H}" '
        'xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">'
        f'<rect data-panel-box="main" x="0" y="0" width="{PANEL_W}" height="{PANEL_H}" '
        'fill="none" stroke="none"/>'
        f'<g data-scale="xy">{inner}</g></svg>'
    )


def compose(binary, src, tag, scale):
    out = os.path.join(TMP, f"{tag}-{scale}.svg")
    done = run([binary, "plot-grid", "--input", src, "--output", out,
                "--width", PANEL_W * scale, "--height", PANEL_H * scale,
                "--ncol", "1", "--plot-margin", "0"])
    if done.returncode != 0:
        raise SystemExit(f"compose failed: {done.stderr[-400:]}")
    return out


def render(svg, tag, scale):
    out = os.path.join(TMP, f"{tag}-{scale}.png")
    done = run([RENDER, svg, out, scale])
    if done.returncode != 0:
        raise SystemExit(f"render failed: {done.stderr[-400:]}")
    return out


def compare(name, src_text, scale, binary=NEW):
    src = write(os.path.join(TMP, f"{name}.svg"), src_text)
    out_svg = compose(binary, src, name, scale)
    ref = render(src, f"{name}-ref", scale)
    got = render(out_svg, f"{name}-out", 1)
    xor, maxdiff = diff(ref, got)
    return xor, maxdiff, ink(ref), ink(got)


MASK_UNITS_WS = panel(
    '<defs><mask id="m" maskUnits=" userSpaceOnUse " maskContentUnits="userSpaceOnUse" '
    'x="0.1" y="0.1" width="0.4" height="0.4">'
    '<rect x="10" y="10" width="40" height="30" fill="#ffffff"/></mask></defs>'
    '<rect x="0" y="0" width="100" height="80" fill="#000000" mask="url(#m)"/>'
)
MASK_CONTENT_INVALID = panel(
    '<defs><mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="foo" '
    'x="10" y="10" width="40" height="30">'
    '<rect x="10" y="10" width="40" height="30" fill="#ffffff"/></mask></defs>'
    '<rect x="0" y="0" width="100" height="80" fill="#000000" mask="url(#m)"/>'
)
MASK_CONTENT_WS = panel(
    '<defs><mask id="m" maskUnits="userSpaceOnUse" maskContentUnits=" objectBoundingBox " '
    'x="10" y="10" width="40" height="30">'
    '<rect x="10" y="10" width="40" height="30" fill="#ffffff"/></mask></defs>'
    '<rect x="0" y="0" width="100" height="80" fill="#000000" mask="url(#m)"/>'
)
MASK_CONTENT_OBB = panel(
    '<defs><mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="objectBoundingBox" '
    'x="10" y="10" width="40" height="30">'
    '<rect x="0.2" y="0.2" width="0.5" height="0.5" fill="#ffffff"/></mask></defs>'
    '<rect x="0" y="0" width="100" height="80" fill="#000000" mask="url(#m)"/>'
)
MARKER_USER = panel(
    '<defs><marker id="k" markerUnits="userSpaceOnUse" markerWidth="20" markerHeight="20" '
    'refX="10" refY="10" orient="auto">'
    '<path d="M 0 0 L 20 10 L 0 20 Z" fill="#3366cc"/></marker></defs>'
    '<path d="M 10 40 L 90 40" stroke="#000000" stroke-width="2" marker-end="url(#k)"/>'
)
MARKER_DEFAULT = panel(
    '<defs><marker id="k" markerWidth="20" markerHeight="20" '
    'refX="10" refY="10" orient="auto">'
    '<path d="M 0 0 L 20 10 L 0 20 Z" fill="#3366cc"/></marker></defs>'
    '<path d="M 10 40 L 90 40" stroke="#000000" stroke-width="2" marker-end="url(#k)"/>'
)
TILE_MIXED = panel(
    '<defs><pattern id="base" x="2" y="3" width="20" height="18">'
    '<rect x="0" y="0" width="10" height="10" fill="#3366cc"/></pattern>'
    '<pattern id="p" href="#base" patternUnits="userSpaceOnUse" x="10">'
    '<rect x="0" y="0" width="10" height="10" fill="#3366cc"/></pattern></defs>'
    '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>'
)
TILE_INHERITED = panel(
    '<defs><pattern id="base" patternUnits="userSpaceOnUse" x="10" y="10" '
    'width="20" height="20">'
    '<rect x="1" y="2" width="6" height="5" fill="#3366cc"/></pattern>'
    '<pattern id="p" href="#base"/></defs>'
    '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>'
)
HREF_LEADING = TILE_INHERITED.replace('href="#base"', 'href=" #base"')
HREF_TRAILING = TILE_INHERITED.replace('href="#base"', 'href="#base "')
HREF_INVALID = panel(
    '<defs><pattern id="base" patternContentUnits="objectBoundingBox">'
    '<rect x="0.1" y="0.1" width="0.5" height="0.5" fill="#3366cc"/></pattern>'
    '<pattern id="p" href="#base extra" patternUnits="userSpaceOnUse" '
    'x="10" y="10" width="20" height="20">'
    '<rect x="2" y="3" width="6" height="5" fill="#3366cc"/></pattern></defs>'
    '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>'
)
GRADIENT_WS = panel(
    '<defs><linearGradient id="g" gradientUnits=" userSpaceOnUse " x1="0" y1="0" x2="50" y2="0">'
    '<stop offset="0" stop-color="#000000"/><stop offset="0.5" stop-color="#888888"/>'
    '<stop offset="1" stop-color="#ffffff"/></linearGradient></defs>'
    '<rect x="0" y="0" width="100" height="80" fill="url(#g)"/>'
)

CASES = [
    ("mask_units_ws", MASK_UNITS_WS),
    ("mask_content_invalid", MASK_CONTENT_INVALID),
    ("mask_content_ws_obb", MASK_CONTENT_WS),
    ("mask_content_strict_obb", MASK_CONTENT_OBB),
    ("marker_user", MARKER_USER),
    ("marker_default", MARKER_DEFAULT),
    ("tile_mixed", TILE_MIXED),
    ("tile_inherited", TILE_INHERITED),
    ("href_leading_space", HREF_LEADING),
    ("href_trailing_space", HREF_TRAILING),
    ("href_invalid_fragment", HREF_INVALID),
    ("gradient_units_ws", GRADIENT_WS),
]

LEFTOVERS = [
    ("nested_pattern", panel(
        '<defs><pattern id="outer" patternUnits="userSpaceOnUse" x="0" y="0" '
        'width="40" height="40">'
        '<rect x="0" y="0" width="40" height="40" fill="#ffffff"/>'
        '<pattern id="inner" patternUnits="userSpaceOnUse" x="0" y="0" '
        'width="10" height="10">'
        '<rect x="0" y="0" width="5" height="5" fill="#3366cc"/></pattern>'
        '<rect x="0" y="0" width="40" height="40" fill="url(#inner)"/></pattern></defs>'
        '<rect x="0" y="0" width="100" height="80" fill="url(#outer)"/>')),
    ("pattern_transform", panel(
        '<defs><pattern id="p" patternUnits="userSpaceOnUse" x="0" y="0" '
        'width="20" height="20" patternTransform="translate(4 6) scale(2)">'
        '<rect x="0" y="0" width="8" height="8" fill="#3366cc"/></pattern></defs>'
        '<rect x="0" y="0" width="100" height="80" fill="url(#p)"/>')),
]


def family_units(scale, binary=NEW, record=True):
    reds = []
    for name, source in CASES:
        xor, maxdiff, ref_ink, out_ink = compare(name, source, scale, binary)
        ok = xor == 0 and maxdiff == 0 and ref_ink > 0 and out_ink > 0
        if record:
            check(ok, f"{name} s={scale}: resvg 像素一致",
                  f"xor={xor} md={maxdiff} ref_ink={ref_ink} out_ink={out_ink}")
        else:
            print(("RED   " if not ok else "green ") + f"{name} s={scale}: "
                  f"xor={xor} md={maxdiff} ref_ink={ref_ink} out_ink={out_ink}")
        if not ok:
            reds.append(name)
    return reds


def family_leftovers():
    if not OLD:
        skip("G 遗留面（需要 SVG_GRID_OLD_BIN）", "未设旧二进制")
        return
    for name, source in LEFTOVERS:
        src = write(os.path.join(TMP, f"{name}.svg"), source)
        old_svg = compose(OLD, src, f"{name}-old", 2)
        new_svg = compose(NEW, src, f"{name}-new", 2)
        same = open(old_svg, "rb").read() == open(new_svg, "rb").read()
        check(same, f"G 遗留面 {name}：与 755ad5b 输出逐字节相同（未恶化）")


def family_fail_closed():
    if not OLD:
        skip("F fail-closed：旧二进制必须红", "未设 SVG_GRID_OLD_BIN")
        return
    reds = family_units(2, OLD, record=False)
    check(len(reds) >= 5, "F 755ad5b 上至少有 5 条为红（用例真的有牙）", f"reds={reds}")


def main():
    if not os.path.isfile(NEW):
        print(f"FAIL  binary not found: {NEW} (run `cargo build --offline`)")
        return 1
    if not os.path.isfile(RENDER):
        print(f"FAIL  reference renderer not found: {RENDER}")
        return 1
    print("== A–D 单位语义（s=1）==")
    family_units(1)
    print("\n== A–D 单位语义（s=2）==")
    family_units(2)
    print("\n== G 遗留面 ==")
    family_leftovers()
    print("\n== F fail-closed ==")
    family_fail_closed()
    print()
    if FAILS:
        print(f"== {len(FAILS)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
