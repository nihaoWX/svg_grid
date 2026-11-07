#!/usr/bin/env python3
"""R16 的可跑证据 —— **paint server（渐变 / 图案）不再被丢，且 `url(#…)` 悬空必定出声（W9）**。

报告见 `docs/PITFALLS/svg_grid-阻塞点-R16-渐变与图案被丢弃.md`。本轮钉六族：

① **墨水恢复**（两张目标图，`--width 216 --plot-margin 0 --png-dpi 300`）：
   `ggplot2/gradient_fill` ≥ 0.55（实测 0.6809；源图 0.6713；未修 0.0672）、
   `matplotlib/bars_grouped` ≥ 0.35（实测 0.4245；源图 0.3891；未修 0.2180）。
   判据是「阈值 + 消融对照」：把 tagged 里的 `<linearGradient>/<radialGradient>/<pattern>` 删掉再渲，
   墨水必须塌回阈值之下（证明这些墨水真是 paint server 画的，不是别处挪来的）。
   源图值（cairosvg 渲的）本机没装 cairosvg ⇒ **只打印参考值不判**（跳过并打印原因）。
② **paint server 在 tagged 里一个不少**：`<linearGradient>`/`<radialGradient>`/`<pattern>`/`<clipPath>`
   计数与源图逐项相等；tagged 的每一处 `url(#…)` 都能解析到本文件定义的 id（**零悬空**）；
   引用条数与源图相同。
③ **干净路径零输出**：两张目标图的组图侧 `stderr` 必须 **0 字节**（修好之后不该再有任何告警）。
④ **W9 出声**：悬空 `url(#no-such-id)` ⇒ 恰好一条 W9，报文含「是什么」（`N reference(s) point at an id
   that nothing in the output defines` + `input #0 <rect fill> -> url(#no-such-id)`）、「为什么」
   （`painted with nothing` … `silently loses ink`）、「怎么修」（`Fix:` + `<defs>`）；rc 仍是 0、产物照写。
   多引用（`clip-path` + 内联 `style`）合一条、计数 `2 reference(s)`、两个 id 都点名。
   干净输入（引用都能解析）⇒ **0 条 W9 且 stderr 0 字节**。
⑤ **58 张语料全量跑两段**：`expect = "unsupported"` 的按 MANIFEST 计 SKIP 并打印原因，其余两段 rc 都必须 0；
   逐条打印 W1/W6 次数（只统计，不判失败）；W9 全语料合计必须 0（②的全量版）。
   另加一条「W1 计数器是活的」对照：同一份 tagged 用 `--width 108` 重跑 ⇒ 必须出 W1。
⑥ **回归**：R4–R15 的 `test.py` 全绿。

无参数。显式用**仓库内** debug 二进制（`convert/target/debug/svg_grid_convert`、`target/debug/svg_grid`），
字体目录走 `$SVG_GRID_FONT_DIR`（并在每条命令上加 `--font-dir`），不依赖 cwd。
离线、只读（不写仓库），临时产物在临时目录。约 6–8 分钟，绝大部分是回归 R4–R15（R15 自己会重跑 R4–R14）。
"""
import os
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R16 → PITFALLS → docs → svg_grid
BIN = SVG_GRID / "target/debug/svg_grid"         # 仓库内 debug：~/.local/bin 上是旧版本
CONVERT = SVG_GRID / "convert/target/debug/svg_grid_convert"
FIXTURES = SVG_GRID / "tests/fixtures"
MANIFEST = FIXTURES / "MANIFEST.toml"
FONT_DIR = pathlib.Path(os.environ.get("SVG_GRID_FONT_DIR")
                        or pathlib.Path.home() / "svg_grid/fontlib")

# 两张目标图：阈值 + 实测值 + 源图参考值（cairosvg 渲的，报告里记的数）+ 未修时的值。
TARGETS = {
    "ggplot2/gradient_fill": {
        "min_ink": 0.55, "measured": 0.6809, "source_ref": 0.6713, "before_fix": 0.0672,
        "servers": {"linearGradient": 13, "radialGradient": 12, "pattern": 0},
    },
    "matplotlib/bars_grouped": {
        "min_ink": 0.35, "measured": 0.4245, "source_ref": 0.3891, "before_fix": 0.2180,
        "servers": {"linearGradient": 0, "radialGradient": 0, "pattern": 1},
    },
}
SERVERS = ("linearGradient", "radialGradient", "pattern", "clipPath")
W1 = "scaled by the layout"
W6 = "draws ink outside its own canvas box"
W9 = "point at an id that nothing in the output defines"

FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def run(args, cwd=None):
    return subprocess.run(args, capture_output=True, text=True, cwd=cwd)


def compose(src, out_svg, out_png=None, width="216", extra=()):
    args = [str(BIN), "plot-grid", "--input", str(src), "--output", str(out_svg)]
    if out_png is not None:
        args += ["--output-png", str(out_png), "--png-dpi", "300"]
    args += ["--width", str(width), "--plot-margin", "0", "--font-dir", str(FONT_DIR), *extra]
    return run(args, cwd="/tmp")            # 不依赖 cwd


def convert(fid, out_svg):
    return run([str(CONVERT), "--input", str(FIXTURES / (fid + ".svg")),
                "--output", str(out_svg)], cwd="/tmp")


def ink(png):
    """墨水占比 = PNG 灰度 < 250 的像素占比（与 MANIFEST / 报告同一个口径）。"""
    import numpy as np
    from PIL import Image
    return float((np.asarray(Image.open(png).convert("L")) < 250).mean())


def ids_of(text):
    """本文件定义的全部 id，双引号与单引号两种写法都算（svglite 用单引号写 `id='pat-0'`）。"""
    return {double or single for double, single
            in re.findall(r"""\bid\s*=\s*(?:"([^"]+)"|'([^']+)')""", text)}


def refs_of(text):
    return re.findall(r"""url\(\s*#([^)'"]+)\s*\)""", text)


def dangling_of(text):
    defined = ids_of(text)
    return sorted({ref for ref in refs_of(text) if ref not in defined})


def count_tag(text, tag):
    return len(re.findall(r"<" + tag + r"\b", text))


def load_manifest():
    """(id, expect, why) 三元组；`why` 优先取条目的 `why = "…"`，否则取该条目块里的注释。"""
    text = MANIFEST.read_text()
    entries = []
    try:
        import tomllib
        raw = tomllib.loads(text)["fixture"]
        blocks = text.split("[[fixture]]")[1:]
        for fixture, block in zip(raw, blocks):
            comments = [line.lstrip("# ").strip() for line in block.splitlines()
                        if line.strip().startswith("#")]
            why = fixture.get("why") or " ".join(comments)
            entries.append((fixture["id"], fixture.get("expect", "pass"), why))
        return entries
    except Exception:
        pass
    for block in text.split("[[fixture]]")[1:]:
        fid = re.search(r"""^\s*id\s*=\s*["']([^"']+)""", block, re.M)
        expect = re.search(r"""^\s*expect\s*=\s*["']([^"']+)""", block, re.M)
        why = re.search(r"""^\s*why\s*=\s*["']([^"']+)""", block, re.M)
        comments = [line.lstrip("# ").strip() for line in block.splitlines()
                    if line.strip().startswith("#")]
        if fid:
            entries.append((fid.group(1), expect.group(1) if expect else "pass",
                            (why.group(1) if why else "") or " ".join(comments)))
    return entries


def main():
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r16-rounddata-"))
    print(f"# 仓库内二进制：{CONVERT}\n#               {BIN}\n# 字体目录：{FONT_DIR}\n")

    # ── 0. 前置：两个二进制、字体目录、语料登记
    check(CONVERT.is_file() and os.access(CONVERT, os.X_OK),
          "仓库内 `convert/target/debug/svg_grid_convert` 存在且可执行（不是 ~/.local/bin 的旧版）",
          f"{CONVERT}")
    check(BIN.is_file() and os.access(BIN, os.X_OK),
          "仓库内 `target/debug/svg_grid` 存在且可执行", f"{BIN}")
    if not (CONVERT.is_file() and BIN.is_file()):
        print("\n== 前置不成立：先 `cargo build --offline`（两个 crate） ==")
        return 1

    entries = load_manifest()
    files = sorted(p for p in FIXTURES.rglob("*.svg"))
    check(len(entries) == 58 and len(files) == 58,
          "语料登记：MANIFEST 58 条 <-> tests/fixtures 58 个 SVG",
          f"manifest={len(entries)} files={len(files)}")
    check(sorted(str(FIXTURES / (fid + ".svg")) for fid, _, _ in entries) == [str(p) for p in files],
          "语料登记 1:1（每条 id 都有对应文件，没有多余文件）")

    faces = sorted(p for p in FONT_DIR.rglob("*")
                   if p.is_file() and p.suffix.lower() in {".ttf", ".otf", ".ttc"}) \
        if FONT_DIR.is_dir() else []
    fonts_ok = len(faces) > 0
    if fonts_ok:
        print(f"INFO  字体目录可用：{len(faces)} 个 face（每条命令都带 `--font-dir {FONT_DIR}`）")
    else:
        skip("依赖渲染的断言（①③④ 的 PNG 部分）",
             f"字体目录不可用：{FONT_DIR}（先跑 fontlib/setup.sh，或设 $SVG_GRID_FONT_DIR）")

    # ── ① 墨水恢复（两张目标图） + 消融对照
    for fid, spec in TARGETS.items():
        print(f"\n-- {fid}: 阈值 ≥ {spec['min_ink']:.2f}；实测 {spec['measured']:.4f}；"
              f"源图 {spec['source_ref']:.4f}（cairosvg 参考值）；未修 {spec['before_fix']:.4f}")
        tagged = tmp / (fid.replace("/", "_") + ".tagged.svg")
        conv = convert(fid, tagged)
        check(conv.returncode == 0, f"{fid}：第一段 `svg_grid_convert` rc=0",
              f"rc={conv.returncode} stderr={conv.stderr[-300:]!r}")
        if not fonts_ok:
            skip(f"{fid}: 墨水 / 消融", "字体目录不可用（见上）")
            continue
        out_svg, out_png = tmp / (fid.replace("/", "_") + ".svg"), tmp / (fid.replace("/", "_") + ".png")
        got = compose(tagged, out_svg, out_png)
        check(got.returncode == 0, f"{fid}：第二段 `plot-grid` rc=0",
              f"rc={got.returncode} stderr={got.stderr[-300:]!r}")
        # ③ 干净路径零输出：修好之后这两张不该再有任何告警。
        check(got.stderr == "", f"{fid}：组图侧 stderr 0 字节（干净路径零输出）",
              f"{len(got.stderr)}B: {got.stderr[:300]!r}")
        value = ink(out_png)
        check(value >= spec["min_ink"],
              f"{fid}：墨水 {value:.4f} ≥ {spec['min_ink']:.2f}"
              f"（实测 {spec['measured']:.4f}；源图参考 {spec['source_ref']:.4f}；未修 {spec['before_fix']:.4f}）",
              f"measured={value:.4f}")
        check(value >= spec["before_fix"] + 0.2,
              f"{fid}：墨水比未修时（{spec['before_fix']:.4f}）至少多 0.2（恢复不是噪声）",
              f"now={value:.4f} before={spec['before_fix']:.4f}")

        # 消融对照：把 paint server 从 tagged 里删掉，同一渲染器、同一参数再渲 ⇒ 墨水必须塌回去。
        text = tagged.read_text()
        stripped = text
        for tag in ("linearGradient", "radialGradient", "pattern"):
            stripped = re.sub(r"<" + tag + r"\b.*?</" + tag + r">", "", stripped, flags=re.S)
        ablated = tmp / (fid.replace("/", "_") + ".ablated.svg")
        ablated.write_text(stripped)
        ab_svg, ab_png = tmp / (fid.replace("/", "_") + ".ab.svg"), tmp / (fid.replace("/", "_") + ".ab.png")
        ab = compose(ablated, ab_svg, ab_png)
        ab_value = ink(ab_png) if ab.returncode == 0 and ab_png.exists() else float("nan")
        check(ab.returncode == 0 and ab_value < spec["min_ink"],
              f"{fid}：消融对照（删掉 paint server 再渲）墨水塌回 {ab_value:.4f} < {spec['min_ink']:.2f}"
              f" —— 这些墨水确是 paint server 画的",
              f"rc={ab.returncode} ablated_ink={ab_value:.4f} stderr={ab.stderr[:200]!r}")

    if not fonts_ok:
        skip("源图墨水复核", "字体目录不可用（见上）")
    try:
        import cairosvg  # noqa: F401
        for fid, spec in TARGETS.items():
            png = tmp / (fid.replace("/", "_") + ".source.png")
            cairosvg.svg2png(url=str(FIXTURES / (fid + ".svg")), write_to=str(png),
                             output_width=900, background_color="white")
            value = ink(png)
            check(abs(value - spec["source_ref"]) < 0.05,
                  f"{fid}：源图墨水 {value:.4f} ≈ 报告记的 {spec['source_ref']:.4f}（±0.05）")
    except ImportError:
        for fid, spec in TARGETS.items():
            skip(f"{fid}: 源图墨水复核",
                 f"本机没有 cairosvg（报告里的源图值 {spec['source_ref']:.4f} 是它渲的；"
                 f"判据改用阈值 + 消融对照）")

    # ── ② paint server 一个不少 + tagged 零悬空
    print()
    for fid, spec in TARGETS.items():
        src_text = (FIXTURES / (fid + ".svg")).read_text()
        tagged_text = (tmp / (fid.replace("/", "_") + ".tagged.svg")).read_text()
        for tag in SERVERS:
            want, got = count_tag(src_text, tag), count_tag(tagged_text, tag)
            check(want == got, f"{fid}：tagged 里 `<{tag}>`×{got} 与源图一致（{want}）",
                  f"source={want} tagged={got}")
        for tag, want in spec["servers"].items():
            check(count_tag(tagged_text, tag) == want,
                  f"{fid}：`<{tag}>` 计数 = 报告记的 {want}",
                  f"tagged={count_tag(tagged_text, tag)}")
        refs, source_refs = refs_of(tagged_text), refs_of(src_text)
        check(len(refs) == len(source_refs),
              f"{fid}：tagged 的 `url(#…)` {len(refs)} 处与源图一致（{len(source_refs)}）",
              f"source={len(source_refs)} tagged={len(refs)}")
        dangling = dangling_of(tagged_text)
        check(not dangling, f"{fid}：tagged 的 `url(#…)` 零悬空（全部能解析到本文件定义的 id）",
              f"{len(dangling)} 悬空: {dangling[:5]}")
        # 参考：源图自己是自洽的（id 27/58 个，引用 91/13 处），所以上面比的是「保留」而不是「修好源图的毛病」。
        check(not dangling_of(src_text),
              f"{fid}：源图本身零悬空（参考：{len(ids_of(src_text))} 个 id / {len(source_refs)} 处引用）",
              f"{dangling_of(src_text)[:5]}")

    # ── ④ W9：悬空出声 / 干净输入零输出
    print()
    dangling_panel = tmp / "w9_dangling.svg"
    dangling_panel.write_text(
        '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>\n'
        '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#no-such-id)"/></g>\n'
        '</svg>\n')
    clean_panel = tmp / "w9_clean.svg"
    clean_panel.write_text(
        '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>\n'
        '<g data-scale="xy"><defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0"'
        ' x2="10" y2="10"><stop offset="0" stop-color="#ffffff"/></linearGradient></defs>\n'
        '<rect x="10" y="10" width="40" height="30" fill="url(#g)"/></g>\n'
        '</svg>\n')
    many_panel = tmp / "w9_many.svg"
    many_panel.write_text(
        '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>\n'
        '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" clip-path="url(#no-such-clip)"'
        ' style="fill: url(#no-such-fill)"/></g>\n'
        '</svg>\n')

    bad_out = tmp / "w9_dangling_out.svg"
    bad = compose(dangling_panel, bad_out, width="100", extra=("--height", "80", "--ncol", "1"))
    bad_stderr = bad.stderr
    lines = [line for line in bad_stderr.splitlines() if W9 in line]
    check(bad.returncode == 0, "W9：悬空 `url(#no-such-id)` 不致命（rc=0）",
          f"rc={bad.returncode} stderr={bad_stderr[:300]!r}")
    check(bad_out.is_file() and bad_out.stat().st_size > 0,
          "W9：rc=0 的同时产物照写（告警不改输出契约）",
          f"{bad_out} exists={bad_out.exists()}")
    check(len(lines) == 1, "W9：悬空引用 ⇒ 恰好一条 W9", f"{len(lines)} 条: {lines}")
    body = lines[0] if lines else ""
    check("1 reference(s) point at an id that nothing in the output defines" in body,
          "W9「是什么」：说清有几处引用指向了没人定义的 id", body[:400])
    check("input #0" in body and "<rect fill>" in body and "url(#no-such-id)" in body,
          "W9「是什么」：点名是哪一份输入、哪个元素/属性、哪个 id（input #0 <rect fill> -> url(#no-such-id)）",
          body[:400])
    check("painted with nothing" in body and "loses ink" in body,
          "W9「为什么」：解释后果（被画成空白 ⇒ 图看着完整却少了墨水）", body[:500])
    check("Fix:" in body and "<defs>" in body,
          "W9「怎么修」：给出 Fix（留着定义该 id 的 <defs>，或去掉引用）", body[:600])
    check("non-fatal warning(s)" in bad_stderr and "exit code and the output file are unchanged" in bad_stderr,
          "W9：走统一收尾（计数行 + 「rc 与产物不变」）", bad_stderr[-300:])

    many_out = tmp / "w9_many_out.svg"
    many = compose(many_panel, many_out, width="100", extra=("--height", "80", "--ncol", "1"))
    many_lines = [line for line in many.stderr.splitlines() if W9 in line]
    check(many.returncode == 0 and len(many_lines) == 1,
          "W9：同一份输入的多处悬空（clip-path + 内联 style）合成一条", f"{len(many_lines)} 条: {many_lines}")
    check("2 reference(s)" in (many_lines[0] if many_lines else "")
          and "url(#no-such-clip)" in (many_lines[0] if many_lines else "")
          and "url(#no-such-fill)" in (many_lines[0] if many_lines else ""),
          "W9：计数 2、两个 id 都点名（clip-path 与内联 style 各算一处）",
          (many_lines[0] if many_lines else "")[:500])

    clean_out = tmp / "w9_clean_out.svg"
    clean = compose(clean_panel, clean_out, width="100", extra=("--height", "80", "--ncol", "1"))
    clean_lines = [line for line in clean.stderr.splitlines() if W9 in line]
    check(clean.returncode == 0 and not clean_lines,
          "W9：干净输入（引用都能解析）⇒ 0 条 W9", f"{len(clean_lines)} 条: {clean_lines[:2]}")
    check(clean.stderr == "", "W9：干净输入 ⇒ stderr 0 字节（不是「只是没说 W9」）",
          f"{len(clean.stderr)}B: {clean.stderr[:300]!r}")

    # ── ⑤ 58 张语料全量跑两段
    print()
    stats, skipped, failing = [], [], []
    w1_total = w6_total = w9_total = 0
    for fid, expect, why in entries:
        tagged = tmp / ("corpus_" + fid.replace("/", "_") + ".tagged.svg")
        conv = convert(fid, tagged)
        if expect != "pass":
            skipped.append(fid)
            skip(f"语料 {fid}", f"MANIFEST 标 expect={expect!r}：{why[:220] or '（理由见 MANIFEST）'}"
                                 f" —— 转换器 rc={conv.returncode}（fail-closed 是正确行为，不计失败）")
            continue
        out_svg = tmp / ("corpus_" + fid.replace("/", "_") + ".svg")
        out_png = tmp / ("corpus_" + fid.replace("/", "_") + ".png")
        got = compose(tagged, out_svg, out_png)
        w1 = sum(1 for line in got.stderr.splitlines() if W1 in line)
        w6 = sum(1 for line in got.stderr.splitlines() if W6 in line)
        w9 = sum(1 for line in got.stderr.splitlines() if W9 in line)
        w1_total += w1
        w6_total += w6
        w9_total += w9
        png_ok = out_png.is_file() and out_png.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"
        ok = conv.returncode == 0 and got.returncode == 0 and out_svg.is_file() and png_ok
        stats.append((fid, conv.returncode, got.returncode, w1, w6, w9, ok))
        print(f"      {'ok  ' if ok else 'FAIL'} {fid:36s} convert={conv.returncode} plot={got.returncode} "
              f"W1={w1} W6={w6} W9={w9}")
        if not ok:
            failing.append(fid)
    checked = len(stats)
    print(f"\n      语料统计：A(两段 rc=0)={checked - len(failing)}  FAIL={len(failing)}  "
          f"B(SKIP 按 MANIFEST)={len(skipped)}  合计={checked + len(skipped)}")
    print(f"      每条 W1/W6/W9 次数（只统计，不判失败）：W1={w1_total} W6={w6_total} W9={w9_total}")
    print("      W1>0 的：" + (", ".join(f"{f}[{w1}]" for f, _, _, w1, _, _, _ in stats if w1) or "（无）"))
    print("      W6>0 的：" + (", ".join(f"{f}[{w6}]" for f, _, _, _, w6, _, _ in stats if w6) or "（无）"))
    check(len(entries) == 58 and checked + len(skipped) == 58,
          "语料：58 张全覆盖（跑通 + SKIP 都点名，没有静默漏跑）",
          f"checked={checked} skipped={len(skipped)}")
    check(not failing, "语料：除 MANIFEST 的 SKIP 外，58 张的两段 rc 都是 0",
          f"失败 {len(failing)} 张: {failing}")
    check(w9_total == 0, "语料：58 张的两段全量里没有任何 W9（没有悬空的 `url(#…)`）", f"{w9_total} 条")
    # W1 统计不能是「计数器坏了」造成的 0：同一份 tagged 强行缩到一半，必须出 W1。
    if fonts_ok and checked:
        probe_fid = "ggplot2/gradient_fill"
        probe_tagged = tmp / ("corpus_" + probe_fid.replace("/", "_") + ".tagged.svg")
        half_svg = tmp / "half.svg"
        half = compose(probe_tagged, half_svg, width="108")
        half_w1 = sum(1 for line in half.stderr.splitlines() if W1 in line)
        check(half.returncode == 0 and half_w1 >= 1,
              "语料：W1 计数器是活的（同一份 tagged 用 --width 108 重跑 ⇒ 出 W1，"
              "所以上表的 W1=0 是「没发生」而不是「没人看」）",
              f"rc={half.returncode} W1={half_w1} stderr={half.stderr[:200]!r}")
    else:
        skip("W1 计数器活性对照", "字体目录不可用或语料为空")

    # ── ⑥ 回归 R4–R15
    print()
    for round_id in ("R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R13", "R14", "R15"):
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        regression = run([sys.executable, str(evidence)])
        check(regression.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              (regression.stdout[-400:] + regression.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
