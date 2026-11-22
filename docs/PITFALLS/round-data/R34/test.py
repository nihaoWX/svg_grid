#!/usr/bin/env python3
"""R34 — W8 把 `--label-fontfamily` 当 CSS 回退链 + 硬错误行 ≤200 字符。

覆盖 D5b：
① 链首缺、链尾可用 ⇒ rc=0（旧实现 rc=3）；`font-family` 原样写出，几何与单族 Arial 一致
② 链里有通用族（serif/sans-serif/…）⇒ rc=0
③ 整条链都不可用 ⇒ rc=3，文案点名整条链 + 可用族 + 修法，且单行 ≤200
④ W3 硬错误行 ≤200（旧 205）
⑤ 无 `--labels` 的输出与 `eac0dee` 逐字节一致（≥5 个 fixture）
无参数；用 `target/debug/svg_grid`；只写系统临时目录。
"""
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
BIN = SVG_GRID / "target/debug/svg_grid"
FONT_DIR = pathlib.Path.home() / "svg_grid/fontlib"
MAX_LINE = 200
FAILS = []
SKIPS = []

PANEL = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
         'viewBox="0 0 100 80"><rect data-panel-box="main" x="0" y="0" '
         'width="100" height="80" fill="none" stroke="none"/>'
         '<circle cx="50" cy="40" r="3" fill="#000"/></svg>')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        FAILS.append(label)
        if detail:
            print("      " + str(detail).replace("\n", "\n      ")[:900])


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")
    SKIPS.append(label)


def run(args):
    return subprocess.run([str(BIN), "plot-grid", *map(str, args)],
                          capture_output=True, text=True, cwd="/tmp")


def panel(tmp, name):
    path = tmp / f"{name}.svg"
    path.write_text(PANEL)
    return path


def longest(stderr):
    return max((len(line) for line in stderr.splitlines()), default=0)


def main():
    if not BIN.exists():
        print(f"FAIL  前置：缺二进制 {BIN} —— 先 cargo build")
        return 1
    fonts = FONT_DIR.is_dir() and any(FONT_DIR.glob("*.ttf"))
    if not fonts:
        print(f"FAIL  前置：字体库 {FONT_DIR} 为空 —— 先跑 fontlib/setup.sh")
        return 1
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r34-"))
    one = panel(tmp, "one")

    # ── ① 链首缺、链尾可用
    print("== ① 回退链：链首缺、链尾可用 ⇒ rc=0 ==")
    chain = "Helvetica, Arial, sans-serif"
    out = tmp / "chain.svg"
    got = run(["--input", one, "--output", out, "--width", 600, "--ncol", 1,
               "--labels", "--label-fontfamily", chain])
    check(got.returncode == 0, "链首 Helvetica 缺、链尾 Arial 在库 ⇒ rc=0（旧实现 rc=3）",
          f"rc={got.returncode} {got.stderr[:400]}")
    check(got.stderr == "", f"满足的链 ⇒ stderr 0 字节（实得 {len(got.stderr)}B）", got.stderr[:400])
    text = out.read_text() if out.exists() else ""
    check(f'font-family="{chain}"' in text,
          "font-family 原样写出（不重写成解析后的族）", text[:200])
    arial = tmp / "arial.svg"
    run(["--input", one, "--output", arial, "--width", 600, "--ncol", 1,
         "--labels", "--label-fontfamily", "Arial"])
    def strip_family(svg):
        import re
        return re.sub(r'font-family="[^"]*"', 'font-family="X"', svg)
    check(arial.exists() and strip_family(text) == strip_family(arial.read_text()),
          "链的几何与单族 Arial 逐字节一致（只差 font-family 值）")

    for value, label in (("Liberation Sans, Arial", "链首 Liberation Sans 缺、链尾 Arial 在库"),
                         ("serif, NoSuchFont_xyz", "链里有通用族 serif"),
                         ("NoSuchFont_xyz, monospace", "链尾是通用族 monospace"),
                         ("Arial, Liberation Sans, sans-serif", "链首在库（旧实现也过）")):
        done = run(["--input", one, "--output", tmp / "ok.svg", "--width", 200, "--ncol", 1,
                    "--labels", "--label-fontfamily", value])
        check(done.returncode == 0 and done.stderr == "",
              f"② {label} ⇒ rc=0、stderr 0 字节", f"rc={done.returncode} {done.stderr[:300]}")

    # ── ③ 整条链都不可用
    print("\n== ③ 整条链都不可用 ⇒ rc=3、文案点名整条链、单行 ≤200 ==")
    for value in ("NoSuchFont_xyz",
                  "Helvetica, Liberation Sans, DejaVu Sans",
                  "A,B,C,D,E,F,G,H"):
        target = tmp / "dead.svg"
        done = run(["--input", one, "--output", target, "--width", 200, "--ncol", 1,
                    "--labels", "--label-fontfamily", value])
        check(done.returncode == 3, f"③ [{value}] ⇒ rc=3（字体错误层）", f"rc={done.returncode}")
        check(not target.exists(), f"③ [{value}] 不写产物")
        check("is not in the font library" in done.stderr,
              f"③ [{value}] 保留 doctor 的 W8 码词", done.stderr[:400])
        check("no family in the chain is usable" in done.stderr,
              f"③ [{value}] 说明是整条链不可用", done.stderr[:400])
        check("available:" in done.stderr and "--list-fonts" in done.stderr,
              f"③ [{value}] 列出可用族与修法", done.stderr[:400])
        check(longest(done.stderr) <= MAX_LINE,
              f"③ [{value}] 单行最长 {longest(done.stderr)} ≤ {MAX_LINE}", done.stderr[:400])

    # ── ④ W3 硬错误行 ≤200
    print("\n== ④ W3 硬错误行 ≤200（旧 205）==")
    row = tmp / "row.svg"
    run(["--input", one, "--output", row, "--width", 400, "--ncol", 1, "--labels", "A",
         "--plot-margin", 0])
    retag = run(["--input", row, "--output", tmp / "retag.svg", "--width", 400, "--ncol", 1,
                 "--labels", "C"])
    check(retag.returncode == 2, "W3 重复加字母 ⇒ rc=2", f"rc={retag.returncode}")
    check("already carry letters" in retag.stderr and "data-svg-grid-label" in retag.stderr,
          "W3 点名重复的字母来源", retag.stderr[:400])
    check("--labels" in retag.stderr and "Fix:" in retag.stderr, "W3 给出修法", retag.stderr[:400])
    check(longest(retag.stderr) <= MAX_LINE,
          f"W3 单行最长 {longest(retag.stderr)} ≤ {MAX_LINE}（旧 205）", retag.stderr[:400])
    plan = run(["--input", row, "--plan", "--labels", "C"])
    check(plan.returncode == 2 and plan.stdout.strip() == "",
          "W3 在 --plan 上同样 rc=2 且不打印计划", plan.stdout + plan.stderr)

    # ── ⑤ 无 --labels 的输出与 eac0dee 逐字节一致
    print("\n== ⑤ 无 --labels 的输出与 eac0dee 逐字节一致 ==")
    baseline = pathlib.Path("/tmp/svg_grid_d5/target/release/svg_grid")
    if not baseline.exists():
        skip("⑤ 与 eac0dee 逐字节对比", "缺 /tmp/svg_grid_d5 基线构建（git worktree + cargo build --release）")
    else:
        fixtures = [SVG_GRID / "tests/fig2_mixed/out/tagged" / f"{name}.svg"
                    for name in ("A_heatmap", "B_A", "B_B", "B_C", "B_D")]
        fixtures += [SVG_GRID / "tests/user_tests/2026-11-20" / f"{name}.svg"
                     for name in ("pandamap_ALB_A", "ABCDE")]
        for fixture in fixtures:
            if not fixture.exists():
                skip(f"⑤ {fixture.name}", "fixture 不存在")
                continue
            new_out, old_out = tmp / f"new_{fixture.stem}.svg", tmp / f"old_{fixture.stem}.svg"
            run(["--input", fixture, "--output", new_out, "--width", 600, "--ncol", 1])
            subprocess.run([str(baseline), "plot-grid", "--input", str(fixture),
                            "--output", str(old_out), "--width", "600", "--ncol", "1"],
                           capture_output=True, text=True, cwd="/tmp")
            same = new_out.exists() and old_out.exists() and new_out.read_bytes() == old_out.read_bytes()
            check(same, f"⑤ {fixture.name}：无 --labels 的输出与 eac0dee 逐字节一致")

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
