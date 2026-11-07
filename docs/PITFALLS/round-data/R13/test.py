#!/usr/bin/env python3
"""R13 的可跑证据 —— **字体库 `fontlib/`：来源唯一 + 默认 Arial + 缺族出声**。

盯四族（都是"成品取决于环境而不是资产"）：

1. **字体来源唯一**：`src/` 里**不再有** `load_system_fonts`；resvg 的 `system-fonts` feature 已去掉
   （连"顺手读系统字体"的路也断掉）。
2. **默认族 = Arial**：不给 `--label-fontfamily` 时，字母 `<text>` 上写的是 `font-family="Arial"`；
   `--list-fonts` 至少列出 Arial（库里实际可用的族）。
3. **缺族出声（W8）**：请求库里没有的族 ⇒ **非致命警告**（含 `Fix:`）且 rc=0、产物照常生成。
4. **空库 fail-closed**：`--font-dir` 指向空目录且**真的要字体**（要 PNG 或要加字母）⇒ 非零退出；
   纯矢量且不画字母 ⇒ 不拦。**正常路径 stderr 0 字节**。

外加：两副本 `diff -r` 为空；回归 R4–R11。

契约见 `tools/svg_grid/ARCHITECTURE/fonts.md`。无参数；秒级；离线；只读（fixture 在临时目录自合成）。
"""
import os
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]          # round-data/R13 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]              # → bioagentforge（根仓库）
BIN = SVG_GRID / "target/debug/svg_grid"
# R14 起字体库在**运行时**解析：`--font-dir` > `$SVG_GRID_FONT_DIR` > `$HOME/svg_grid/fontlib`
# （仓库里的 fontlib/ 只剩 README + setup.sh）。所以这里判"有没有字体"要跟二进制读同一个目录。
FONTLIB = (pathlib.Path(os.environ["SVG_GRID_FONT_DIR"]) if os.environ.get("SVG_GRID_FONT_DIR")
           else pathlib.Path.home() / "svg_grid" / "fontlib")
GUIDE = SVG_GRID / "docs/general-figure-guide"
COPY = REPO / "tools/manuscripts_skills/general-figure-guide"
FAILS = []

SVG = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">'
       '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'
       '</svg>')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def run(args):
    return subprocess.run([str(BIN), "plot-grid"] + args, capture_output=True, text=True)


def main():
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r13-rounddata-"))
    panel = tmp / "panel.svg"
    panel.write_text(SVG)
    has_fonts = BIN.exists() and FONTLIB.is_dir() and any(FONTLIB.glob("*.tt[fc]"))

    # ── 1. 字体来源唯一（纯源码/清单检查，不需要构建）
    src = "\n".join(p.read_text() for p in (SVG_GRID / "src").rglob("*.rs"))
    check("load_system_fonts" not in src, "来源 源码里不再有 `load_system_fonts`")
    cargo = (SVG_GRID / "Cargo.toml").read_text()
    check('"system-fonts"' not in cargo, "来源 Cargo.toml 不再启用 resvg 的 `system-fonts` feature")

    # ── 2/3/4：需要二进制与已填充的 fontlib
    if not has_fonts:
        print(f"SKIP  字体行为断言：没有 target/debug/svg_grid 或字体库 {FONTLIB} 为空"
              "（先 cargo build --offline && fontlib/setup.sh；数据根默认 $HOME/svg_grid/fontlib）"
              "—— 跳过原因已打印，不是静默跳过")
    else:
        out = tmp / "o.svg"
        p = run(["--input", str(panel), "--output", str(out), "--labels", "A", "--width", "200"])
        text = out.read_text() if out.exists() else ""
        check(p.returncode == 0 and 'font-family="Arial"' in text,
              '默认族 不给 --label-fontfamily ⇒ 字母写字为 font-family="Arial"',
              f"rc={p.returncode} {text[:300]} {p.stderr[:200]}")

        lf = run(["--list-fonts"])
        families = [l.strip() for l in lf.stdout.splitlines() if l.strip()]
        check(lf.returncode == 0 and any(f == "Arial" for f in families),
              f"--list-fonts 列出库里可用族（{len(families)} 个，含 Arial）", lf.stdout[:300] + lf.stderr[:200])

        bad = tmp / "w8.svg"
        p = run(["--input", str(panel), "--output", str(bad), "--labels", "A", "--width", "200",
                 "--label-fontfamily", "NoSuchFont_xyz"])
        check(p.returncode == 0 and "not in the font library" in p.stderr and "Fix:" in p.stderr,
              "W8 缺族 ⇒ 非致命警告（含 Fix:）、rc=0、产物照常生成",
              f"rc={p.returncode} stderr={p.stderr[:400]}")
        check(bad.exists(), "W8 缺族时产物仍然生成")

        empty = tmp / "emptyfonts"
        empty.mkdir(exist_ok=True)
        p = run(["--input", str(panel), "--output", str(tmp / "e1.svg"), "--output-png", str(tmp / "e1.png"),
                 "--font-dir", str(empty)])
        check(p.returncode != 0 and "font library" in p.stderr,
              "空库 要出 PNG 且字体库为空 ⇒ 非零退出（配置错，不许静默出白图）",
              f"rc={p.returncode} stderr={p.stderr[:300]}")
        p = run(["--input", str(panel), "--output", str(tmp / "e2.svg"), "--labels", "A",
                 "--font-dir", str(empty)])
        check(p.returncode != 0, "空库 要画字母 ⇒ 非零退出", f"rc={p.returncode} stderr={p.stderr[:300]}")
        p = run(["--input", str(panel), "--output", str(tmp / "e3.svg"), "--font-dir", str(empty)])
        check(p.returncode == 0, "空库 纯矢量且不画字母 ⇒ 不拦（字体只在读者端解析）",
              f"rc={p.returncode} stderr={p.stderr[:300]}")

        # 干净路径：画布宽 = 面板宽 + 2×margin（24）⇒ 格子恰好 200、sx=sy=1 ⇒ stderr 必须 0 字节
        p = run(["--input", str(panel), "--output", str(tmp / "ok.svg"), "--output-png", str(tmp / "ok.png"),
                 "--width", "248", "--plot-margin", "24", "--png-dpi", "300"])
        check(p.returncode == 0 and p.stderr == "",
              "干净路径（不画字母）stderr 0 字节",
              f"rc={p.returncode} stderr={p.stderr[:400]!r}")
        p = run(["--input", str(panel), "--output", str(tmp / "ok2.svg"), "--output-png", str(tmp / "ok2.png"),
                 "--labels", "A", "--width", "248", "--plot-margin", "24", "--png-dpi", "300"])
        check(p.returncode == 0 and p.stderr == "",
              "干净路径（画字母、默认族在库里）stderr 0 字节：不许有字体告警，也不许有裁切告警",
              f"rc={p.returncode} stderr={p.stderr[:400]!r}")

    # ── 两副本
    d = subprocess.run(["diff", "-r", str(GUIDE), str(COPY)], capture_output=True, text=True)
    check(d.returncode == 0, "指南两副本逐字节一致（diff -r 为空）", d.stdout[:400])

    # ── 回归
    for r in ("R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11"):
        t = SVG_GRID / "docs/PITFALLS/round-data" / r / "test.py"
        if not t.exists():
            print(f"SKIP  回归 round-data/{r}：没有 test.py")
            continue
        p = subprocess.run([sys.executable, str(t)], capture_output=True, text=True)
        check(p.returncode == 0, f"回归 round-data/{r} 仍全绿", (p.stdout[-300:] + p.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
