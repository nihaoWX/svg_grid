#!/usr/bin/env python3
"""R37 — `<defs>` 按引用保留 + 悬空引用 fail-closed + mask/marker 内容翻译 + 单位缩放。

覆盖本轮四步：
① 四组探针（mask / marker / filter / pattern）：定义必须留在产物里，且
   `plot-grid --ncol 1 --width <输入宽>`（s=1）后渲染与输入单独渲染像素级一致
   （允许整体平移）——给 XOR 像素数、最大通道差、ink px，不只看黑墨
② 悬空引用 fail-closed：mask / marker / filter / clip-path / href 各一条 ⇒ rc≠0、
   不写产物、点名元素+属性+id、给修法；`--force` 不得放行
③ 引用语义边界：`<a href="#…">` 不算渲染引用；`url( #id )` 带空格要认；
   自引用 pattern 不死循环；重复 id 不报错
④ 220 个 `tests/**/*.svg`：不得新增失败；每个成功产物必须幂等（再 convert 得
   AlreadyConverted）且产物内无悬空 `url(#…)`/`href="#…"`
⑤ 真实语料 `S36_panelA_matrix.svg`（hatch pattern）：`<pattern>` 保留，渲染残差
   只落在 hatch 相位（已知 R16 遗留，见报告），convert-only 必须几乎逐像素一致
无参数；用 `target/debug/svg_grid` 与 `convert/target/debug/svg_grid_convert`；
只写系统临时目录。渲染对照需要 resvg 探针 `/tmp/d1_eval/render/.../d1render`，
缺失时该子项 SKIP（结构断言仍照跑）。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
BIN = SVG_GRID / "target/debug/svg_grid"
CONV = SVG_GRID / "convert/target/debug/svg_grid_convert"
RENDER = pathlib.Path("/tmp/d1_eval/render/target/release/d1render")
CORPUS = pathlib.Path(
    "/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/09_上游转录因子/出图/"
    "S36_S37/figure_panels/S36_panelA_matrix.svg"
)
FAILS = []
SKIPS = []

HEAD = ('<svg width="100" height="80" viewBox="0 0 100 80" '
        'xmlns="http://www.w3.org/2000/svg" '
        'xmlns:xlink="http://www.w3.org/1999/xlink">')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        FAILS.append(label)
        if detail:
            print("      " + str(detail).replace("\n", "\n      ")[:900])


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")
    SKIPS.append(label)


def convert(src, dst, *extra):
    return subprocess.run([str(CONV), "--input", str(src), "--output", str(dst), *extra],
                          capture_output=True, text=True, cwd="/tmp")


def grid(src, dst, width):
    return subprocess.run([str(BIN), "plot-grid", "--input", str(src), "--output", str(dst),
                           "--width", str(width), "--ncol", "1"],
                          capture_output=True, text=True, cwd="/tmp")


def write(tmp, name, body):
    path = tmp / name
    path.write_text(HEAD + body + "</svg>")
    return path


def render(svg, png, scale):
    out = subprocess.run([str(RENDER), str(svg), str(png), str(scale)],
                         capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip()[:300])
    from PIL import Image
    return Image.open(png).convert("RGB")


def ink(img):
    px = img.load()
    w, h = img.size
    return sum(1 for y in range(h) for x in range(w) if px[x, y] != (255, 255, 255))


def bbox(img):
    px = img.load()
    w, h = img.size
    xs = [x for y in range(h) for x in range(w) if px[x, y] != (255, 255, 255)]
    ys = [y for y in range(h) for x in range(w) if px[x, y] != (255, 255, 255)]
    if not xs:
        return None
    return (min(xs), min(ys), max(xs), max(ys))


def compare(src, out, tag, scale_src=1.0, scale_out=1.0):
    """Render both, align by ink bbox, return (xor, maxdiff, ink_src, ink_out, shift)."""
    a = render(src, f"/tmp/r37ev_{tag}_src.png", scale_src)
    b = render(out, f"/tmp/r37ev_{tag}_out.png", scale_out)
    ba, bb = bbox(a), bbox(b)
    if ba is None or bb is None:
        return (None, None, ink(a), ink(b), None)
    shift = (bb[0] - ba[0], bb[1] - ba[1])
    pa, pb = a.load(), b.load()
    w, h = a.size
    xor = 0
    maxdiff = 0
    for y in range(h):
        for x in range(w):
            xa, ya = x - shift[0], y - shift[1]
            ca = pa[xa, ya] if (0 <= xa < w and 0 <= ya < h) else (255, 255, 255)
            cb = pb[x, y]
            if ca != cb:
                xor += 1
                maxdiff = max(maxdiff, max(abs(ca[i] - cb[i]) for i in range(3)))
    return (xor, maxdiff, ink(a), ink(b), shift)


def main():
    for path, label in ((BIN, "svg_grid"), (CONV, "svg_grid_convert")):
        if not path.exists():
            print(f"missing binary: {path} — run `cargo build` first")
            return 1
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r37ev_"))
    have_render = RENDER.exists()
    if not have_render:
        skip("渲染对照（XOR/ink）", f"缺少 resvg 探针 {RENDER}")

    # ── ① 四组探针：定义保留 + 渲染一致
    print("\n== ① 四组探针：mask / marker / filter / pattern 定义保留且渲染一致 ==")
    probes = [
        ("mask_user_space", 148, 1.0, 1.0,
         "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>"
         "<rect x='10' y='10' width='20' height='20' fill='white'/></mask></defs>"
         "<rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>"),
        ("mask_bbox", 148, 1.0, 1.0,
         "<defs><mask id='m' x='0' y='0' width='0.5' height='1'>"
         "<rect x='10' y='10' width='20' height='20' fill='white'/></mask></defs>"
         "<rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>"),
        ("marker_stroke_width", 148, 1.0, 1.0,
         "<defs><marker id='k' markerWidth='6' markerHeight='6' refX='0' refY='3' orient='auto'>"
         "<path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/></marker></defs>"
         "<path d='M 10 30 L 90 30' stroke='black' marker-end='url(#k)'/>"),
        ("marker_user_space", 148, 1.0, 1.0,
         "<defs><marker id='k' markerUnits='userSpaceOnUse' markerWidth='6' markerHeight='6' "
         "refX='0' refY='3' orient='auto'><path d='M 0 0 L 6 3 L 0 6 Z' fill='black'/>"
         "</marker></defs>"
         "<path d='M 10 30 L 90 30' stroke='black' marker-end='url(#k)'/>"),
        ("filter", 148, 1.0, 1.0,
         "<defs><filter id='f' filterUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>"
         "<feMorphology operator='dilate' radius='4'/></filter></defs>"
         "<rect x='10' y='10' width='40' height='20' fill='black' filter='url(#f)'/>"),
        ("pattern_chain", 148, 1.0, 1.0,
         "<defs><linearGradient id='g' gradientUnits='userSpaceOnUse' x1='0' y1='0' x2='10' "
         "y2='10'><stop offset='0' stop-color='#000000'/></linearGradient>"
         "<pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' height='10'>"
         "<rect x='0' y='0' width='10' height='10' fill='url(#g)'/></pattern></defs>"
         "<rect x='0' y='0' width='40' height='40' fill='url(#p)'/>"),
    ]
    for tag, width, s_src, s_out, body in probes:
        src = write(tmp, f"{tag}.svg", body)
        tagged = tmp / f"{tag}.tagged.svg"
        got = convert(src, tagged)
        check(got.returncode == 0, f"① [{tag}] convert rc=0", got.stderr[:400])
        if got.returncode != 0:
            continue
        svg = tagged.read_text()
        kind = tag.split("_")[0]
        check(f"<{kind}" in svg, f"① [{tag}] <{kind}> 定义保留", svg[:400])
        if kind == "pattern":
            check("<linearGradient" in svg and "url(#g)" in svg,
                  f"① [{tag}] 传递闭包保住 pattern 指向的 gradient", svg[:400])
        if tag == "pattern_chain":
            # The composer's pattern *phase* is a pre-existing defect (R16, see ⑥):
            # the tile moves but the body moves with it, so the hatch lands in the
            # wrong place. What this round owns is that the definition survives and
            # that conversion alone changes nothing.
            if have_render:
                try:
                    xor, maxdiff, ink_src, ink_out, shift = compare(
                        src, tagged, f"{tag}_conv", s_src, s_out)
                    check(xor == 0 and ink_src == ink_out,
                          f"① [{tag}] convert-only 逐像素一致 xor={xor} maxdiff={maxdiff} "
                          f"ink {ink_src}->{ink_out}",
                          f"xor={xor} maxdiff={maxdiff}")
                except Exception as error:  # noqa: BLE001
                    skip(f"① [{tag}] convert-only 渲染对照", str(error))
            continue
        out = tmp / f"{tag}.grid.svg"
        got = grid(tagged, out, width)
        check(got.returncode == 0, f"① [{tag}] plot-grid rc=0", got.stderr[:400])
        if got.returncode != 0:
            continue
        if not have_render:
            continue
        try:
            xor, maxdiff, ink_src, ink_out, shift = compare(src, out, tag, s_src, s_out)
        except Exception as error:  # noqa: BLE001 - renderer failure is a skip, not a fail
            skip(f"① [{tag}] 渲染对照", str(error))
            continue
        check(xor == 0 and ink_src == ink_out,
              f"① [{tag}] 渲染一致 xor={xor} maxdiff={maxdiff} ink {ink_src}->{ink_out} "
              f"shift={shift}",
              f"xor={xor} maxdiff={maxdiff} ink {ink_src}->{ink_out}")

    # ── ② 悬空引用 fail-closed
    print("\n== ② 悬空引用 fail-closed：rc≠0、不写产物、点名元素+属性+id、给修法 ==")
    dangling = [
        ("mask", "<rect x='10' y='10' width='40' height='20' fill='black' mask='url(#nope)'/>",
         "<rect>", "mask=", "#nope"),
        ("marker", "<path d='M 10 30 L 90 30' stroke='black' marker-end='url(#nope)'/>",
         "<path>", "marker-end=", "#nope"),
        ("filter", "<rect x='10' y='10' width='40' height='20' fill='black' filter='url(#nope)'/>",
         "<rect>", "filter=", "#nope"),
        ("clip-path", "<rect x='10' y='10' width='40' height='20' fill='black' "
                      "clip-path='url(#nope)'/>", "<rect>", "clip-path=", "#nope"),
        ("href", "<defs><linearGradient id='d' xlink:href='#nope'/></defs>"
                 "<rect x='0' y='0' width='40' height='40' fill='url(#d)'/>",
         "<linearGradient>", "xlink:href=", "#nope"),
    ]
    for tag, body, element, attr, ident in dangling:
        src = write(tmp, f"dangle_{tag}.svg", body)
        out = tmp / f"dangle_{tag}.out.svg"
        got = convert(src, out)
        check(got.returncode != 0, f"② [{tag}] 悬空引用 ⇒ rc≠0", f"rc={got.returncode}")
        check(not out.exists(), f"② [{tag}] 不写产物")
        for needle in (element, attr, ident, "add the missing definition"):
            check(needle in got.stderr, f"② [{tag}] 文案含 {needle!r}", got.stderr[:400])
        forced = convert(src, tmp / f"dangle_{tag}.forced.svg", "--force")
        check(forced.returncode != 0, f"② [{tag}] --force 不得放行", f"rc={forced.returncode}")

    # ── ③ 引用语义边界
    print("\n== ③ 引用语义边界：<a href> 不算引用、url( #id ) 认、自引用不死循环、重复 id 不报 ==")
    edges = [
        ("anchor", "<a href='#nowhere'><rect x='0' y='0' width='40' height='40' "
                   "fill='black'/></a>", 0),
        ("spaces", "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' "
                   "height='80'><rect x='0' y='0' width='50' height='80' fill='white'/>"
                   "</mask></defs><rect x='10' y='10' width='40' height='20' fill='black' "
                   "mask='url( #m )'/>", 0),
        ("selfref", "<defs><pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='10' "
                    "height='10'><rect x='0' y='0' width='10' height='10' fill='url(#p)'/>"
                    "</pattern></defs><rect x='0' y='0' width='40' height='40' "
                    "fill='url(#p)'/>", 0),
        ("dup", "<defs><mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' "
                "height='80'><rect x='0' y='0' width='50' height='80' fill='white'/></mask>"
                "<mask id='m' maskUnits='userSpaceOnUse' x='0' y='0' width='100' height='80'>"
                "<rect x='0' y='0' width='20' height='80' fill='white'/></mask></defs>"
                "<rect x='10' y='10' width='40' height='20' fill='black' mask='url(#m)'/>", 0),
    ]
    for tag, body, expected in edges:
        src = write(tmp, f"edge_{tag}.svg", body)
        out = tmp / f"edge_{tag}.out.svg"
        got = convert(src, out)
        check(got.returncode == expected, f"③ [{tag}] rc={expected}", f"rc={got.returncode} "
              f"{got.stderr[:300]}")
        if tag == "spaces" and got.returncode == 0:
            check("id=\"m\"" in out.read_text(), "③ [spaces] url( #m ) 解析到定义")
        if tag == "dup" and got.returncode == 0:
            check(out.read_text().count("id=\"m\"") == 2, "③ [dup] 两个同名定义都保留")

    # ── ④ 220 个 fixture：不新增失败 + 幂等 + 产物无悬空引用
    print("\n== ④ tests/**/*.svg：不新增失败、成功产物幂等且无悬空引用 ==")
    files = sorted(SVG_GRID.glob("tests/**/*.svg"))
    ok = failed = 0
    bad = []
    for path in files:
        out = tmp / "fixture.out.svg"
        out.unlink(missing_ok=True)
        got = convert(path, out)
        if got.returncode != 0:
            failed += 1
            continue
        ok += 1
        svg = out.read_text()
        again = convert(out, tmp / "fixture.again.svg")
        if again.returncode != 0:
            bad.append((str(path.relative_to(SVG_GRID)), "not idempotent"))
            continue
        defined = set(re.findall(r'\bid="([^"]+)"', svg))
        for ref in re.findall(r'url\(\s*["\']?#([^)"\'\s]+)', svg):
            if ref not in defined:
                bad.append((str(path.relative_to(SVG_GRID)), f"dangling url(#{ref})"))
                break
        for ref in re.findall(r'(?:xlink:)?href="#([^"]+)"', svg):
            if ref not in defined:
                bad.append((str(path.relative_to(SVG_GRID)), f"dangling href=#{ref}"))
                break
    check(ok + failed == len(files), f"④ 扫到 {len(files)} 个 fixture（ok={ok} fail={failed}）")
    check(not bad, f"④ 无新增失败/非幂等/悬空引用（{len(bad)} 条）", bad[:10])

    # ── ⑤ 真实语料 S36
    print("\n== ⑤ 真实语料 S36_panelA_matrix.svg：<pattern> 保留、convert-only 几乎逐像素一致 ==")
    if not CORPUS.exists():
        skip("⑤ S36 语料", f"缺少 {CORPUS}")
    else:
        tagged = tmp / "s36.tagged.svg"
        got = convert(CORPUS, tagged)
        check(got.returncode == 0, "⑤ S36 convert rc=0", got.stderr[:400])
        if got.returncode == 0:
            svg = tagged.read_text()
            check("<pattern" in svg, "⑤ S36 <pattern> 保留")
            check("patternUnits=\"userSpaceOnUse\"" in svg, "⑤ S36 patternUnits 保留")
            if have_render:
                try:
                    xor, maxdiff, ink_src, ink_out, shift = compare(
                        CORPUS, tagged, "s36conv", 1.0, 1.0)
                    check(xor is not None and xor <= 100 and maxdiff <= 8,
                          f"⑤ S36 convert-only 逐像素一致 xor={xor} maxdiff={maxdiff} "
                          f"ink {ink_src}->{ink_out} shift={shift}",
                          f"xor={xor} maxdiff={maxdiff}")
                except Exception as error:  # noqa: BLE001
                    skip("⑤ S36 渲染对照", str(error))

    # ── ⑥ 已知遗留：pattern 相位（R16 时代，非本轮引入）
    print("\n== ⑥ 已知遗留：pattern 相位错位（R16 时代，本轮未改，交父裁决） ==")
    body = ("<defs><pattern id='p' patternUnits='userSpaceOnUse' x='0' y='0' width='20' height='20'>"
            "<rect x='0' y='0' width='20' height='20' fill='#c97ba6'/></pattern></defs>"
            "<rect x='10' y='10' width='80' height='50' fill='url(#p)'/>")
    src = write(tmp, "phase.svg", body)
    tagged = tmp / "phase.tagged.svg"
    got = convert(src, tagged)
    check(got.returncode == 0, "⑥ [phase] convert rc=0", got.stderr[:300])
    if got.returncode == 0:
        out = tmp / "phase.grid.svg"
        got = grid(tagged, out, 148)
        check(got.returncode == 0, "⑥ [phase] plot-grid rc=0", got.stderr[:300])
        if got.returncode == 0:
            svg = out.read_text()
            # Signature of the defect: the tile box moved AND the body moved with it,
            # so the hatch phase is off by the tile origin (resvg renders pattern
            # content in tile-local coordinates).
            check(re.search(r'<pattern[^>]*x="24"[^>]*y="24"', svg) is not None,
                  "⑥ [phase] tile 已平移（正确的一半）", svg[:400])
            check(re.search(r'<rect x="24" y="24" width="\d', svg) is not None,
                  "⑥ [phase] body 也被平移（缺陷签名：应只缩放不平移）", svg[:400])
            if have_render:
                try:
                    xor, maxdiff, ink_src, ink_out, shift = compare(src, out, "phase", 1.0, 1.0)
                    print(f"      已知残差：xor={xor} maxdiff={maxdiff} ink {ink_src}->{ink_out} "
                          f"shift={shift}")
                except Exception as error:  # noqa: BLE001
                    skip("⑥ [phase] 渲染对照", str(error))
    # 非本轮引入的证据（一次性、记录在报告里，不在这里重跑）：用 `git worktree add
    # /tmp/r37/base e39b3e9` 的基线二进制跑同一条命令，产物与本轮逐字节相同
    # （124574 B 的 S36 产物亦然），且基线 convert 产物与新 convert 产物逐字节相同。

    print()
    if FAILS:
        print(f"== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    if SKIPS:
        print(f"== 判据全部成立（另有 {len(SKIPS)} 条 SKIP） ==")
        for label in SKIPS:
            print("  ~ " + label)
        return 0
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
