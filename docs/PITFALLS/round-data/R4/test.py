#!/usr/bin/env python3
"""R4 的可跑证据（v7 round-data）。

盯两族**复发**（按 `ARCHITECTURE/warnings.md` 的 W6 与 `ARCHITECTURE/check-deliverable.md`）：

**族一「内容越出自身画布」**（`R3-F8` → `F2` → `G3` → `A1`，两天 4 次）
  → 工具侧解药 = W6 告警。断言：**旋转 y 轴标签画到画布外时必须出声**（这正是 A1 的旗舰案例；
    旧版 `ink.rs` 忽略 `transform`，对旋转文本完全无效），且把它挪回画布内时必须**静默**。

**族二「死白带判据误报」**（`R3-F10` → `G2`）
  → 解药 = 判据修正后落成唯一脚本 `tools/svg_grid/scripts/check_deliverable.py`。断言：
    通栏 + 右侧图例的版面**不再**被判死带；**真死带仍然要报**。

无参数；秒级；离线；不写仓库（fixture 在临时目录自合成，故本目录**没有 `data/`**）。
契约：`tools/pitfalls-manager/references/02_consume.md` §4.3（退出码即结论 / 每族一行 / 只有 FAIL 才给细节 / 显式 skip）。
"""
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                    # round-data/R4 → PITFALLS → docs → svg_grid
SCRIPTS = SVG_GRID / "scripts"
FAILS = []

# A1 的真实坐标（取自 research/…/出图/S39_S40/figure_panels/S39_panelC_depth_v1.svg）
V1_X = "-1.684688"
FIXED_X = "11.747625"

PANEL_TMPL = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 528 187.2" width="528" height="187.2">
  <rect data-panel-box="main" x="18.48" y="11.232" width="417.12" height="147.888" fill="none" stroke="none"/>
  <rect x="18.48" y="11.232" width="417.12" height="147.888" fill="#ffffff" stroke="#20303a" stroke-width="0.5"/>
  <text style="font-size: 7.2px; font-family: DejaVu Serif, serif; text-anchor: middle; fill: #20303a" x="{x}" y="85.176" transform="rotate(-90 {x} 85.176)">NES</text>
  <text style="font-size: 6.4px; font-family: DejaVu Serif, serif; text-anchor: middle; fill: #20303a" x="227" y="180">depth slope</text>
</svg>
"""


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + detail.replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def find_bin():
    for rel in ("target/debug/svg_grid", "target/release/svg_grid"):
        p = SVG_GRID / rel
        if p.exists():
            return p
    return None


def run_plot_grid(binary, svg, tmp):
    out = pathlib.Path(tmp) / "out.svg"
    p = subprocess.run([str(binary), "plot-grid", "--input", str(svg), "--output", str(out),
                        "--ncol", "1", "--plot-margin", "24", "--width", "4400"],
                       capture_output=True, text=True)
    return p


def family_one(tmp):
    binary = find_bin()
    if binary is None:
        print("SKIP  族一：没找到 svg_grid 二进制 —— 先 `cd tools/svg_grid && cargo build --offline`（这是环境问题，不是行为差异）")
        return
    print(f"-- 族一：内容越出自身画布（W6）；二进制 {binary.relative_to(SVG_GRID)}")
    for tag, x in (("v1（越界，A1 原样坐标）", V1_X), ("修好版（挪回画布内）", FIXED_X)):
        svg = pathlib.Path(tmp) / f"panel_{tag[0]}.svg"
        svg.write_text(PANEL_TMPL.format(x=x))
        p = run_plot_grid(binary, svg, tmp)
        w6 = [l for l in p.stderr.splitlines() if "draws ink outside its own canvas box" in l]
        if tag.startswith("v1"):
            check(p.returncode == 0 and any("left by" in l for l in w6),
                  "族一 旋转 ylabel 越出左缘 ⇒ W6 必须出声（含 left by）",
                  f"rc={p.returncode} W6={w6!r} stderr={p.stderr[:400]!r}")
        else:
            check(p.returncode == 0 and not w6,
                  "族一 同一标签挪回画布内 ⇒ 该元素不再触发 W6（负向对照）",
                  f"rc={p.returncode} W6={w6!r}")


def family_two(tmp):
    if not (SCRIPTS / "check_deliverable.py").exists():
        print("SKIP  族二：没找到 scripts/check_deliverable.py（判据的唯一来源）")
        return
    sys.path.insert(0, str(SCRIPTS))
    import numpy as np
    import check_deliverable as cd

    print("-- 族二：死白带判据（通栏 + 右图例不得误报；真死带仍须报）")
    blank, ink = 255, 0

    # 通栏版面：4400 宽面板 + 右侧大留白（页边距）+ 顶部字母带 —— 修复前必然误报
    a = np.full((3480, 4448), blank, np.uint8)
    a[126 + 24:126 + 24 + 1400, 100:3690] = ink          # 面板内容（右侧留 ~660 px 页边距）
    a[40:100, 40:80] = ink                               # 面板字母（住在 margin 里）
    r = cd.check_dead_band(a, margin=24, label_band=126, allow_gap=True)
    check(r["status"] == "pass", "族二 通栏面板 + 右侧图例/页边距 ⇒ 不判死带（G2 的误报已消）", f"{r}")

    # 真死带：中间整行无墨（面板缺失的样子）。判据只统计**完整窗口**（既有口径），故刻意让
    # 空带对齐一个 200 px 窗口（行 1224 起、宽 400 px）——这是该判据的已知边界，不是本次要改的东西。
    b = np.full((3480, 4448), blank, np.uint8)
    b[24:1224, 24:4424] = ink
    b[1624:3480, 24:4424] = ink
    r = cd.check_dead_band(b, margin=24, label_band=0, allow_gap=False)
    check(r["status"] == "fail", "族二 真死带 ⇒ 仍然要报（负向对照）", f"{r}")


def family_two_real(tmp):
    """真实成品（存在就顺带跑一次，这才是端到端；不在就显式 skip）。"""
    real = (SVG_GRID.parent.parent / "research/白藜芦醇胰腺癌/steps/10_下游表型/出图/成品")
    if not real.is_dir():
        print("SKIP  族二(真实成品)：research 下那份成品目录不在")
        return
    picks = [("Figure S39.png", 24), ("Figure S40.png", 24), ("Figure 7.png", 150)]
    got = [(n, m) for n, m in picks if (real / n).exists()]
    if not got:
        print("SKIP  族二(真实成品)：三张成品都不在")
        return
    for name, margin in got:
        p = subprocess.run([sys.executable, str(SCRIPTS / "check_deliverable.py"),
                            str(real / name), "--margin", str(margin)],
                           capture_output=True, text=True)
        check(p.returncode == 0,
              f"族二(真实成品) {name} --margin {margin} ⇒ 两条检查全过（2026-09-28 R6 起脚本删掉了恒真的那条）",
              p.stdout[-600:] + p.stderr[-300:])


def main():
    tmp = tempfile.mkdtemp(prefix="r4-rounddata-")
    try:
        family_one(tmp)
        family_two(tmp)
        family_two_real(tmp)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
