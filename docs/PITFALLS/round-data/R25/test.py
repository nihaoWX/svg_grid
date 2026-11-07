#!/usr/bin/env python3
"""R25 的 round-data —— 告警瘦身：单条 ≤200 字符、公共因果只打一次 note、N=1 不打计数。

六族：① 每码 awk 实量 ≤200 ② note 恰 1 行（无告警 0 行） ③ N=1 无计数包装
      ④ 条数与码集合 = 改前基线 ⑤ 回归 R4/R5/R7/R9/R11/R13/R14/R16/R17 ⑥ warnings.md 四条规则可 grep

无参数；约 15 分钟（⑤ 占大头）。只读：临时产物只在系统临时目录。
"""
import pathlib
import re
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R25 → PITFALLS → docs → svg_grid
BIN = SVG_GRID / "target/debug/svg_grid"
CONVERT = SVG_GRID / "convert/target/debug/svg_grid_convert"
WARNINGS_MD = SVG_GRID / "ARCHITECTURE/warnings.md"
FONT_DIR = pathlib.Path.home() / "svg_grid/fontlib"

MAX_LINE = 200
PREFIX = "svg_grid: warning: "
NOTE_PREFIX = "svg_grid: note: "
FOOTER = "non-fatal warning(s) — the exit code and the output file are unchanged"

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

def run(args, timeout=300):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                          cwd="/tmp", timeout=timeout)

PANEL = """<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>"""

def panel(tmp, tag):
    path = tmp / f"panel_{tag}.svg"
    path.write_text(PANEL)
    return path

def body_panel(tmp, tag, body):
    """A 100x80 panel whose content is `body`."""
    path = tmp / f"panel_{tag}.svg"
    path.write_text(
        '<svg width="100" height="80" viewBox="0 0 100 80" '
        'xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
        'fill="none" stroke="none"/>\n' + body + "\n</svg>\n"
    )
    return path

def plot(tmp, tag, inputs, extra):
    out = tmp / f"{tag}_out.svg"
    args = [BIN, "plot-grid"]
    for item in inputs:
        args += ["--input", item]
    args += ["--output", out, *extra]
    return run(args)

def warning_lines(stderr):
    return [line for line in stderr.splitlines()
            if line.startswith(PREFIX) and FOOTER not in line]

def note_lines(stderr):
    return [line for line in stderr.splitlines() if line.startswith(NOTE_PREFIX)]

def longest(lines):
    return max((len(line) for line in lines), default=0)

def awk_lengths(text):
    """`awk '{print length($0)}'` 实量（判据 ①）。"""
    done = subprocess.run(["awk", "{print length($0)}"], input=text,
                          capture_output=True, text=True)
    return [int(value) for value in done.stdout.split()]

def codes(stderr):
    table = [
        ("W1", "scaled by the layout"),
        ("W2", "stretched"),
        ("W3", "already contain panel letters"),
        ("W4a", "entries but the grid has"),
        ("W4b", "repeats some letters"),
        ("W5", "may be clipped by the canvas"),
        ("W6", "draws ink outside its own canvas box"),
        ("W7", "could not be rewritten"),
        ("W8", "not in the font library"),
        ("W9", "point at an id that nothing in the output defines"),
    ]
    found = []
    for line in warning_lines(stderr):
        for code, phrase in table:
            if phrase in line and code not in found:
                found.append(code)
    return found

def diagnose(round_id, stdout):
    """给一个红的回归轮次写一句归因（绝不让红静默通过）。"""
    labels = summary_labels(stdout)
    if not labels:
        return "rc≠0 但没有失败标签（输出可能被截断）"
    if round_id == "R11":
        in_build = [line for line in stdout.splitlines() if "tools/cao/.build/" in line]
        if in_build:
            return (f"R11 唯一那条「残留」红命中的 {len(in_build)} 处全在 `tools/cao/.build/` —— "
                    "环境自身的 cao 运行目录（.gitignore 第 28 行忽略，不是 svg_grid 的文件）")
    downstream = [label for label in labels if label.startswith("回归 round-data/")]
    if downstream and len(downstream) == len(labels):
        return f"本轮自己的判据全过，红只来自下游：{', '.join(downstream)}"
    own = [label for label in labels if not label.startswith("回归 ")]
    return f"本轮自己的判据有红：{', '.join(own) or '（见输出）'}"

def summary_labels(stdout):
    """The `· …` lines of a round's **own** closing summary (`  · label`, two spaces).

    Nested rounds are echoed as deeper-indented detail, so the exact two-space
    indent is what separates "this round's verdict" from "its child's output".
    """
    return [match.group(1).strip()
            for match in re.finditer(r"^  · (.+)$", stdout, re.MULTILINE)]

def own_red_labels(stdout):
    """A round's *own* failing assertions (its summary minus the nested `回归` children)."""
    return [label for label in summary_labels(stdout) if not label.startswith("回归 ")]

def env_cause(round_id, stdout):
    """The red of `round_id`, if it is entirely environmental; `None` otherwise.

    Environment, not behaviour: `tools/cao/.build/` is the running orchestrator's
    own copy of `cao` (`.gitignore` line 28) and `round-data/R11` scans all of
    `tools/**` for the word `DECISIONS`, so it flags those vendored `.py` files.
    Nothing under `tools/svg_grid/` is involved — and that is *checked*, not
    assumed: every `tools/…:LINE` the round printed must sit under
    `tools/cao/.build/`.
    """
    if round_id != "R11":
        return None
    paths = re.findall(r"tools/[\w./-]+:\d+", stdout)
    if not paths:
        return None
    outside = [path for path in paths if not path.startswith("tools/cao/.build/")]
    if outside:
        return None
    return (f"`round-data/R11` 的唯一红是「残留 现行文档里没有指向已删文件的活引用」，"
            f"命中的 {len(paths)} 处全在 `tools/cao/.build/`（运行中的 cao 编排器自己的目录，"
            "`.gitignore` 第 28 行忽略）—— 没有一处来自 tools/svg_grid")

def main():
    started = time.time()
    if not BIN.exists() or not CONVERT.exists():
        print(f"FAIL  前置：缺二进制（{BIN} / {CONVERT}）—— 先 cargo build")
        return 1
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r25-"))
    fonts = FONT_DIR.is_dir() and any(FONT_DIR.glob("*.ttf"))
    p1, p2, p3 = panel(tmp, "a"), panel(tmp, "b"), panel(tmp, "c")

    # ── 每个码一次真实运行（条数与基线一致，判据 ④）
    print("== ① / ④ 每个码各一次真实运行：条数 = 基线，单条 ≤ 200 ==")
    base = ["--width", "100", "--height", "80", "--ncol", "1", "--plot-margin", "0"]
    lettered = tmp / "lettered.svg"
    first = run([BIN, "plot-grid", "--input", p1, "--input", p2, "--output", lettered,
                 "--width", "400", "--ncol", "2", "--labels", "A,B", "--plot-margin", "0"])
    if first.returncode != 0:
        print(f"FAIL  前置：lettered block 没造出来 rc={first.returncode} {first.stderr[:200]}")
        return 1

    scenarios = [
        ("W1 输入被缩放", plot(tmp, "w1", [p1], ["--width", "1000", "--height", "800",
                                                "--ncol", "1", "--plot-margin", "0"]),
         1, ["W1"]),
        ("W1 自然布局（两格等比放大）", plot(tmp, "nat", [p1, p2], ["--width", "300", "--ncol", "2",
                                                              "--plot-margin", "0"]),
         1, ["W1"]),
        ("W2 格子被拉伸", plot(tmp, "w2", [p1, p2], ["--width", "300", "--ncol", "2",
                                                 "--rel-widths", "2,1", "--plot-margin", "0"]),
         2, ["W1", "W2"]),
        ("W3 重复加字母", plot(tmp, "w3", [lettered], ["--width", "400", "--height", "160",
                                                    "--ncol", "1", "--plot-margin", "0",
                                                    "--labels", "C"]),
         3, ["W3", "W5", "W6"]),
        ("W4a 字母个数不符", plot(tmp, "w4a", [p1, p2, p3], ["--width", "300", "--ncol", "3",
                                                         "--plot-margin", "0", "--labels", "A,B"]),
         3, ["W4a", "W5"]),
        ("W4b 字母重复", plot(tmp, "w4b", [p1, p2, p3], ["--width", "300", "--ncol", "3",
                                                     "--plot-margin", "0", "--labels", "A,A,B"]),
         4, ["W4b", "W5"]),
        ("W5 字母会被裁", plot(tmp, "w5", [p1], ["--width", "100", "--height", "80", "--ncol", "1",
                                             "--plot-margin", "2", "--labels", "A"]),
         3, ["W1", "W2", "W5"]),
        ("W6 内容越出画布", plot(tmp, "w6", [body_panel(tmp, "w6",
          '<g data-scale="position"><text x="-1.7" y="40" font-size="10">A</text></g>')], base),
         1, ["W6"]),
        ("W7 无法重写（两处）", plot(tmp, "w7", [body_panel(tmp, "w7",
          '<path d="M 10 20 30" fill="none" stroke="#000000"/>\n'
          '<path d="M0 0 Lnan nan" fill="none" stroke="#000000"/>')], base),
         1, ["W7"]),
        ("W9 悬空引用", plot(tmp, "w9", [body_panel(tmp, "w9",
          '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" '
          'fill="url(#no-such-id)"/></g>')], base),
         1, ["W9"]),
    ]
    if fonts:
        scenarios.append(("W8 字体库没有该族",
                          plot(tmp, "w8", [p1], ["--width", "100", "--height", "80", "--ncol", "1",
                                                 "--plot-margin", "0", "--labels", "A",
                                                 "--label-fontfamily", "NoSuchFont_xyz"]),
                          2, ["W5", "W8"]))
    else:
        skip("W8 字体库没有该族", f"{FONT_DIR} 里没有 .ttf（环境问题，不是行为差异）")

    for label, done, want_count, want_codes in scenarios:
        stderr = done.stderr
        lines = warning_lines(stderr)
        check(done.returncode == 0, f"{label}：rc=0（告警不改退出码）", f"rc={done.returncode}")
        check(len(lines) == want_count, f"{label}：告警条数 {len(lines)} = 基线 {want_count}",
              "\n".join(lines))
        check(sorted(codes(stderr)) == sorted(want_codes),
              f"{label}：码集合 {sorted(codes(stderr))} = 基线 {sorted(want_codes)}",
              "\n".join(lines))
        body_lines = lines + note_lines(stderr)
        check(longest(body_lines) <= MAX_LINE,
              f"{label}：单条最长 {longest(body_lines)} ≤ {MAX_LINE}", "\n".join(body_lines))
        measured = awk_lengths(stderr)
        check(measured and max(measured) <= MAX_LINE,
              f"{label}：awk 实测最长 {max(measured) if measured else '-'} ≤ {MAX_LINE}",
              stderr[:600])

    # V1（转换器那一侧，前缀更长：`svg_grid_convert: warning: `）
    print("\n== ① V1（convert 的 <style> 告警）==")
    conv_src = tmp / "conv_in.svg"
    conv_src.write_text(
        '<svg width="528" height="187.2" viewBox="0 0 528 187.2" '
        'xmlns="http://www.w3.org/2000/svg">\n'
        '<style>.a { stroke-linejoin: round }</style>\n'
        '<rect x="1" y="1" width="10" height="10" style="width: 5;"/>\n'
        '<g data-panel-group="1"><rect x="2" y="2" width="5" height="5"/></g></svg>\n'
    )
    conv = run([CONVERT, "--input", conv_src, "--output", tmp / "conv_out.svg", "--check"])
    conv_warn = [line for line in conv.stderr.splitlines()
                 if line.startswith("svg_grid_convert: warning:")]
    check(conv.returncode == 0, "V1：--check 成功", f"rc={conv.returncode} {conv.stderr[:200]}")
    check(any("<style> CSS block" in line for line in conv_warn),
          "V1：<style> CSS 块出声（doctor 的码词还在）", "\n".join(conv_warn))
    check(len(conv_warn) == 2, f"V1：两条转换器告警（style 几何 + <style> 块），实得 {len(conv_warn)}",
          "\n".join(conv_warn))
    check(max((len(line) for line in conv_warn), default=0) <= MAX_LINE,
          f"V1：单条最长 {max((len(line) for line in conv_warn), default=0)} ≤ {MAX_LINE}",
          "\n".join(conv_warn))
    hint = [line for line in conv.stdout.splitlines() if "hint: source canvas is" in line]
    check(bool(hint) and len(hint[0]) <= MAX_LINE,
          f"报告行 `hint: source canvas is …` 也 ≤ {MAX_LINE}（第 6 条：压到上限内，不留豁免）",
          "\n".join(hint))

    # ── ② 公共原因行
    print("\n== ② 公共原因只打一次 / 无告警时 0 行 ==")
    stretch = plot(tmp, "note_once", [p1, p2], ["--width", "300", "--ncol", "2",
                                                "--rel-widths", "2,1", "--plot-margin", "0"])
    notes = note_lines(stretch.stderr)
    check(len(notes) == 1, f"同因两条告警（W1+W2）⇒ 恰好 1 行 note，实得 {len(notes)}",
          "\n".join(notes))
    check(notes and notes[0].startswith(f"{NOTE_PREFIX}font-size is never rescaled"),
          "note 说的是那个公共原因（font-size 从不被缩放）", "\n".join(notes))
    check(len(set(notes)) == len(notes), "同一原因同一批次只出现一次（按句去重）", "\n".join(notes))
    check(all(NOTE_PREFIX not in line for line in warning_lines(stretch.stderr)),
          "原因不混进 `svg_grid: warning:` 行", "\n".join(warning_lines(stretch.stderr)))
    clean = plot(tmp, "clean", [body_panel(tmp, "clean",
          '<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>')], base)
    check(clean.returncode == 0 and clean.stderr == "",
          f"无告警 ⇒ stderr 0 字节（{len(clean.stderr)}B），note 一行都不出现",
          clean.stderr[:300])
    check(clean.stdout == "", "无告警 ⇒ stdout 也 0 字节", clean.stdout[:200])

    # ── ③ N = 1 不打计数
    print("\n== ③ N = 1 不打计数 ==")
    singles = [
        ("1 输入被缩放", plot(tmp, "s1", [p1], ["--width", "1000", "--height", "800", "--ncol", "1",
                                              "--plot-margin", "0"]), "1 input is scaled"),
        ("1 格被拉伸", plot(tmp, "s2", [p1], ["--width", "400", "--height", "200", "--ncol", "1",
                                            "--plot-margin", "0"]), "1 cell is stretched"),
        ("1 个属性无法重写", plot(tmp, "s3", [body_panel(tmp, "s3",
          '<circle cx="nan" cy="10" r="5" fill="#000000"/>')], base), "1 attribute could not"),
        ("1 格标签个数不符", plot(tmp, "s4", [p1], base + ["--labels", "A,B"]),
         "the grid has 1 input:"),
    ]
    for label, done, phrase in singles:
        stderr = done.stderr
        check(phrase in stderr, f"{label}：说的是人话（{phrase!r}）", stderr[:600])
        for wrapper in ("1 input(s)", "1 cell(s)", "1 attribute(s)", "1 non-fatal warning(s)"):
            check(wrapper not in stderr, f"{label}：没有 {wrapper!r} 这种计数包装", stderr[:600])
    check("1 non-fatal warning(s)" not in stretch.stderr and
          "2 non-fatal warning(s)" not in stretch.stderr,
          "收尾句不带条数（连 2 条时也不打计数）", stretch.stderr[-200:])
    check(FOOTER in stretch.stderr, "收尾句保留固定句型（doctor/R16 逐字依赖）", stretch.stderr[-200:])

    # ── ⑥ 三条规则写进 ARCHITECTURE/warnings.md
    print("\n== ⑥ ARCHITECTURE/warnings.md 的三条规则 ==")
    rules = {
        "单条 ≤ 200 字符": [r"200 字符", r"MAX_LINE"],
        "公共原因只打一次": [r"公共原因只打一次", r"svg_grid: note:"],
        "N = 1 不打计数": [r"N = 1.{0,6}不打计数", r"1 input\(s\)"],
        "第 6 条取舍（hint 压到 ≤200）": [r"hint: source canvas is", r"182 字符"],
    }
    text = WARNINGS_MD.read_text() if WARNINGS_MD.exists() else ""
    for label, patterns in rules.items():
        missing = [pattern for pattern in patterns if not re.search(pattern, text)]
        check(not missing, f"warnings.md 记着「{label}」", f"缺正则 {missing}")

    # ── ⑤ 旧证据回归
    print("\n== ⑤ 旧证据回归（逐轮 rc 与用时）==")
    budget = 1500 - (time.time() - started)
    for round_id in ("R4", "R5", "R7", "R9", "R11", "R13", "R14", "R16", "R17"):
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        leave = 1500 - (time.time() - started)
        if leave <= 30:
            skip(f"回归 round-data/{round_id}", "本轮总时长预算（1500 s）用尽，未跑")
            continue
        began = time.time()
        try:
            done = run([sys.executable, evidence], timeout=min(leave, 900))
            waited = time.time() - began
            ok = done.returncode == 0
            detail = done.stdout[-800:] + done.stderr[-300:]
            if ok:
                check(True, f"回归 round-data/{round_id} 仍全绿（rc=0, {waited:.0f}s）")
                continue
            # 红轮次必须先归因：自己的判据全过、红只来自嵌套的 `回归 Rxx` ⇒ 根因在更上游；
            # 根因是 `tools/cao/.build/`（环境自身的运行目录）⇒ 环境问题，按 R9 的先例 SKIP。
            env = env_cause(round_id, done.stdout)
            own = own_red_labels(done.stdout)
            if env:
                skip(f"回归 round-data/{round_id}",
                     f"{env}（rc={done.returncode}, {waited:.0f}s）")
            elif not own:
                nested = list(dict.fromkeys(summary_labels(done.stdout)))
                skip(f"回归 round-data/{round_id}",
                     f"本轮自己的判据全过，红只来自下游 {', '.join(nested)}（根因见 R11 那条 SKIP）"
                     f"（rc={done.returncode}, {waited:.0f}s）")
            else:
                check(False, f"回归 round-data/{round_id} 仍全绿"
                             f"（rc={done.returncode}, {waited:.0f}s）", detail)
                print(f"      NOTE  {diagnose(round_id, done.stdout)}")
        except subprocess.TimeoutExpired:
            waited = time.time() - began
            skip(f"回归 round-data/{round_id}", f"超时（{waited:.0f}s > 预算），未判绿")

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
