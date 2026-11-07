#!/usr/bin/env python3
"""R17 的可跑证据 —— **`svg_grid doctor`：一条命令把整套语料跑完并给判据**。

契约见 `ARCHITECTURE/testing.md`（冻结）；本轮不解释契约，只钉行为。六族：

① **全量语料**（`tests/fixtures/`，58 条）：`svg_grid doctor` ⇒ rc=0、pass ≥ 56 / fail = 0 / skip = 1；
   58 条逐条一行且格式合规；唯一的 SKIP 是 `matplotlib/mathtext` 且打出 `why`；用**仓库内**二进制
   （`target/debug/…`），`convert:` 行说明第一段是进程内的库（R18 起不再是外部路径）；
   按自然尺寸组图 ⇒ W1 = 0、W7/W9 = 0。
② **坏 fixture 必须被判红**：悬空 `url(#nope)` ⇒ FAIL（W9 + 无墨水）、空图 ⇒ FAIL（PNG 纯白），rc=1。
③ **`--only` 只跑命中项**；`--json` 可被 `json.loads` 且字段齐（rc / 告警码 / 墨水 / 耗时）；
   `--keep` 留住中间产物。
④ **配置错只留给语料**：语料目录不存在 ⇒ rc=3 + 一句英文 + `Fix:`；R18 起第一段是库调用，
   `PATH` 里没有 `svg_grid_convert` **不再**是配置错（旧轮的那条 exit 3 断言已随之翻面）。
⑤ **装机版 + 语料目录解析**：`~/.local/bin/svg_grid doctor`（cwd=`/tmp`、干净环境）跑通全量；
   `SVG_GRID_FIXTURES` 指向只含 1 条的临时目录 ⇒ 只跑那 1 条（`--fixtures` > env > 数据根 > 仓库）。
⑥ **回归** R4–R16 的 `test.py` 全绿（R16 自己会重跑 R4–R15，整段约 7–9 分钟；没有静默略过）。

无参数。临时产物在临时目录；离线、只读（不写仓库）。字体目录走 `$SVG_GRID_FONT_DIR`
（缺省 `$HOME/svg_grid/fontlib`）；不可用 ⇒ 依赖渲染的断言 SKIP 并打印原因。
"""
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                       # round-data/R17 → PITFALLS → docs → svg_grid
BIN = SVG_GRID / "target/debug/svg_grid"         # 仓库内 debug：~/.local/bin 上是旧版本
CONVERT = SVG_GRID / "convert/target/debug/svg_grid_convert"
FIXTURES = SVG_GRID / "tests/fixtures"
MANIFEST = FIXTURES / "MANIFEST.toml"
FONT_DIR = pathlib.Path(os.environ.get("SVG_GRID_FONT_DIR")
                        or pathlib.Path.home() / "svg_grid/fontlib")

# 每条 fixture 的输出行：PASS|FAIL|SKIP  <id>  <step>  <ms>  warnings=N
LINE_RE = re.compile(r"^(PASS|FAIL|SKIP)  (\S+)  (\S+)  (\d+)  warnings=(\d+)$")
SUMMARY_RE = re.compile(r"^summary: (\d+) pass, (\d+) fail, (\d+) skip$")
WARNING_CODES = {"W1", "W2", "W3", "W4a", "W4b", "W5", "W6", "W7", "W8", "W9", "V1"}
KNOWN_CODE_WORDS = {"W1": "scaled by the layout",
                    "W7": "could not be rewritten and were left exactly as written",
                    "W9": "point at an id that nothing in the output defines"}

FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1200])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def run(args, env=None):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                          cwd="/tmp", env=env)          # 不依赖 cwd


def doctor_env(path_dirs):
    env = dict(os.environ)
    env["PATH"] = os.pathsep.join(str(p) for p in path_dirs)
    return env


def doctor(args, with_convert=True, extra_env=None):
    """跑 doctor；`with_convert=False` 时 PATH 里只剩一个空目录（模拟没装第一段）。"""
    if not with_convert:
        env = doctor_env([TMP / "empty-path"])
    else:
        bin_dir = TMP / "bin"
        bin_dir.mkdir(exist_ok=True)
        link = bin_dir / CONVERT.name
        if not link.exists():
            link.symlink_to(CONVERT)
        env = doctor_env([bin_dir])
    env.update(extra_env or {})
    return run([BIN, "doctor", *args], env=env)


def verdict_lines(stdout):
    return [line for line in stdout.splitlines()
            if line.startswith(("PASS ", "FAIL ", "SKIP "))]


def summary_of(stdout):
    for line in stdout.splitlines():
        match = SUMMARY_RE.match(line.strip())
        if match:
            return tuple(int(value) for value in match.groups())
    return None


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


GOOD_PANEL = """<svg width="100pt" height="80pt" viewBox="0 0 100 80" \
xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="#3366cc"/>
<line x1="10" y1="60" x2="90" y2="60" stroke="#000000" stroke-width="1"/></g>
</svg>
"""
DANGLING_PANEL = """<svg width="100pt" height="80pt" viewBox="0 0 100 80" \
xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#nope)"/></g>
</svg>
"""
BLANK_PANEL = """<svg width="100pt" height="80pt" viewBox="0 0 100 80" \
xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"></g>
</svg>
"""


def manifest_ids():
    """MANIFEST 的 (id, expect) 列表（`why` 只用于打印）。"""
    text = MANIFEST.read_text()
    ids = []
    for block in text.split("[[fixture]]")[1:]:
        fid = re.search(r"""^\s*id\s*=\s*["']([^"']+)""", block, re.M)
        expect = re.search(r"""^\s*expect\s*=\s*["']([^"']+)""", block, re.M)
        if fid:
            ids.append((fid.group(1), expect.group(1) if expect else "pass"))
    return ids


def family_ok():
    faces = [p for p in FONT_DIR.rglob("*")
             if p.is_file() and p.suffix.lower() in {".ttf", ".otf", ".ttc"}] \
        if FONT_DIR.is_dir() else []
    return faces


def main():
    global TMP
    TMP = pathlib.Path(tempfile.mkdtemp(prefix="r17-rounddata-"))
    print(f"# 仓库内二进制：{BIN}\n#                {CONVERT}\n# 字体目录：{FONT_DIR}\n")

    # ── 0. 前置
    check(BIN.is_file() and os.access(BIN, os.X_OK),
          "仓库内 `target/debug/svg_grid` 存在且可执行", str(BIN))
    check(CONVERT.is_file() and os.access(CONVERT, os.X_OK),
          "仓库内 `convert/target/debug/svg_grid_convert` 存在且可执行（不是 ~/.local/bin 的旧版）",
          str(CONVERT))
    if FAILS:
        print("\n== 前置不成立：先 `cargo build --offline`（两个 crate） ==")
        return 1
    entries = manifest_ids()
    files = sorted(FIXTURES.rglob("*.svg"))
    check(len(entries) == 58 and len(files) == 58,
          "语料登记：MANIFEST 58 条 <-> tests/fixtures 58 个 SVG",
          f"manifest={len(entries)} files={len(files)}")
    faces = family_ok()
    if faces:
        print(f"INFO  字体目录可用：{len(faces)} 个 face")
    else:
        skip("依赖渲染的断言（PNG / 墨水 / 全量 doctor）",
             f"字体目录不可用：{FONT_DIR}（先跑 fontlib/setup.sh，或设 $SVG_GRID_FONT_DIR）")

    # ── ① 全量语料
    print("\n== ① 全量语料：`svg_grid doctor` ==")
    if not faces:
        skip("① 全量语料", "字体目录不可用（见上）")
    else:
        started = time.time()
        full = doctor([])
        elapsed = time.time() - started
        check(full.returncode == 0, "全量：rc=0（有告警也可以，但没有任何 FAIL）",
              f"rc={full.returncode}\n{full.stderr[-800:]}")
        convert_line = next((line for line in full.stdout.splitlines()
                             if line.startswith("convert: ")), "")
        # R18 起第一段是**库**调用（进程内），所以这一行是说明而不是路径。
        check("in-process" in convert_line and "svg_grid_convert" in convert_line,
              "全量：`convert:` 行说明第一段是进程内的 `svg_grid_convert` 库（没有外部二进制可找）",
              convert_line)
        lines = verdict_lines(full.stdout)
        check(len(lines) == 58, f"全量：58 条逐条一行（实际 {len(lines)} 行）",
              "\n".join(lines[:5]))
        parsed = [LINE_RE.match(line) for line in lines]
        check(all(parsed), "全量：每行都是 `PASS|FAIL|SKIP  <id>  <step>  <ms>  warnings=N`",
              [line for line, match in zip(lines, parsed) if not match][:3])
        rows = [(m.group(1), m.group(2), m.group(3), int(m.group(4)), int(m.group(5)))
                for m in parsed if m]
        ids = [row[1] for row in rows]
        check(len(set(ids)) == len(ids) == 58,
              "全量：58 条的 id 与 MANIFEST 一一对应（没有漏跑、没有重复）",
              f"unique={len(set(ids))}")
        check([row[1] for row in rows] == [fid for fid, _ in entries],
              "全量：报告顺序 = MANIFEST 顺序")
        statuses = {}
        for status, *_ in rows:
            statuses[status] = statuses.get(status, 0) + 1
        passed = statuses.get("PASS", 0)
        failed = statuses.get("FAIL", 0)
        skipped = statuses.get("SKIP", 0)
        check(passed >= 56 and failed == 0 and skipped == 1,
              f"全量：pass={passed} ≥ 56、fail={failed} = 0、skip={skipped} = 1",
              str(statuses))
        summary = summary_of(full.stdout)
        check(summary == (passed, failed, skipped),
              "全量：`summary: N pass, M fail, K skip` 与逐行计数一致", f"{summary} vs {statuses}")
        check(any(row[0] == "SKIP" and row[1] == "matplotlib/mathtext" for row in rows),
              "全量：唯一的 SKIP 是 `matplotlib/mathtext`（MANIFEST 的 expect=unsupported）",
              [row for row in rows if row[0] == "SKIP"])
        why_lines = [line.strip() for line in full.stdout.splitlines() if line.strip().startswith("why:")]
        check(why_lines and "rotated ancestor" in why_lines[0],
              "全量：SKIP 打出了 MANIFEST 的 `why`（不许静默跳过）", why_lines[:2])
        print(f"INFO  全量 doctor 用时 {elapsed:.1f}s（58 张、两段、串行）")

        # 机读版：告警码口径（W1 / W7 / W9 的硬约束）
        machine = doctor(["--json"])
        check(machine.returncode == 0, "全量 --json：rc=0", machine.stderr[-400:])
        try:
            report = json.loads(machine.stdout)
        except Exception as error:                      # noqa: BLE001 - 报文即证据
            report = None
            check(False, "全量 --json：可被 json.loads 解析", error)
        if report:
            codes = {}
            for fixture in report["fixtures"]:
                for code in fixture["warning_codes"]:
                    codes[code] = codes.get(code, 0) + 1
            print(f"      告警码统计：{codes}")
            check("W7" not in codes and "W9" not in codes,
                  "全量：没有任何 W7（原样保留）或 W9（悬空 url(#…)）——两条判 FAIL 的口径",
                  str(codes))
            check("W1" not in codes,
                  "全量：W1 = 0（doctor 按每条 fixture 的自然尺寸组图，没人被缩放）", str(codes))
            no_ink = [f["id"] for f in report["fixtures"]
                      if f["status"] == "pass" and not (f["ink_pixels"] or 0) > 0]
            check(not no_ink, "全量：每个 PASS 的 PNG 都有墨水（非白像素 > 0）", no_ink)

    # ── ② 坏 fixture 必须被判红
    print("\n== ② 坏 fixture：悬空 url(#nope) / 空图 ==")
    corpus = TMP / "corrupt-corpus"
    write(corpus / "MANIFEST.toml", """# R17 的坏语料：一条好的，两条必须判红的。
[[fixture]]
id = "ggplot2/ok"
expect = "pass"

[[fixture]]
id = "corrupt/dangling"
expect = "pass"

[[fixture]]
id = "corrupt/blank"
expect = "pass"
""")
    write(corpus / "ggplot2/ok.svg", GOOD_PANEL)
    write(corpus / "corrupt/dangling.svg", DANGLING_PANEL)
    write(corpus / "corrupt/blank.svg", BLANK_PANEL)
    if not faces:
        skip("② 坏 fixture", "字体目录不可用（见上）")
    else:
        bad = doctor(["--fixtures", corpus])
        check(bad.returncode == 1, "坏语料：doctor 退出 1（红就是红，不是 0）",
              f"rc={bad.returncode}\n{bad.stderr[-400:]}")
        check(summary_of(bad.stdout) == (1, 2, 0),
              "坏语料：`summary: 1 pass, 2 fail, 0 skip`", summary_of(bad.stdout))
        rows = verdict_lines(bad.stdout)
        hanging = [line for line in rows if line.startswith("FAIL  corrupt/dangling ")]
        blank = [line for line in rows if line.startswith("FAIL  corrupt/blank ")]
        check(len(hanging) == 1 and "  check  " in hanging[0],
              "坏语料：悬空 url(#nope) ⇒ `FAIL  corrupt/dangling  check`", rows)
        check(len(blank) == 1 and "  check  " in blank[0],
              "坏语料：空图 ⇒ `FAIL  corrupt/blank  check`", rows)
        check("failures:" in bad.stdout, "坏语料：失败的 fixture 在末尾再列一遍", bad.stdout[-800:])
        head = bad.stdout.split("failures:", 1)[1] if "failures:" in bad.stdout else ""
        check("corrupt/dangling" in head and "W9" in head,
              "坏语料：悬空的判据点名 W9", head[:800])
        check("url(#nope)" in head,
              "坏语料：把 stderr 的关键行（点名哪个 id）再打一遍", head[:800])
        check("no ink" in head and "pure white" in head,
              "坏语料：空图的判据是「PNG 纯白、没有墨水」", head[:800])
        check("PASS  ggplot2/ok  both" in bad.stdout,
              "坏语料：同一次运行里的好图照旧 PASS（不是一刀切）", rows)

    # ── ③ --only / --json / --keep
    print("\n== ③ `--only` / `--json` / `--keep` ==")
    if not faces:
        skip("③ --only / --json / --keep", "字体目录不可用（见上）")
    else:
        ggplot2 = [fid for fid, _ in entries if fid.startswith("ggplot2/")]
        only = doctor(["--only", "ggplot2/*"])
        check(only.returncode == 0, "`--only 'ggplot2/*'`：rc=0", only.stderr[-400:])
        rows = verdict_lines(only.stdout)
        check(len(rows) == len(ggplot2) and all(row.split()[1].startswith("ggplot2/") for row in rows),
              f"`--only 'ggplot2/*'`：只跑 {len(ggplot2)} 条 ggplot2 语料", rows[:3])
        check(summary_of(only.stdout) == (len(ggplot2), 0, 0),
              f"`--only 'ggplot2/*'`：summary: {len(ggplot2)} pass, 0 fail, 0 skip",
              summary_of(only.stdout))
        single = doctor(["--only", "matplotlib/mathtext"])
        check(single.returncode == 0 and len(verdict_lines(single.stdout)) == 1
              and verdict_lines(single.stdout)[0].startswith("SKIP  matplotlib/mathtext"),
              "`--only 'matplotlib/mathtext'`：只跑这一条，且仍是 SKIP",
              verdict_lines(single.stdout))
        nothing = doctor(["--only", "nope/*"])
        check(nothing.returncode == 2 and "matched none" in nothing.stderr,
              "`--only` 命中不到任何 fixture ⇒ 退出 2（用法错，不是静默跑 0 条）",
              f"rc={nothing.returncode} {nothing.stderr[-300:]}")

        machine = doctor(["--json"])
        try:
            report = json.loads(machine.stdout)
        except Exception as error:                      # noqa: BLE001 - 报文即证据
            report = None
            check(False, "`--json`：json.loads 可解析", error)
        if report:
            for key in ("fixtures_dir", "convert", "jobs", "only", "total", "summary", "fixtures"):
                check(key in report, f"`--json`：顶层有 `{key}`", sorted(report))
            check(report["total"] == 58 == len(report["fixtures"]),
                  "`--json`：total=58，`fixtures` 里也正好 58 条",
                  (report.get("total"), len(report.get("fixtures", []))))
            counted = {"pass": 0, "fail": 0, "skip": 0}
            for fixture in report["fixtures"]:
                counted[fixture["status"]] = counted.get(fixture["status"], 0) + 1
            check(report["summary"] == counted,
                  "`--json`：summary 与逐条 status 一致", (report["summary"], counted))
            required = {"id", "expect", "status", "step", "ms", "warnings", "warning_codes",
                        "convert_rc", "plot_rc", "svg_bytes", "png_bytes", "ink_pixels",
                        "why", "problems", "stderr"}
            missing = [f["id"] for f in report["fixtures"] if not required <= set(f)]
            check(not missing, "`--json`：每条都有 rc / 告警码 / 墨水像素 / 耗时等字段",
                  missing[:3])
            bad_codes = [f["id"] for f in report["fixtures"]
                         if not set(f["warning_codes"]) <= WARNING_CODES]
            check(not bad_codes, "`--json`：warning_codes 只含 W1–W9/V1", bad_codes[:3])
            sample = report["fixtures"][0]
            print(f"      样本：{json.dumps({k: sample[k] for k in
                                           ('id', 'status', 'step', 'ms', 'warnings',
                                            'warning_codes', 'ink_pixels')}, ensure_ascii=False)}")
            check(isinstance(sample["ms"], int) and sample["ms"] >= 0
                  and isinstance(sample["warnings"], int)
                  and isinstance(sample["ink_pixels"], int) and sample["ink_pixels"] > 0,
                  "`--json`：ms / warnings / ink_pixels 都是整数，PASS 的墨水 > 0", sample)

        keep = TMP / "keep"
        kept = doctor(["--only", "ggplot2/polygon", "--keep", keep])
        files = sorted(path.name for path in keep.iterdir()) if keep.is_dir() else []
        check(kept.returncode == 0 and len(files) == 3
              and any(name.endswith(".tagged.svg") for name in files)
              and any(name.endswith(".png") for name in files),
              "`--keep <dir>`：留下 tagged / 组图 SVG / PNG 三个中间产物", files)

    # ── ④ 没有外部第一段也要跑通；语料缺仍是配置错
    print("\n== ④ 无外部 convert 仍跑通 / 语料缺：退出 3 + Fix: ==")
    (TMP / "empty-path").mkdir(exist_ok=True)
    no_convert = run([BIN, "doctor", "--fixtures", FIXTURES],
                     env=doctor_env([TMP / "empty-path"]))
    # R18：doctor 直接调 convert 库，PATH 里没有 `svg_grid_convert` 也必须照跑。
    check(no_convert.returncode == 0,
          "PATH 里没有任何 `svg_grid_convert` ⇒ doctor 仍退出 0（第一段是库调用）",
          f"rc={no_convert.returncode}\n{no_convert.stderr[-500:]}")
    check(summary_of(no_convert.stdout) is not None
          and summary_of(no_convert.stdout)[1] == 0,
          "PATH 里没有 convert：全量语料照跑，0 fail", summary_of(no_convert.stdout))
    check("first stage is missing" not in no_convert.stderr
          and "svg_grid_convert" not in no_convert.stderr,
          "不再出现「第一段找不到」的配置错报文（那条 exit 3 分支已删）",
          no_convert.stderr[-500:])
    no_corpus = doctor(["--fixtures", TMP / "does-not-exist"])
    check(no_corpus.returncode == 3,
          "语料目录不存在 ⇒ 退出 3（配置错）", f"rc={no_corpus.returncode}")
    check("--fixtures" in no_corpus.stderr and "Fix:" in no_corpus.stderr,
          "缺语料：说清怎么指（`--fixtures <dir>`）+ `Fix:`", no_corpus.stderr[-500:])
    help_text = run([BIN, "doctor", "--help"])
    check(help_text.returncode == 0 and help_text.stdout == run([BIN, "--help"]).stdout,
          "`doctor --help`：stdout + rc=0 + 与 `--help` 同一份文本", help_text.stderr[-300:])
    for flag in ("--fixtures", "--only", "--jobs", "--json", "--keep"):
        check(flag in help_text.stdout, f"`doctor --help` 覆盖 `{flag}`")

    # ── ⑤ 装机版 doctor + 语料目录解析（`--fixtures` > `SVG_GRID_FIXTURES` > 数据根 > 仓库）
    print("\n== ⑤ 装机版 `~/.local/bin/svg_grid doctor` / `SVG_GRID_FIXTURES` ==")
    installed = pathlib.Path.home() / ".local/bin/svg_grid"
    if not faces or not installed.is_file():
        skip("⑤ 装机版 doctor", f"没有装机版 {installed}（先跑 tools/svg_grid/install.sh）")
    else:
        clean = {"HOME": str(pathlib.Path.home()), "LANG": "C",
                 "PATH": f"{installed.parent}:/usr/local/bin:/usr/bin:/bin"}
        home_run = subprocess.run([str(installed), "doctor"], capture_output=True, text=True,
                                  cwd="/tmp", env=clean)
        got = summary_of(home_run.stdout)
        check(home_run.returncode == 0 and got is not None and got[0] >= 56
              and got[1] == 0 and got[2] == 1,
              "装机版：cwd=/tmp + 干净环境 ⇒ rc=0、全量语料 pass≥56 / fail=0 / skip=1",
              f"rc={home_run.returncode} summary={got}\n{home_run.stderr[-600:]}")
        one = TMP / "one-fixture"
        write(one / "MANIFEST.toml", '[[fixture]]\nid = "ggplot2/ok"\nexpect = "pass"\n')
        write(one / "ggplot2/ok.svg", GOOD_PANEL)
        scoped = doctor([], extra_env={"SVG_GRID_FIXTURES": str(one)})
        rows = verdict_lines(scoped.stdout)
        check(summary_of(scoped.stdout) == (1, 0, 0) and len(rows) == 1
              and rows[0].startswith("PASS  ggplot2/ok "),
              "`SVG_GRID_FIXTURES` 指向只含 1 条语料的临时目录 ⇒ 只跑那 1 条",
              f"{summary_of(scoped.stdout)} {rows} {scoped.stderr[-400:]}")

    # ── ⑥ 回归 R4–R16
    print("\n== ⑥ 回归 R4–R16 ==")
    rounds = ["R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R13", "R14", "R15", "R16"]
    total = time.time()
    for round_id in rounds:
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        started = time.time()
        regression = run([sys.executable, evidence])
        print(f"      {round_id}: rc={regression.returncode} 用时 {time.time() - started:.1f}s")
        check(regression.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              regression.stdout[-500:] + regression.stderr[-300:])
    print(f"INFO  回归总用时 {time.time() - total:.1f}s（R16 自己会重跑 R4–R15）")

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
