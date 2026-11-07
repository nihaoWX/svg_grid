#!/usr/bin/env python3
"""R18 的可跑证据 —— **合成一个二进制**：`convert` 变成 `svg_grid` 的子命令。

契约见 `ARCHITECTURE/cli.md` §子命令与参数面 与 `ARCHITECTURE/install.md`（冻结）；本轮只钉行为。
四族：

① **一个二进制，`convert` 是子命令**：`~/.local/bin` 里只有 `svg_grid`（没有 `svg_grid_convert`），
   且 `svg_grid convert …` 与工作区里仍能构建的 `convert/target/debug/svg_grid_convert` 在**全量 58 条
   fixture × 4 种模式**（`--output` / `--check` / `--report` / `--force`）+ 错误面 + 幂等/半成品/不兼容分支上
   **逐字节相同**（stdout / stderr / 退出码 / 产出的 SVG 文件）。唯一的**有意**差异是 `--help`：
   它在取值之前被特判，所以 `svg_grid convert --help` 打的是 `svg_grid` 的完整帮助（rc=0、只写 stdout）。
② **装机版 `svg_grid doctor` 全量语料**（cwd=`/tmp`、干净环境）：rc=0、`57 pass / 0 fail / 1 skip`，
   58 条逐条一行且格式合规，唯一的 SKIP 打出 `why`，W1 = W7 = W9 = 0，每个 PASS 的 PNG 都有墨水，
   报告里 `convert:` 行说明第一段是**进程内**的库。
③ **doctor 不再依赖外部二进制**：`PATH` 只含 `~/.local/bin`（里面没有 convert）⇒ 仍 rc=0；
   `PATH` 只含一个**空目录** ⇒ 同样 rc=0；不再出现「first stage is missing」。
   配置错这一档只剩语料：语料目录不存在 ⇒ rc=3 + `Fix:`（分档没被抹平）。
④ **回归 R4–R17** 的 `test.py` 全绿（R15/R16/R17 各自带回归，整段约 10–15 分钟；没有静默略过）。

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
SVG_GRID = HERE.parents[3]                        # round-data/R18 → PITFALLS → docs → svg_grid
BIN = SVG_GRID / "target/debug/svg_grid"          # 仓库内 debug（与旧二进制的对照组同代）
OLD_CONVERT = SVG_GRID / "convert/target/debug/svg_grid_convert"
FIXTURES = SVG_GRID / "tests/fixtures"
MANIFEST = FIXTURES / "MANIFEST.toml"
PREFIX = pathlib.Path.home() / ".local/bin"
INSTALLED = PREFIX / "svg_grid"
INSTALL = SVG_GRID / "install.sh"
FONT_DIR = pathlib.Path(os.environ.get("SVG_GRID_FONT_DIR")
                        or pathlib.Path.home() / "svg_grid/fontlib")

LINE_RE = re.compile(r"^(PASS|FAIL|SKIP)  (\S+)  (\S+)  (\d+)  warnings=(\d+)$")
SUMMARY_RE = re.compile(r"^summary: (\d+) pass, (\d+) fail, (\d+) skip$")
CONVERT_FLAGS = ("--input", "--output", "--check", "--report", "--force")

FAILS = []
TMP = None


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1500])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def info(text):
    print(f"INFO  {text}")


def run(args, env=None):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                          cwd="/tmp", env=env)          # 不依赖 cwd


def clean_env(path_dirs, home=None):
    return {"HOME": str(home or pathlib.Path.home()), "LANG": "C",
            "PATH": os.pathsep.join(str(p) for p in path_dirs)}


def summary_of(stdout):
    for line in stdout.splitlines():
        match = SUMMARY_RE.match(line.strip())
        if match:
            return tuple(int(value) for value in match.groups())
    return None


def verdict_lines(stdout):
    return [line for line in stdout.splitlines()
            if line.startswith(("PASS ", "FAIL ", "SKIP "))]


def manifest_ids():
    text = MANIFEST.read_text()
    ids = []
    for block in text.split("[[fixture]]")[1:]:
        fid = re.search(r"""^\s*id\s*=\s*["']([^"']+)""", block, re.M)
        expect = re.search(r"""^\s*expect\s*=\s*["']([^"']+)""", block, re.M)
        if fid:
            ids.append((fid.group(1), expect.group(1) if expect else "pass"))
    return ids


def font_faces():
    if not FONT_DIR.is_dir():
        return []
    return [p for p in FONT_DIR.rglob("*")
            if p.is_file() and p.suffix.lower() in {".ttf", ".otf", ".ttc"}]


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


# ── ① 逐字节对照 ────────────────────────────────────────────────────────────────────
def same(label, args, verbose=False):
    """`svg_grid convert <args>` vs `svg_grid_convert <args>`：四样都要一致。

    参数里的 `{out}` 换成两边的输出路径（路径不同是设计如此的，比对前在两边流里
    归一成一个占位符，于是只有真正的行为差异才会让它红）。
    """
    old_out, new_out = TMP / "old.svg", TMP / "new.svg"
    for path in (old_out, new_out):
        if path.exists():
            path.unlink()
    old = run([OLD_CONVERT, *[a.replace("{out}", str(old_out)) for a in args]])
    new = run([INSTALLED, "convert", *[a.replace("{out}", str(new_out)) for a in args]])

    def norm(text, out_path):
        # 输出路径两边不同（设计如此）⇒ 归一；R19 起程序名前缀也随调用方
        # （子命令说 `svg_grid convert:`，独立二进制说 `svg_grid_convert:`），
        # 同一句话的两种署名不算行为差异。
        text = text.replace(str(out_path), "OUT")
        for prog in ("svg_grid convert", "svg_grid_convert"):
            text = text.replace(prog, "PROG")
        return text

    old_stdout = norm(old.stdout, old_out)
    new_stdout = norm(new.stdout, new_out)
    old_stderr = norm(old.stderr, old_out)
    new_stderr = norm(new.stderr, new_out)
    problems = []
    if old.returncode != new.returncode:
        problems.append(f"rc(old={old.returncode} new={new.returncode})")
    if old_stdout != new_stdout:
        problems.append("stdout")
    if old_stderr != new_stderr:
        problems.append("stderr")
    if old_out.exists() != new_out.exists():
        problems.append("output-exists")
    elif old_out.exists() and old_out.read_bytes() != new_out.read_bytes():
        problems.append("output-bytes")
    if problems:
        print(f"      MISMATCH {label}: {problems}")
        if verbose or True:
            print("      old stderr: " + old_stderr[:400].replace("\n", "\n                  "))
            print("      new stderr: " + new_stderr[:400].replace("\n", "\n                  "))
    return not problems, old.returncode


def family_one():
    print("\n== ① 一个二进制：`svg_grid convert` 与旧 `svg_grid_convert` 逐字节相同 ==")
    check(not (PREFIX / "svg_grid_convert").exists(),
          "`~/.local/bin` 里只有 `svg_grid`：没有 `svg_grid_convert`",
          sorted(p.name for p in PREFIX.iterdir() if "svg_grid" in p.name))
    which = subprocess.run(["/usr/bin/env", "sh", "-c", "command -v svg_grid_convert"],
                           capture_output=True, text=True,
                           env=clean_env([PREFIX, "/usr/bin", "/bin"]))
    check(which.returncode != 0,
          "`command -v svg_grid_convert` 找不到（不再装到 PATH）",
          which.stdout.strip())

    # 装机版真能把一张方言 SVG 变成协议格式（不是"只是能跑"）
    panel = TMP / "panel.svg"
    write(panel, """<svg xmlns='http://www.w3.org/2000/svg' width='288pt' height='216pt' \
viewBox='0 0 288 216'>
<defs><style type='text/css'><![CDATA[ .svglite circle { fill: none; } ]]></style></defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; fill: #CFA15A;'/>
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</svg>
""")
    tagged = TMP / "tagged.svg"
    produced = run([INSTALLED, "convert", "--input", panel, "--output", tagged, "--report"])
    body = tagged.read_text() if tagged.exists() else ""
    check(produced.returncode == 0
          and 'data-panel-box="main"' in body and 'data-scale' in body
          and "converted ->" in produced.stdout,
          "装机版 `svg_grid convert --input … --output … --report` 真产出协议格式 + 报告",
          f"rc={produced.returncode}\n{produced.stderr[-400:]}\n{produced.stdout[:300]}")

    # 全量语料 × 4 种模式：逐字节
    cases = 0
    mismatches = []
    for fixture in sorted(FIXTURES.rglob("*.svg")):
        name = fixture.relative_to(FIXTURES)
        for mode, args in (("output", ["--input", str(fixture), "--output", "{out}"]),
                           ("check", ["--input", str(fixture), "--check"]),
                           ("report", ["--input", str(fixture), "--output", "{out}",
                                       "--report"]),
                           ("force", ["--input", str(fixture), "--output", "{out}",
                                      "--force"])):
            cases += 1
            ok, rc = same(f"{mode} {name}", args)
            if not ok:
                mismatches.append((f"{mode} {name}", rc))
    check(cases == 232 and not mismatches,
          "全量 58 条 × 4 种模式（--output/--check/--report/--force）逐字节相同",
          f"cases={cases} mismatches={mismatches[:5]}")

    # 错误面 / 边界分支：同样逐字节（含退出码 1 与 `svg_grid_convert:` 前缀）
    extra = [
        ("unknown flag", ["--bogus"]),
        ("unknown short flag", ["-x"]),
        ("missing --input", ["--check"]),
        ("missing --output", ["--input", str(FIXTURES / "ggplot2/boxplot.svg")]),
        ("missing value", ["--input"]),
        ("unreadable input", ["--input", "/nonexistent/nope.svg", "--check"]),
    ]
    half = TMP / "half.svg"
    write(half, """<svg xmlns='http://www.w3.org/2000/svg' width='288' height='216' \
viewBox='0 0 288 216'>
<rect data-panel-box='main' x='0' y='0' width='288' height='216' visibility='hidden' \
pointer-events='none'/>
<circle cx='10' cy='10' r='1'/>
</svg>
""")
    rotated = TMP / "rotated.svg"
    write(rotated, """<svg xmlns='http://www.w3.org/2000/svg' width='100pt' height='80pt' \
viewBox='0 0 100 80'>
<rect x='0' y='0' width='100' height='80' fill='#ffffff'/>
<g transform='translate(14 87) rotate(-90)'><text x='0' y='0' font-size='10' \
fill='#000000'>y</text></g>
</svg>
""")
    extra += [
        ("already converted", ["--input", str(tagged), "--output", "{out}"]),
        ("already converted --check", ["--input", str(tagged), "--check"]),
        ("already converted --report", ["--input", str(tagged), "--output", "{out}",
                                        "--report"]),
        ("half-finished product", ["--input", str(half), "--output", "{out}"]),
        ("half-finished --force", ["--input", str(half), "--output", "{out}", "--force"]),
        ("incompatible input", ["--input", str(rotated), "--output", "{out}"]),
        ("incompatible --check --report", ["--input", str(rotated), "--check", "--report"]),
    ]
    bad = [label for label, args in extra if not same(label, args)[0]]
    check(not bad, f"错误面与幂等/半成品/不兼容分支逐字节相同（{len(extra)} 例）", bad)

    error = run([INSTALLED, "convert", "--bogus"])
    check(error.returncode == 1 and error.stderr.startswith("svg_grid convert: ")
          and "unknown argument: --bogus" in error.stderr,
          "解析错仍是**退出 1**（convert 自己的一档，不并进 svg_grid 的 2/3）+ 前缀随调用方 "
          "（子命令是 `svg_grid convert:`，R19 起）", f"rc={error.returncode} {error.stderr[:300]}")

    # `--help`：唯一的有意差异
    old_help = run([OLD_CONVERT, "--help"])
    new_help = run([INSTALLED, "convert", "--help"])
    top_help = run([INSTALLED, "--help"])
    check(old_help.returncode == 0 and new_help.returncode == 0
          and new_help.stdout == top_help.stdout and new_help.stderr == "",
          "`convert --help` ⇒ rc=0、只写 stdout、打的是 `svg_grid --help`（有意差异，"
          "`-h/--help` 在取值前特判）", new_help.stderr[:200])
    missing = [flag for flag in CONVERT_FLAGS if flag not in top_help.stdout]
    check(not missing and "svg_grid convert --input" in top_help.stdout,
          "`svg_grid --help` 覆盖 convert 的全部 flag（帮助覆盖单测同口径）", missing)


# ── ② 装机版 doctor 全量语料 ─────────────────────────────────────────────────────────
def family_two(faces):
    print("\n== ② 装机版 `svg_grid doctor`：全量语料 ==")
    if not faces:
        skip("② 装机版 doctor 全量", f"字体目录不可用：{FONT_DIR}")
        return
    env = clean_env([PREFIX, "/usr/local/bin", "/usr/bin", "/bin"])
    started = time.time()
    full = subprocess.run([str(INSTALLED), "doctor"], capture_output=True, text=True,
                          cwd="/tmp", env=env)
    elapsed = time.time() - started
    check(full.returncode == 0, "全量：rc=0（有告警可以，但没有 FAIL）",
          f"rc={full.returncode}\n{full.stderr[-800:]}")
    summary = summary_of(full.stdout)
    check(summary == (57, 0, 1), "全量：`summary: 57 pass, 0 fail, 1 skip`", summary)
    lines = verdict_lines(full.stdout)
    parsed = [LINE_RE.match(line) for line in lines]
    check(len(lines) == 58, f"全量：58 条逐条一行（实际 {len(lines)}）", "\n".join(lines[:4]))
    check(all(parsed),
          "全量：每行都是 `PASS|FAIL|SKIP  <id>  <step>  <ms>  warnings=N`",
          [line for line, match in zip(lines, parsed) if not match][:3])
    ids = [match.group(2) for match in parsed if match]
    check(ids == [fid for fid, _ in manifest_ids()],
          "全量：报告顺序 = MANIFEST 顺序，58 条无漏无重", ids[:3])
    check(any(line.startswith("SKIP  matplotlib/mathtext") for line in lines),
          "全量：唯一的 SKIP 是 `matplotlib/mathtext`",
          [line for line in lines if line.startswith("SKIP")])
    why = [line.strip() for line in full.stdout.splitlines() if line.strip().startswith("why:")]
    check(why and "rotated ancestor" in why[0],
          "全量：SKIP 打出了 MANIFEST 的 `why`（不静默跳过）", why[:2])
    convert_line = next((line for line in full.stdout.splitlines()
                         if line.startswith("convert: ")), "")
    check("in-process" in convert_line and "svg_grid_convert" in convert_line,
          "全量：`convert:` 行说明第一段是**进程内**的库（不是某个可执行文件的路径）",
          convert_line)
    info(f"全量 doctor 用时 {elapsed:.1f}s（58 张、两段、串行）")

    machine = subprocess.run([str(INSTALLED), "doctor", "--json"], capture_output=True,
                             text=True, cwd="/tmp", env=env)
    try:
        report = json.loads(machine.stdout)
    except Exception as error:                       # noqa: BLE001 - 报文即证据
        report = None
        check(False, "全量 `--json`：可被 json.loads 解析", error)
    if report:
        codes = {}
        for fixture in report["fixtures"]:
            for code in fixture["warning_codes"]:
                codes[code] = codes.get(code, 0) + 1
        info(f"告警码统计：{codes}")
        check("W1" not in codes and "W7" not in codes and "W9" not in codes,
              "全量：W1 = W7 = W9 = 0（按自然尺寸组图；没有原样保留的属性 / 悬空 url）", codes)
        no_ink = [f["id"] for f in report["fixtures"]
                  if f["status"] == "pass" and not (f.get("ink_pixels") or 0) > 0]
        check(not no_ink, "全量：每个 PASS 的 PNG 都有墨水（非白像素 > 0）", no_ink)
        check(report["summary"] == {"pass": 57, "fail": 0, "skip": 1},
              "全量 `--json`：summary 与逐行计数一致", report["summary"])


# ── ③ doctor 不再依赖外部二进制 ──────────────────────────────────────────────────────
def family_three(faces):
    print("\n== ③ doctor 不再依赖外部二进制 ==")
    if not faces:
        skip("③ 无外部二进制", f"字体目录不可用：{FONT_DIR}")
        return
    only_prefix = subprocess.run([str(INSTALLED), "doctor"], capture_output=True, text=True,
                                 cwd="/tmp", env=clean_env([PREFIX]))
    check(only_prefix.returncode == 0
          and summary_of(only_prefix.stdout) == (57, 0, 1),
          "`PATH` 只含 `~/.local/bin`（里面没有 convert）⇒ rc=0、全量 57/0/1",
          f"rc={only_prefix.returncode} summary={summary_of(only_prefix.stdout)}\n"
          f"{only_prefix.stderr[-600:]}")

    (TMP / "empty-path").mkdir(exist_ok=True)
    empty_path = subprocess.run([str(INSTALLED), "doctor"], capture_output=True, text=True,
                                cwd="/tmp", env=clean_env([TMP / "empty-path"]))
    check(empty_path.returncode == 0
          and summary_of(empty_path.stdout) == (57, 0, 1),
          "`PATH` 只有一个**空目录** ⇒ 仍 rc=0、全量 57/0/1（第一段是库调用）",
          f"rc={empty_path.returncode}\n{empty_path.stderr[-600:]}")
    check("first stage is missing" not in empty_path.stderr
          and "svg_grid_convert" not in empty_path.stderr,
          "不再出现「第一段找不到 / 在 PATH 上找 convert」的配置错报文（exit 3 分支已删）",
          empty_path.stderr[-600:])

    no_corpus = subprocess.run([str(INSTALLED), "doctor", "--fixtures",
                                str(TMP / "does-not-exist")],
                               capture_output=True, text=True, cwd="/tmp",
                               env=clean_env([PREFIX]))
    check(no_corpus.returncode == 3,
          "语料目录不存在 ⇒ 仍退出 3（配置错这一档没被抹平）",
          f"rc={no_corpus.returncode}")
    check("--fixtures" in no_corpus.stderr and "Fix:" in no_corpus.stderr,
          "缺语料：说清怎么指 + `Fix:`", no_corpus.stderr[-400:])

    dry = subprocess.run(["bash", str(INSTALL), "--dry-run"], capture_output=True, text=True,
                         cwd="/tmp", env=dict(os.environ))
    check(dry.returncode == 0 and "would install svg_grid ->" in dry.stdout
          and "would install svg_grid_convert" not in dry.stdout,
          "`install.sh --dry-run` 只计划装一个二进制（不再有 svg_grid_convert）",
          dry.stdout[:600])
    check(dry.stdout.count("would install") == 1,
          "`install.sh --dry-run` 的安装计划只有一行", dry.stdout)


# ── ④ 回归 ──────────────────────────────────────────────────────────────────────────
ROUNDS = ["R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R13", "R14", "R15", "R16",
          "R17"]


def family_four():
    print("\n== ④ 回归 R4–R17 ==")
    total = time.time()
    for round_id in ROUNDS:
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        started = time.time()
        regression = subprocess.run([sys.executable, str(evidence)], capture_output=True,
                                    text=True, cwd="/tmp")
        info(f"{round_id}: rc={regression.returncode} 用时 {time.time() - started:.1f}s")
        check(regression.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              regression.stdout[-800:] + regression.stderr[-400:])
    info(f"回归总用时 {time.time() - total:.1f}s")


def main():
    global TMP
    TMP = pathlib.Path(tempfile.mkdtemp(prefix="r18-rounddata-"))
    print(f"# 仓库内旧二进制：{OLD_CONVERT}\n# 装机版：{INSTALLED}\n# 字体目录：{FONT_DIR}\n")

    # ── 0. 前置
    check(BIN.is_file() and os.access(BIN, os.X_OK),
          "仓库内 `target/debug/svg_grid` 存在且可执行", str(BIN))
    check(OLD_CONVERT.is_file() and os.access(OLD_CONVERT, os.X_OK),
          "工作区里 `convert/target/debug/svg_grid_convert` 仍可构建、可执行（对照组）",
          str(OLD_CONVERT))
    check(INSTALLED.is_file() and os.access(INSTALLED, os.X_OK),
          "装机版 `~/.local/bin/svg_grid` 存在且可执行（先跑 install.sh）", str(INSTALLED))
    if FAILS:
        print("\n== 前置不成立：先 `cargo build --offline` 与 `tools/svg_grid/install.sh` ==")
        return 1
    entries = manifest_ids()
    files = sorted(FIXTURES.rglob("*.svg"))
    check(len(entries) == 58 and len(files) == 58,
          "语料登记：MANIFEST 58 条 <-> tests/fixtures 58 个 SVG",
          f"manifest={len(entries)} files={len(files)}")
    faces = font_faces()
    if faces:
        info(f"字体目录可用：{len(faces)} 个 face")
    else:
        skip("依赖渲染的断言（② 全量 doctor / ③ 无外部二进制）",
             f"字体目录不可用：{FONT_DIR}（先跑 fontlib/setup.sh，或设 $SVG_GRID_FONT_DIR）")

    family_one()
    family_two(faces)
    family_three(faces)
    family_four()

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
