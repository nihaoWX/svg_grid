#!/usr/bin/env python3
"""R33 — `--labels` NULL 规则 / W3 W8 升硬错 / margin 0 字母内移。

覆盖 D5 的 A–D，并钉住 D5 带来的体积回退（R8 的 20000 B 约束）。
无参数；使用 `target/debug/svg_grid`；不写仓库。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
REPO = HERE.parents[5]
BIN = SVG_GRID / "target/debug/svg_grid"
FAILS = []
PANEL = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
         'viewBox="0 0 100 80"><rect data-panel-box="main" x="0" y="0" '
         'width="100" height="80" fill="none" stroke="none"/>'
         '<circle cx="50" cy="40" r="3" fill="#000"/></svg>')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        FAILS.append(label)
        if detail:
            print("      " + str(detail).replace("\n", "\n      ")[:800])


def run(args):
    return subprocess.run([str(BIN), "plot-grid", *map(str, args)],
                          capture_output=True, text=True)


def panel(tmp, name):
    p = tmp / f"{name}.svg"
    p.write_text(PANEL)
    return p


def labels_in(svg):
    found = {}
    for m in re.finditer(r'<text data-svg-grid-label="(\d+)"[^>]*\by="([^"]+)"[^>]*>([^<]*)</text>', svg):
        found[int(m.group(1))] = (m.group(3), float(m.group(2)))
    return found


def main():
    if not BIN.exists():
        print(f"SKIP: {BIN} missing (cargo build --offline)")
        return 0
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r33-"))
    panels = [panel(tmp, f"p{i}") for i in range(5)]
    out = tmp / "auto.svg"

    # A1: bare --labels leaves NULL unlettered and does not consume a letter.
    args = []
    for p in panels[:2] + [None] + panels[2:4]:
        args += ["--input", "NULL" if p is None else p]
    args += ["--output", out, "--width", 500, "--height", 300, "--ncol", 2, "--labels"]
    r = run(args)
    svg = out.read_text() if out.exists() else ""
    got = labels_in(svg)
    check(r.returncode == 0, "A1 bare --labels rc=0", r.stderr)
    actual = {i: text for i, (text, _) in got.items() if i < 5 and text}
    check(actual == {0: "A", 1: "B", 3: "C", 4: "D"},
          "A1 AUTO 跳过 NULL（A,B,–,C,D）", got)

    # A2: explicit list consumes NULL slots; the letter lands in the empty cell.
    out2 = tmp / "explicit.svg"
    args = [x for x in args]
    args[args.index("--labels") + 0:args.index("--labels") + 1] = ["--labels", "A,B,X,,E"]
    args[args.index("--output") + 1] = out2
    r = run(args)
    svg = out2.read_text() if out2.exists() else ""
    got = labels_in(svg)
    check(r.returncode == 0, "A2 explicit --labels rc=0", r.stderr)
    check(got.get(2, ("",))[0] == "X" and 3 not in got and got.get(4, ("",))[0] == "E",
          "A2 显式列表按槽位：NULL 槽画 X、空串槽跳过", got)

    # B: W3 is a hard error, in both the run and --plan.
    row = tmp / "row.svg"
    row_args = []
    for p in panels[:2]:
        row_args += ["--input", p]
    row_args += ["--output", row, "--width", 400, "--height", 160, "--ncol", 2,
                 "--labels", "A,B", "--plot-margin", 0]
    r = run(row_args)
    check(r.returncode == 0 and row.exists(), "B 前置：先组一行带字母的图", r.stderr)

    retag = tmp / "retag.svg"
    r = run(["--input", row, "--output", retag, "--width", 400, "--height", 160,
             "--ncol", 1, "--labels", "C"])
    check(r.returncode == 2 and not retag.exists(), "B 重复加字母 ⇒ rc=2、无输出", r.stderr)
    check('input #0 (letter "C")' in r.stderr and "data-svg-grid-label" in r.stderr
          and "Fix:" in r.stderr, "B 文案点名输入与字母、给修法", r.stderr)
    plan = run(["--input", row, "--plan", "--labels", "C"])
    check(plan.returncode == 2 and plan.stdout.strip() == "",
          "B --plan 同样 rc=2 且不打印计划", plan.stdout + plan.stderr)
    skip = run(["--input", row, "--output", tmp / "skip.svg", "--width", 400,
                "--height", 160, "--ncol", 1, "--labels", ",C"])
    check(skip.returncode == 0, "B 空串槽不冲突，仍可运行", skip.stderr)

    # C: W8 is a font error for SVG and PNG; --plan stays font-free.
    for extra, output in (([], tmp / "w8.svg"), (["--output-png", tmp / "w8.png"], tmp / "w8.png")):
        r = run(["--input", panels[0], "--output", tmp / "w8.svg", *extra,
                 "--width", 200, "--height", 160, "--labels", "A",
                 "--label-fontfamily", "NoSuchFont_xyz"])
        check(r.returncode == 3 and not output.exists(),
              f"C 缺字体族 ⇒ rc=3、无产物（{output.suffix or '.svg'}）", r.stderr)
        check("NoSuchFont_xyz" in r.stderr and "available:" in r.stderr
              and "--list-fonts" in r.stderr, "C 文案点名缺族、可用族与修法", r.stderr)
    plan = run(["--input", panels[0], "--plan", "--labels", "A",
                "--label-fontfamily", "NoSuchFont_xyz"])
    check(plan.returncode == 0, "C --plan 不读字体，保持成功", plan.stderr)

    # D: --plot-margin 0 moves every letter inside the canvas.
    out3 = tmp / "margin.svg"
    args = []
    for p in panels:
        args += ["--input", p]
    args += ["--output", out3, "--width", 500, "--height", 160, "--ncol", 5,
             "--plot-margin", 0, "--labels", "--verbose"]
    r = run(args)
    svg = out3.read_text() if out3.exists() else ""
    got = labels_in(svg)
    check(r.returncode == 0 and len(got) == 5, "D margin 0 五图五字母齐全", (r.stderr, got))
    check(all(y >= 0 for _, y in got.values()), "D 所有字母 y ≥ 0（不再被画布裁掉）", got)
    check("was moved inside the canvas" in r.stderr, "D W5 说明字母已内移", r.stderr)

    # Collateral: D5 must not push a source file back over R8's 20000 B ceiling.
    over = []
    for base in ("src", "convert/src", "tests", "scripts"):
        for p in (REPO / base).rglob("*"):
            if p.is_file() and p.suffix in (".rs", ".py") and p.stat().st_size > 20000:
                over.append(f"{p.relative_to(REPO)}={p.stat().st_size}B")
    check(not over, "体积 D5 改动后仍 ≤ 20000 B", over)
    check(not any("include!(" in p.read_text(errors="ignore")
                  for p in (REPO / "src").rglob("*.rs")),
          "模块树 不含 include!( )")

    print(f"\nR33: {len(FAILS)} failure(s)")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
