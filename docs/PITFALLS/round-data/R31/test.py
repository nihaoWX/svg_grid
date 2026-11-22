#!/usr/bin/env python3
"""R31 的可跑证据 —— **网格上界校验 + `--verbose` 默认静音**。

A. 显式网格 `(rows-1)*cols < count <= rows*cols`：违反下界（最后一行整行全空）
   必须 rc=2，且 `--plan` 与真跑报同一句可行动的错；短末行、补齐 NULL、默认
   近方形网格不得误报。
B. `--verbose`：默认成功路径 stderr 恰为 0 字节（W1–W9 + `note:` 全静音）；
   带 `--verbose` 恢复原告警；`--plan` 的 stdout `note:` 契约不变；convert
   子命令同语义。

无参数；秒级；离线；只写临时目录。找不到二进制时打 SKIP 并以 1 退出。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]  # round-data/R31 → PITFALLS → docs → svg_grid
BIN = next(
    (REPO / rel for rel in ("target/release/svg_grid", "target/debug/svg_grid")
     if (REPO / rel).exists()),
    None,
)
FAILS = []

PANEL = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">'
    '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>'
    '<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>'
    "</svg>"
)

SVGLITE = """<svg xmlns='http://www.w3.org/2000/svg' width='288pt' height='216pt' viewBox='0 0 288 216'>
<defs><style type='text/css'><![CDATA[ .svglite circle { fill: none; } ]]></style></defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; fill: #CFA15A;'/>
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</svg>
"""


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def run(args):
    return subprocess.run([str(BIN), *args], capture_output=True, text=True)


def inputs(paths, extra=(), output=None, plan=False):
    args = ["plot-grid"]
    for path in paths:
        args += ["--input", str(path)]
    args += ["--width", "1000", "--plot-margin", "0", *extra]
    if output is not None:
        args += ["--output", str(output)]
    if plan:
        args.append("--plan")
    return args


def cells_of(proc):
    return re.findall(r"cell ([\d.]+)x([\d.]+)", proc.stdout)


def main():
    if BIN is None:
        print("SKIP  no target/{release,debug}/svg_grid binary")
        return 1

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r31_"))
    panels = []
    for index in range(9):
        path = tmp / f"p{index}.svg"
        path.write_text(PANEL)
        panels.append(path)

    # ── A1. 必须触发：最后一行整行全空
    # (输入数, 显式网格, 最小可用的 --nrow, 需要补的 NULL 数)
    cases = [
        (5, ["--nrow", "4"], 3, 3),
        (5, ["--ncol", "3", "--nrow", "3"], 2, 4),
        (6, ["--ncol", "3", "--nrow", "3"], 2, 3),
        (3, ["--ncol", "3", "--nrow", "3"], 1, 6),
    ]
    for index, (count, extra, min_rows, missing) in enumerate(cases):
        out = tmp / f"empty_{index}.svg"
        real = run(inputs(panels[:count], extra, out))
        text = real.stderr
        check(real.returncode == 2, f"{count} 图 {' '.join(extra)} ⇒ rc=2", text)
        check(not out.exists(), f"{count} 图 {' '.join(extra)} ⇒ 不写输出")
        check("grid leaves the last row empty" in text,
              f"{count} 图 {' '.join(extra)} ⇒ 指出空行", text)
        check(f"reduce --nrow to {min_rows}" in text,
              f"{count} 图 {' '.join(extra)} ⇒ 给出 --nrow {min_rows}", text)
        check(f"add {missing} `--input NULL`" in text,
              f"{count} 图 {' '.join(extra)} ⇒ 给出补 {missing} 个 NULL", text)
        plan = run(inputs(panels[:count], extra, plan=True))
        check(plan.returncode == 2 and plan.stderr == real.stderr,
              f"{count} 图 {' '.join(extra)} ⇒ --plan 同样 rc=2 且同一句", plan.stderr)

    # ── A2. 不得误报：短末行、自动行数、默认近方形、显式 NULL 补齐
    good = [
        (5, ["--ncol", "2"]),
        (4, ["--ncol", "3", "--nrow", "2"]),
        (7, ["--ncol", "3", "--nrow", "3"]),
        (5, ["--ncol", "3"]),
    ]
    for count, extra in good:
        out = tmp / f"good_{count}_{'_'.join(extra)}.svg"
        proc = run(inputs(panels[:count], extra, out))
        check(proc.returncode == 0 and out.exists(),
              f"{count} 图 {' '.join(extra)} ⇒ rc=0（合法短末行）", proc.stderr)

    padded = [*panels[:5], "none", "none", "none"]
    out = tmp / "padded.svg"
    proc = run(inputs(padded, ["--ncol", "4", "--nrow", "2"], out))
    check(proc.returncode == 0 and out.exists(),
          "5 图 + 3 个显式 NULL 补成 4×2 ⇒ rc=0", proc.stderr)

    for count in range(1, 10):
        proc = run(inputs(panels[:count], plan=True))
        check(proc.returncode == 0, f"默认网格 {count} 图 ⇒ rc=0（永不空行）", proc.stderr)

    # ── B1. 默认静音 / --verbose 恢复（2 图 --ncol 1：1000×400 格子对 100×80 面板）
    silent = tmp / "silent.svg"
    quiet = run(inputs(panels[:2], ["--ncol", "1"], silent))
    check(quiet.returncode == 0, "2 图 --ncol 1 默认 ⇒ rc=0", quiet.stderr)
    check(quiet.stderr == "", "默认成功路径 stderr 恰为 0 字节", quiet.stderr)

    loud = tmp / "loud.svg"
    verbose = run(inputs(panels[:2], ["--ncol", "1"], loud, plan=False)
                  + ["--verbose"])
    check(verbose.returncode == 0, "带 --verbose ⇒ rc=0", verbose.stderr)
    check("scaled by the layout" in verbose.stderr,
          "--verbose 恢复 W1 告警", verbose.stderr)
    check("non-fatal warning(s)" in verbose.stderr,
          "--verbose 恢复告警汇总行", verbose.stderr)
    check(silent.read_bytes() == loud.read_bytes(),
          "静音/verbose 的输出字节相同（诊断不影响产物）")

    # ── B2. --plan 的 stdout note: 不受 --verbose 影响
    plan_quiet = run(inputs(panels[:3], ["--ncol", "3"], plan=True))
    plan_loud = run(inputs(panels[:3], ["--ncol", "3"], plan=True) + ["--verbose"])
    check(plan_quiet.returncode == plan_loud.returncode == 0,
          "--plan 两种模式都 rc=0")
    check("note:" in plan_quiet.stdout, "--plan 的 stdout 仍有 note: 契约行",
          plan_quiet.stdout)
    check(plan_quiet.stderr == "", "--plan 默认 stderr 为 0 字节", plan_quiet.stderr)
    check(plan_quiet.stdout == plan_loud.stdout,
          "--plan stdout 与 --verbose 无关", plan_loud.stdout)

    # ── B3. convert 同语义
    svglite = tmp / "svglite.svg"
    svglite.write_text(SVGLITE)
    conv_quiet = run(["convert", "-i", str(svglite), "-o", str(tmp / "c1.svg")])
    check(conv_quiet.returncode == 0 and conv_quiet.stderr == "",
          "convert 默认成功路径 stderr 为 0 字节", conv_quiet.stderr)
    conv_loud = run(["convert", "-i", str(svglite), "-o", str(tmp / "c2.svg"), "--verbose"])
    check(conv_loud.returncode == 0 and "warning:" in conv_loud.stderr,
          "convert --verbose 恢复告警", conv_loud.stderr)

    print()
    if FAILS:
        print(f"== {len(FAILS)} FAIL ==")
        for label in FAILS:
            print("  -", label)
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
