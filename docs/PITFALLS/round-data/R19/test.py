#!/usr/bin/env python3
"""R19 的可跑证据 —— **自检只有一个测试目标** + convert 的 stderr 前缀随调用方。

用户裁决（照做）：「测试功能本身也是属于主库的能力，自检，所以为什么 compose_tests 和 doctor_tests
是分开的？它们语义上应该是自检，也就是都属于 doctor。」契约见 `ARCHITECTURE/testing.md`（冻结）。

五族：

① **目录树**：`tests/` 下只有 `selftest/`（外加 `fixtures/` 与 `fig2_mixed`、`mpl_dialect`、
   `poppler_dialect`、`rdkit_dialect`、`s22_repro`、`shape_paint` 这些历史资产目录）；
   `compose_tests/`、`doctor_tests/` 已删；`cargo test` 只编出**一个**集成测试目标
   `tests/selftest/main.rs`。
② **用例数不变**：迁移前 `cargo test --offline` = lib 60 + bin 57 + compose_tests 60 + doctor_tests 13
   = **190**（写死）；迁移后 lib 60 + bin 57 + selftest **73** = **190** —— 73 = 60 + 13，一条没丢。
③ **体积与模块树**：所有 `tests/**/*.rs` ≤ 20000 B；源码里没有 `include!(`（R8 的教训：rustfmt 看不见）；
   每个非 `mod.rs`/`main.rs` 的 `.rs` 都被上层 `mod` 声明；`selftest/main.rs` 只做 `mod` 声明且 ≤ 4500 B。
④ **前缀随调用方**（本轮要修的 R18 遗留）：`svg_grid convert` 的错误/告警/`already converted` 前缀是
   `svg_grid convert:`（usage 行同）；独立二进制仍是 `svg_grid_convert:`（usage 行同）；
   `doctor` 的告警分类仍认得子命令前缀（端到端：`<style>` 图 ⇒ `warnings=1` + `V1`，不是"只是没炸"）。
⑤ **回归**：R8/R9/R14 + R18（R18 的 ④ 内部逐轮跑 R4–R17，含 R15/R16/R17；不静默略过）。
   R18 的 ① 里「逐字节相同 / 前缀」三类断言被本轮裁决**有意**取代（R18 的 `test.py` 不在本轮允许改动范围），
   本脚本把它们**逐条列出**并且只容忍这三类，R18 其余断言一条都不许红。

无参数。临时产物在临时目录；离线、只读（除 `cargo test` 写 `target/`）。字体目录走
`$SVG_GRID_FONT_DIR`（缺省 `$HOME/svg_grid/fontlib`）；不可用 ⇒ 依赖渲染的那条断言 SKIP 并打印原因。
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
SVG_GRID = HERE.parents[3]                        # round-data/R19 → PITFALLS → docs → svg_grid
TESTS = SVG_GRID / "tests"
BIN = SVG_GRID / "target/debug/svg_grid"           # 仓库内 debug（与对照组的旧二进制同代）
CONVERT = SVG_GRID / "convert/target/debug/svg_grid_convert"
INSTALLED = pathlib.Path.home() / ".local/bin/svg_grid"
FONT_DIR = pathlib.Path(os.environ.get("SVG_GRID_FONT_DIR")
                        or pathlib.Path.home() / "svg_grid/fontlib")

LIMIT = 20000                                      # R8 的口径：按字节从严（20 KB）
ENTRY_LIMIT = 4500                                 # 门面（main.rs）要薄
BASELINE = 190                                     # 迁移前 `cargo test --offline` 的用例总数（写死）
COUNTS = {"unittests src/lib.rs": 60, "unittests src/main.rs": 57,
          "tests/selftest/main.rs": 73}
# `tests/` 下允许存在的**非测试目标**目录：语料 + 历史资产（不是本轮引入的，也不装 main.rs）。
ASSET_DIRS = {"fixtures", "fig2_mixed", "mpl_dialect", "poppler_dialect", "rdkit_dialect",
              "s22_repro", "shape_paint"}

RUNNING_RE = re.compile(r"^\s+(?:Running|Doc-tests) (.+?)(?: \([^)]*\))?\s*$")
RESULT_RE = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed;")

FAILS = []
TMP = None


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1200])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def info(text):
    print(f"INFO  {text}")


def run(args, cwd="/tmp", env=None):
    return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                          cwd=cwd, env=env)          # 不依赖 cwd


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


STYLE_PANEL = """<svg xmlns='http://www.w3.org/2000/svg' width='288pt' height='216pt' \
viewBox='0 0 288 216'>
<defs><style type='text/css'><![CDATA[ .svglite circle { fill: none; } ]]></style></defs>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<rect x='20' y='120' width='60' height='70' style='fill: #3366cc; stroke: none;'/>
<circle cx='60.82' cy='135.96' r='0.65' style='stroke-width: 0.77; fill: #CFA15A;'/>
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</svg>
"""

PLAIN_PANEL = """<svg xmlns='http://www.w3.org/2000/svg' width='288pt' height='216pt' \
viewBox='0 0 288 216'>
<rect width='100%' height='100%' style='stroke: none; fill: #FFFFFF;'/>
<rect x='20' y='120' width='60' height='70' style='fill: #3366cc; stroke: none;'/>
<text x='147.29' y='18.33' style='font-size: 8.80px;'>Title</text>
</svg>
"""


# ── ① 目录树 ────────────────────────────────────────────────────────────────────────
def family_one():
    print("\n== ① 自检只有一个测试目标：tests/selftest/ ==")
    dirs = sorted(p.name for p in TESTS.iterdir() if p.is_dir())
    check("selftest" in dirs, "`tests/selftest/` 存在", dirs)
    check("compose_tests" not in dirs and "doctor_tests" not in dirs,
          "`tests/compose_tests/`、`tests/doctor_tests/` 已删（不留空目录）", dirs)
    extra = sorted(set(dirs) - ASSET_DIRS - {"selftest"})
    check(not extra,
          f"`tests/` 下除 `selftest/` 与语料/历史资产（{len(ASSET_DIRS)} 个）外没有别的目录", extra)
    targets = sorted(p.name for p in TESTS.iterdir()
                     if p.is_dir() and (p / "main.rs").is_file())
    check(targets == ["selftest"],
          "`tests/*/main.rs` 只有一个（cargo 因此只编出一个集成测试目标）", targets)
    for needed in ("common/mod.rs", "common/harness.rs", "compose/mod.rs", "labels/mod.rs",
                   "layout/mod.rs", "transform/mod.rs", "warnings/mod.rs", "refs/mod.rs",
                   "fonts/mod.rs", "doctor/mod.rs", "doctor/convert_cmd.rs", "doctor/corpus.rs"):
        if not (TESTS / "selftest" / needed).is_file():
            check(False, f"按主题的模块文件存在：selftest/{needed}", "缺失")
            return
    check(True, "按主题分模块：common/ compose/ labels/ layout/ transform/ warnings/ refs/ fonts/ doctor/")


# ── ② 用例数 ────────────────────────────────────────────────────────────────────────
def cargo_test_targets():
    """跑一次 `cargo test --offline`，返回 [(目标, passed, failed)] 与原始输出。

    cargo 把 `Running <目标>` 写 **stderr**、把每个目标的 `test result:` 写 **stdout**，
    两条流各自有序、顺序一一对应，所以按顺序 zip（不猜、也不解析进度行的缩进）。
    """
    result = run(["cargo", "test", "--offline"], cwd=SVG_GRID)
    names = [match.group(1).strip() for match in
             (RUNNING_RE.match(line) for line in result.stderr.splitlines()) if match]
    results = [match.groups() for match in
               (RESULT_RE.match(line) for line in result.stdout.splitlines()) if match]
    counts = [(int(passed), int(failed)) for _, passed, failed in results]
    targets = [(names[index] if index < len(names) else "<unknown>", passed, failed)
               for index, (passed, failed) in enumerate(counts)]
    if len(names) != len(counts):
        info(f"目标数 {len(names)} 与结果数 {len(counts)} 不一致（stderr/stdout 未对齐）")
    return targets, result


def family_two():
    print("\n== ② 一条测试都没丢：cargo test 用例数 = 迁移前 ==")
    targets, result = cargo_test_targets()
    if result.returncode != 0:
        check(False, "`cargo test --offline` 全绿", (result.stdout[-1200:] + result.stderr[-600:]))
        return
    by_name = {name: passed for name, passed, _ in targets}
    total = sum(passed for _, passed, _ in targets)
    failed = sum(failed for _, _, failed in targets)
    info("目标与用例数：" + ", ".join(f"{name}={passed}" for name, passed, _ in targets))
    check(failed == 0, "`cargo test --offline` 0 failed", failed)
    check(total == BASELINE, f"用例总数 = 迁移前 {BASELINE}（写死）", f"{total} != {BASELINE}")
    for name, want in COUNTS.items():
        check(by_name.get(name) == want, f"`{name}` = {want} 条", by_name.get(name))
    integ = [name for name, _, _ in targets if name.startswith("tests/")]
    check(integ == ["tests/selftest/main.rs"],
          "集成测试目标只有 `tests/selftest/main.rs`（73 = 60 + 13）", integ)
    check(by_name.get("tests/selftest/main.rs") == 73,
          "`selftest` 73 条 = 迁移前 compose_tests 60 + doctor_tests 13", by_name)


# ── ③ 体积与模块树 ──────────────────────────────────────────────────────────────────
def module_file(path):
    """`path` 的 `mod` 声明应该在哪：同目录的 `mod.rs`，或（`mod.rs` 自己）上一层的 `mod.rs`/`main.rs`。"""
    if path.name == "mod.rs":
        owner = path.parent.parent
        return [owner / "mod.rs", owner / "main.rs"], path.parent.name
    return [path.parent / "mod.rs", path.parent / "main.rs"], path.stem


def family_three():
    print("\n== ③ 每个测试文件 ≤ 20 KB、都在模块树里 ==")
    files = sorted(TESTS.rglob("*.rs"))
    sizes = [(p.stat().st_size, str(p.relative_to(SVG_GRID))) for p in files]
    over = [(name, size) for size, name in sizes if size > LIMIT]
    biggest = sorted(sizes, reverse=True)[:3]
    check(not over, f"`tests/**/*.rs` 每个 ≤ {LIMIT} B（{len(files)} 个文件）", over)
    info("最大三个：" + ", ".join(f"{name} {size}B" for size, name in biggest))

    inc = [str(p.relative_to(SVG_GRID)) for p in files if "include!(" in p.read_text()]
    check(not inc, "没有 `include!(`（否则 rustfmt 看不见那些文件）", inc)

    orphans = []
    for path in files:
        if path.name == "main.rs" and path.parent == TESTS / "selftest":
            continue
        owners, stem = module_file(path)
        texts = [owner.read_text() for owner in owners if owner.is_file()]
        pattern = rf"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(stem)}\s*;"
        if not any(re.search(pattern, text, re.M) for text in texts):
            orphans.append(str(path.relative_to(SVG_GRID)))
    check(not orphans, "每个 .rs 都被上层 `mod` 声明（整棵模块树对 rustfmt 可见）", orphans)

    facade = (TESTS / "selftest/main.rs")
    body = [line.strip() for line in facade.read_text().splitlines()]
    stray = [line for line in body if line and not line.startswith(("//", "mod "))]
    check(not stray, "`selftest/main.rs` 只做 `mod` 声明（门面薄）", stray)
    check(facade.stat().st_size <= ENTRY_LIMIT,
          f"`selftest/main.rs` ≤ {ENTRY_LIMIT} B", f"{facade.stat().st_size} B")


# ── ④ 前缀随调用方 ──────────────────────────────────────────────────────────────────
def style_panel():
    path = TMP / "style.svg"
    write(path, STYLE_PANEL)
    return path


def warning_lines(stderr):
    return [line for line in stderr.splitlines() if ": warning: " in line]


def doctor_counts_v1():
    """doctor 端到端：`<style>` 图 ⇒ `warnings=1` + `V1`（分类器认得子命令前缀）。"""
    corpus = TMP / "corpus"
    write(corpus / "t/style.svg", STYLE_PANEL)
    write(corpus / "t/plain.svg", PLAIN_PANEL)
    write(corpus / "MANIFEST.toml",
          '[[fixture]]\nid = "t/style"\npackage = "test"\nexpect = "pass"\n\n'
          '[[fixture]]\nid = "t/plain"\npackage = "test"\nexpect = "pass"\n')
    result = run([BIN, "doctor", "--fixtures", corpus, "--json"])
    if result.returncode != 0:
        check(False, "doctor 在 2 条小语料上 rc=0（--json）",
              (result.stdout[-800:] + result.stderr[-600:]))
        return
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        check(False, "`doctor --json` 可被 json.loads", f"{error}: {result.stdout[:400]}")
        return
    by_id = {fixture["id"]: fixture for fixture in report["fixtures"]}
    style = by_id.get("t/style", {})
    check(style.get("status") == "pass" and style.get("warnings") == 1
          and style.get("warning_codes") == ["V1"],
          "doctor：`<style>` 图 ⇒ PASS + warnings=1 + V1（告警分类认得 `svg_grid convert: warning:`）",
          style)


def family_four():
    print("\n== ④ 前缀随调用方：`svg_grid convert:` vs `svg_grid_convert:` ==")
    panel = style_panel()

    bad = run([BIN, "convert", "--bogus"])
    check(bad.returncode == 1 and bad.stderr.startswith("svg_grid convert: ")
          and "unknown argument: --bogus" in bad.stderr,
          "`svg_grid convert --bogus` ⇒ rc=1 + 前缀 `svg_grid convert: `",
          f"rc={bad.returncode} stderr={bad.stderr[:300]}")
    check("usage: svg_grid convert --input" in bad.stderr
          and "usage: svg_grid_convert" not in bad.stderr,
          "子命令的 usage 行同样自称为 `svg_grid convert`", bad.stderr[:400])
    check(not bad.stderr.startswith("svg_grid_convert: "),
          "子命令的诊断不再自称 `svg_grid_convert:`", bad.stderr[:200])

    warned = run([BIN, "convert", "--input", panel, "--output", TMP / "tagged.svg"])
    lines = warning_lines(warned.stderr)
    check(warned.returncode == 0 and lines
          and all(line.startswith("svg_grid convert: warning: ") for line in lines),
          "子命令的告警行前缀是 `svg_grid convert: warning: `", warned.stderr[:400])
    check(any("<style>" in line for line in lines), "告警确实是 V1（<style> CSS 块）", lines[:2])

    copied = run([BIN, "convert", "--input", TMP / "tagged.svg",
                  "--output", TMP / "copy.svg"])
    check(copied.returncode == 0 and copied.stdout.startswith("svg_grid convert: ")
          and "already converted" in copied.stdout,
          "stdout 的 `already converted` 行也用同一个前缀（不只 stderr）", copied.stdout[:300])

    old_bad = run([CONVERT, "--bogus"])
    check(old_bad.returncode == 1 and old_bad.stderr.startswith("svg_grid_convert: ")
          and "usage: svg_grid_convert --input" in old_bad.stderr,
          "独立二进制 `svg_grid_convert --bogus` ⇒ rc=1 + 前缀/usage 仍是 `svg_grid_convert`",
          f"rc={old_bad.returncode} stderr={old_bad.stderr[:300]}")
    old_warn = run([CONVERT, "--input", panel, "--output", TMP / "tagged-old.svg"])
    old_lines = warning_lines(old_warn.stderr)
    check(old_warn.returncode == 0 and old_lines
          and all(line.startswith("svg_grid_convert: warning: ") for line in old_lines),
          "独立二进制的告警行仍是 `svg_grid_convert: warning: `", old_warn.stderr[:300])

    if INSTALLED.is_file():
        installed = run([INSTALLED, "convert", "--bogus"])
        check(installed.returncode == 1
              and installed.stderr.startswith("svg_grid convert: "),
              "装机版 `~/.local/bin/svg_grid convert` 同一前缀", installed.stderr[:300])
    else:
        skip("装机版前缀", f"{INSTALLED} 不存在（先跑 install.sh）")

    faces = sorted(p for p in FONT_DIR.rglob("*")
                   if p.is_file() and p.suffix.lower() in {".ttf", ".otf", ".ttc"}) \
        if FONT_DIR.is_dir() else []
    if faces:
        doctor_counts_v1()
    else:
        skip("doctor 端到端告警分类", f"字体目录不可用：{FONT_DIR}（先跑 fontlib/setup.sh）")


# ── ⑤ 回归 ──────────────────────────────────────────────────────────────────────────
# 本轮按裁决只起这三个进程；R18 的 ④ 内部会逐轮跑 R4–R17（含 R15/R16/R17），所以覆盖没有缺口，
# 只是不从 R19 里重复起进程（R18 单跑约 20 分钟，全程约 25 分钟）。
ROUNDS = ["R8", "R9", "R14"]

# R18 的 ① 里「逐字节相同 / 前缀」三类断言被本轮裁决有意取代（前缀按定义不同了）。R18 的 test.py
# 不在本轮的允许改动范围内，故这里逐条列出并只容忍这三类；别的断言红一条都不放过。
SUPERSEDED = ("逐字节相同", "`svg_grid_convert:` 前缀")


def family_five():
    print("\n== ⑤ 回归（R18 的 ④ 内部覆盖 R4–R17） ==")
    started = time.time()
    for round_id in ROUNDS:
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.is_file():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        one = time.time()
        result = run([sys.executable, evidence])
        info(f"{round_id}: rc={result.returncode} 用时 {time.time() - one:.1f}s")
        check(result.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              result.stdout[-800:] + result.stderr[-400:])

    evidence = HERE.parent / "R18" / "test.py"
    one = time.time()
    result = run([sys.executable, evidence])
    info(f"R18: rc={result.returncode} 用时 {time.time() - one:.1f}s")
    failing = [line[len("FAIL  "):] for line in result.stdout.splitlines()
               if line.startswith("FAIL  ")]
    superseded = [label for label in failing
                  if any(key in label for key in SUPERSEDED)]
    other = [label for label in failing if label not in superseded]
    for label in superseded:
        info("R18 上被本轮裁决取代的断言（逐条）：" + label)
    check(not other, "回归 R18：除被取代的前缀/逐字节断言外全绿", other or result.stdout[-600:])
    check(len(superseded) == len(failing),
          "R18 红掉的断言**全部**是前缀/逐字节那几条（没有夹带别的回归）", other)
    info(f"总用时 {time.time() - started:.1f}s")


def main():
    global TMP
    TMP = pathlib.Path(tempfile.mkdtemp(prefix="r19-rounddata-"))
    print(f"# 仓库内：{BIN}\n# 对照组：{CONVERT}\n# 装机版：{INSTALLED}\n# 字体：{FONT_DIR}\n")

    check(BIN.is_file() and os.access(BIN, os.X_OK),
          "`target/debug/svg_grid` 存在且可执行（先 cargo build --offline）", str(BIN))
    check(CONVERT.is_file() and os.access(CONVERT, os.X_OK),
          "`convert/target/debug/svg_grid_convert` 存在且可执行（对照组）", str(CONVERT))
    if FAILS:
        print("\n== 前置不成立：先 `cargo build --offline`（两个 crate） ==")
        return 1

    family_one()
    family_two()
    family_three()
    family_four()
    family_five()

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
