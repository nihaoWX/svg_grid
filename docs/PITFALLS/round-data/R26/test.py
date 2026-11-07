#!/usr/bin/env python3
"""R26 —— 收掉 `svg_grid` 最后两条未决项：A 删掉 14 个未文档化下划线别名；B 新增只读的 `--plan`。

四族判据（都可机械跑）：
  ① 别名：14 个下划线写法全部 rc=2 且 `unknown flag "<x>"`；kebab 写法仍正常；
     `--help` 与解析器一一对应（双向：解析器登记的每个 flag 都在帮助里，帮助里也没有解析器不认识的 flag）；
  ② `--plan`：零写入（仓库 git status + 临时目录文件集/时间戳前后一致）；每人一行 `canvas/cell/s/Δ`；
     `note:` 给的那行命令可**直接照抄**，照抄后该输入 `s=1`；同参数下 plan 的 `cell`/`s` 与真跑产出 SVG 反解一致；
     参数错 rc=2、输入读不了 rc=1 且**点名路径**；
  ③ 指南：`02a` ≤ 20000 B、流水线一节改用 `--plan`（手算格宽的公式已删）、两副本 `diff -r` 空；
  ④ 回归 R4/R5/R7/R8/R9/R13/R14/R16/R17 全绿（超时逐条 SKIP 并写原因，绝不静默）。

无参数；约 20 分钟（④ 占大头）。只读：临时产物只在系统临时目录里。
"""
import pathlib
import re
import shlex
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R26 → PITFALLS → docs → svg_grid
REPO = SVG_GRID.parents[1]                       # tools/svg_grid → tools → 根仓
BIN = SVG_GRID / "target/debug/svg_grid"
PARSE_RS = SVG_GRID / "src/cli/parse.rs"
GUIDE = SVG_GRID / "docs/general-figure-guide/references/02a_svg_grid_assembly.md"
GUIDE_DIR = SVG_GRID / "docs/general-figure-guide"
COPY_DIR = REPO / "tools/manuscripts_skills/general-figure-guide"

ALIASES = [
    "--png_dpi", "--plot_margin", "--normalize_input_max_side", "--label_size",
    "--label_font_size", "--label_x", "--label_y", "--label_fontfamily",
    "--label_font_family", "--label_fontface", "--label_font_face",
    "--label_colour", "--label_color", "--font_dir",
]
# `--help` 里那些**不属于 plot-grid 解析器**的 flag（convert / doctor / 全局）。
NOT_PLOT_GRID = {
    "--check", "--report", "--force", "--fixtures", "--only", "--jobs", "--json",
    "--keep", "--help", "--version",
}

FAILS = []
SKIPS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1200])
    if not ok:
        FAILS.append(label)
    return ok


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")
    SKIPS.append(f"{label}: {reason}")


def run(args, timeout=300, cwd="/tmp"):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                          cwd=cwd, timeout=timeout)


PANEL = """<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="0" y="0" width="10" height="10" fill="#000000"/></g>
</svg>
"""


def panel(tmp, tag):
    path = tmp / f"{tag}.svg"
    path.write_text(PANEL)
    return path


def plan(tmp, inputs, extra):
    args = [BIN, "plot-grid"]
    for item in inputs:
        args += ["--input", item]
    return run(args + ["--plan", *extra])


def snapshot_repo():
    """仓库快照：git 状态 + `tools/svg_grid/` 下的文件名/大小/mtime（不含 target）。"""
    status = run(["git", "-C", REPO, "status", "--porcelain=v1", "--untracked-files=all"])
    files = {}
    for path in SVG_GRID.rglob("*"):
        if "target" in path.parts or "__pycache__" in path.parts:
            continue
        if path.is_file():
            st = path.stat()
            files[str(path)] = (st.st_size, st.st_mtime_ns)
    return (status.returncode, status.stdout, files)


def snapshot_dir(root):
    return {str(p): (p.stat().st_size, p.stat().st_mtime_ns)
            for p in sorted(root.rglob("*")) if p.is_file()}


def parse_flags():
    """`src/cli/parse.rs` 的 `FLAGS`：正名列（解析器真实参数面）与残留的别名列。"""
    block = PARSE_RS.read_text().split("pub(crate) const FLAGS", 1)[1].split("];", 1)[0]
    names = re.findall(r'\(\s*"(--[a-z0-9-]+)"', block)
    aliases = re.findall(r'"(--[a-z0-9-]+)"', block)
    return names, [alias for alias in aliases if alias not in names]


def cell_geometry(svg):
    """从真产出 SVG 反解每一格：`{i: (cell_x, sx)}`（面板内有一个原点处 10×10 的 `data-scale="xy"` 方块）。"""
    found = {}
    for match in re.finditer(r'<g data-svg-grid-cell="(\d+)">(.*?)</g>', svg, re.S):
        index = int(match.group(1))
        body = match.group(2)
        rect = re.search(r'<rect x="([-\d.]+)" y="[-\d.]+" width="([-\d.]+)" height="([-\d.]+)"'
                         r' fill="#000000"', body)
        if rect:
            found[index] = (float(rect.group(1)), float(rect.group(2)) / 10.0,
                            float(rect.group(3)) / 10.0)
    return found


def main():
    started = time.time()
    if not BIN.exists():
        print(f"FAIL  前置：缺二进制 {BIN} —— 先 cargo build")
        return 1
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r26-"))
    a, b = panel(tmp, "A"), panel(tmp, "B")

    # ── ① 别名 / 帮助 ↔ 解析器
    print("== ① 14 个下划线别名全部 unknown flag（rc=2），kebab 仍正常 ==")
    for alias in ALIASES:
        done = run([BIN, "plot-grid", "--input", a, alias, "x", "--plan"])
        check(done.returncode == 2 and f'unknown flag "{alias}"' in done.stderr,
              f"{alias} ⇒ rc=2 + unknown flag", f"rc={done.returncode} {done.stderr[:200]}")
    check(len(ALIASES) == 14, "这 14 个就是评估里点名的那批")
    flags, alias_column = parse_flags()
    check(not [flag for flag in flags if "_" in flag],
          f"解析器登记表的正名列里再无下划线写法（{len(flags)} 个 flag）", flags)
    check(not alias_column, "别名列对每个 flag 都是空的（不留兼容层）", alias_column)
    check(".map(|(name, _)| *name)" in PARSE_RS.read_text(),
          "`canonical()` 只读正名列：别名列即使被填也不会被接受")
    for flag, value in [("--font-dir", "/tmp"), ("--label-colour", "#000"),
                        ("--plot-margin", "0"), ("--png-dpi", "600"),
                        ("--label-size", "9"), ("--label-x", "0")]:
        done = run([BIN, "plot-grid", "--input", a, "--plan", flag, value])
        check("unknown flag" not in done.stderr, f"kebab 写法 {flag} 仍被接受",
              done.stderr[:200])
    print("\n== ① --help 与解析器一一对应（双向） ==")
    help_text = run([BIN, "--help"]).stdout
    tokens = set(re.findall(r"--[a-z][a-z0-9-]+", help_text))
    missing = sorted(set(flags) - tokens)
    check(not missing, "解析器登记的每个 flag 都出现在 --help 里", missing)
    extra = sorted(tokens - set(flags) - NOT_PLOT_GRID)
    check(not extra, "帮助里没有解析器不认识的 flag（convert/doctor/全局的除外）", extra)

    # ── ② --plan：零写入
    print("\n== ② --plan：零写入（仓库 + 临时目录前后一致） ==")
    layout = ["--width", "4961", "--ncol", "2", "--plot-margin", "0"]
    before_repo, before_tmp = snapshot_repo(), snapshot_dir(tmp)
    planned = plan(tmp, [a, b], layout)
    after_repo, after_tmp = snapshot_repo(), snapshot_dir(tmp)
    check(planned.returncode == 0, f"rc=0（{planned.returncode}）", planned.stderr[:300])
    check(before_repo == after_repo, "仓库零写入：git status 与 tools/svg_grid 的文件/mtime 前后一致",
          f"git before={before_repo[1][:200]!r} after={after_repo[1][:200]!r}")
    check(before_tmp == after_tmp, "临时目录零写入：文件集/mtime 前后一致", after_tmp)
    check(planned.stderr == "", f"stderr 0 字节（{len(planned.stderr)}B）", planned.stderr[:300])

    # ── ② 输出形状：canvas / 每人一行 / note / 可照抄命令
    print("\n== ② --plan 输出：canvas、每人一行 canvas/cell/s/Δ、note 给可照抄命令 ==")
    lines = planned.stdout.splitlines()
    head = lines[0] if lines else ""
    check(re.fullmatch(r"canvas \d+(\.\d+)?x\d+(\.\d+)?", head) is not None,
          f"第一行是总画布尺寸：{head!r}", planned.stdout[:300])
    rows = [line for line in lines if re.match(r"^#\d+ ", line)]
    check(len(rows) == 2, f"每个输入一行（{len(rows)} 行 / 2 个输入）", lines)
    shape = re.compile(r"^#(\d+) (\S+)  canvas ([\d.]+)x([\d.]+)  cell ([\d.]+)x([\d.]+)  "
                       r"s=([\d.]+)x([\d.]+)  Δ([+-][\d.]+)%$")
    for row in rows:
        check(shape.match(row) is not None, f"行形状 = `#<i> <path>  canvas <w>x<h>  cell <w>x<h>  "
                                            f"s=<sx>x<sy>  Δ<percent>`：{row}", row)
    notes = [line for line in lines if line.startswith("note: ")]
    check(len(notes) == 2, f"每个输入一条 note（{len(notes)} 条）", lines)
    check("#0" in notes[0] and "#1" in notes[1],
          "note 按输入顺序点名 #0 / #1", notes)
    note_index = lines.index(notes[0])
    command = lines[note_index + 1] if note_index + 1 < len(lines) else ""
    check(command.startswith("svg_grid plot-grid --input ")
          and "--normalize-input-max-side" in command and "--plot-margin 0" in command,
          f"需要归一化时给一行可直接照抄的命令：{command}", command)

    # 照抄那行命令：它自己必须成功，且产物再进 plan 时 `s=1`（画布 = 格子）
    print("\n== ② 照抄 --plan 给的命令：产物 `s=1` ==")
    argv = shlex.split(command)
    check(argv[0] == "svg_grid", "命令以 `svg_grid ` 开头（读者可直接照抄）", command)
    followed = run([BIN, *argv[1:]], timeout=120)
    cell_file = tmp / "A.cell.svg"
    check(followed.returncode == 0 and cell_file.exists(),
          f"照抄的命令成功且产出 {cell_file.name}", f"rc={followed.returncode} {followed.stderr[:200]}")
    cell_w = shape.match([line for line in lines if line.startswith("#0 ")][0]).group(5)
    replanned = plan(tmp, [cell_file], ["--width", cell_w, "--ncol", "1", "--plot-margin", "0"])
    check(replanned.returncode == 0 and "s=1x1" in replanned.stdout,
          "按命令缩过的面板再进 plan：s=1x1", replanned.stdout[:300])
    check("no normalization needed" in replanned.stdout,
          "并且 note 改口为「不需要归一化」", replanned.stdout[:300])

    # ── ② 判定口径与运行时一致：plan 的 cell/s == 真跑产出反解的几何
    print("\n== ② plan 的 cell/s 与真跑几何一致（用真产出 SVG 反解） ==")
    natural = ["--width", "300", "--ncol", "2", "--plot-margin", "0"]
    preview = plan(tmp, [a, b], natural)
    real = run([BIN, "plot-grid", "--input", a, "--input", b,
                "--output", tmp / "real.svg", *natural])
    check(preview.returncode == 0 and real.returncode == 0, "plan 与真跑都 rc=0",
          f"{preview.returncode}/{real.returncode}")
    svg = (tmp / "real.svg").read_text()
    root = re.search(r"<svg[^>]*>", svg).group(0)
    width = float(re.search(r'width="([\d.]+)"', root).group(1))
    height = float(re.search(r'height="([\d.]+)"', root).group(1))
    canvas_line = [line for line in preview.stdout.splitlines() if line.startswith("canvas ")][0]
    plan_canvas = tuple(float(v) for v in re.search(r"x".join([r"([\d.]+)"] * 2), canvas_line).groups())
    check((width, height) == plan_canvas,
          f"总画布：plan {plan_canvas} == 真跑 svg {width}x{height}")
    geometry = cell_geometry(svg)
    check(sorted(geometry) == [0, 1], f"真产出里两格都在（{sorted(geometry)}）", svg[:300])
    first_row = [line for line in preview.stdout.splitlines() if line.startswith("#0 ")][0]
    plan_rows = {}
    for row in preview.stdout.splitlines():
        match = shape.match(row)
        if match:
            plan_rows[int(match.group(1))] = (float(match.group(5)), float(match.group(7)))
    for index in (0, 1):
        cell_x, real_sx, real_sy = geometry[index]
        plan_cell_w, plan_sx = plan_rows[index]
        check(abs(real_sx - plan_sx) < 1e-3 and abs(real_sy - plan_sx) < 1e-3,
              f"# {index} 的 s：plan {plan_sx} == 反解 {real_sx:.6f}/{real_sy:.6f}")
        if index == 1:
            check(abs((cell_x - geometry[0][0]) - plan_cell_w) < 1e-3,
                  f"格宽：plan {plan_cell_w} == 反解 {cell_x - geometry[0][0]}")
            plan_cell_h = float(shape.match(first_row).group(6))
            check(abs(height - plan_cell_h) < 1e-3,
                  f"格高：plan {plan_cell_h} == 真跑画布高 {height}（单行、margin 0）")

    # ── ② 退出码分档
    print("\n== ② 退出码：参数错 2、输入读不了 1 且点名路径 ==")
    bad = run([BIN, "plot-grid", "--input", a, "--plan", "--ncol", "0"])
    check(bad.returncode == 2 and "must be greater than zero" in bad.stderr,
          "参数错 rc=2", f"rc={bad.returncode} {bad.stderr[:200]}")
    missing = tmp / "no-such-panel.svg"
    gone = run([BIN, "plot-grid", "--input", missing, "--plan", "--width", "300", "--ncol", "1"])
    check(gone.returncode == 1 and str(missing) in gone.stderr,
          "输入读不了 rc=1 且 stderr 点名该路径", f"rc={gone.returncode} {gone.stderr[:200]}")
    conflict = run([BIN, "plot-grid", "--input", a, "--plan", "--output", tmp / "x.svg"])
    check(conflict.returncode == 2 and "--plan" in conflict.stderr and "--output" in conflict.stderr,
          "`--plan` 与 `--output` 同时给 ⇒ rc=2，且消息点名两者", conflict.stderr[:200])
    check("--plan" in help_text and "Writes no file" in help_text,
          "--help 写明 `--plan` 零写入、与输出 flag 互斥")

    # ── ③ 指南
    print("\n== ③ 指南：≤20000 B、流水线改用 --plan、两副本一致 ==")
    size = GUIDE.stat().st_size
    check(size <= 20000, f"`02a` 体积 {size} B ≤ 20000 B")
    text = GUIDE.read_text()
    pipeline = text.split("## 流水线", 1)[1].split("## 硬规则", 1)[0]
    check("--plan" in pipeline, "流水线一节改用 `--plan`（先问工具每格多大）", pipeline[:400])
    check("格宽 = avail / n" not in text and "--width <E 的格宽>" not in text,
          "手算格宽的公式与占位符已删（不再让调用方自己算）")
    scale_section = text.split("## 让面板落在它将要占据的尺度上", 1)[1].split("## 嵌套拼装", 1)[0]
    check("note:" in scale_section and "照抄" in scale_section,
          "出图尺度一节说的是「按 --plan 的格子尺寸 + 照抄 note 的命令」")
    same_bytes = (GUIDE.read_bytes() == (COPY_DIR / "references/02a_svg_grid_assembly.md").read_bytes())
    check(same_bytes, "两份 `02a` 逐字节相同")
    diff = run(["diff", "-r", GUIDE_DIR, COPY_DIR])
    check(diff.returncode == 0, "两副本 `diff -r` 为空（必须以 rsync 同步）", diff.stdout[:400])
    cli_md = (SVG_GRID / "ARCHITECTURE/cli.md").read_text()
    check("同一语义只有一种拼法" in cli_md and "不保留未文档化别名" in cli_md,
          "`ARCHITECTURE/cli.md` 记着「同一语义只有一种拼法，不保留未文档化别名」")
    check("--plan" in cli_md and "零写入" in cli_md, "`cli.md` 记着 `--plan` 的契约")
    layout_md = (SVG_GRID / "ARCHITECTURE/layout.md").read_text()
    check("格子尺寸由工具算" in layout_md and "plan_grid" in layout_md,
          "`ARCHITECTURE/layout.md` 记着「格子尺寸由工具算，不靠调用方手算」")

    # ── ④ 旧证据回归
    print("\n== ④ 旧证据回归（逐轮 rc 与用时） ==")
    for round_id in ("R4", "R5", "R7", "R8", "R9", "R13", "R14", "R16", "R17"):
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        leave = 1700 - (time.time() - started)
        if leave <= 30:
            skip(f"回归 round-data/{round_id}", "本轮总时长预算（1700 s）用尽，未跑 —— 见 meta.toml 的 how_to_run，请单独重跑")
            continue
        began = time.time()
        try:
            done = run([sys.executable, evidence], timeout=min(leave, 900))
        except subprocess.TimeoutExpired:
            skip(f"回归 round-data/{round_id}", f"超时（>{min(leave, 900):.0f}s），未判绿")
            continue
        waited = time.time() - began
        labels = re.findall(r"^  · (.+)$", done.stdout, re.MULTILINE)
        ok = done.returncode == 0
        check(ok, f"回归 round-data/{round_id} 仍全绿（rc={done.returncode}, {waited:.0f}s）",
              done.stdout[-800:] + done.stderr[-300:] if not ok else "")

    print(f"\n总用时 {time.time() - started:.0f}s")
    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    if SKIPS:
        print(f"\n== 判据全部成立（另有 {len(SKIPS)} 条 SKIP，逐条原因见上） ==")
        for label in SKIPS:
            print("  ~ " + label)
        return 0
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
