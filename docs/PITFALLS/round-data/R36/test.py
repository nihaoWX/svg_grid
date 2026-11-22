#!/usr/bin/env python3
"""R36 — W8 硬错误行：预算按真实拼串计算，`available:` 与链回显都不许丢。

覆盖 D5d（D5c 评审 FAIL 的两个阻断，同一个根因）：
① 单字符链 + 7×100 字符族名库 ⇒ 必须含 ` (available: `，回显 `"X"`，单行 ≤200
   （D5c 才是 100 字符、整段 available: 丢失）
② `Noto` + 真实 Noto CJK 长名库 ⇒ 链回显必须是 `"Noto"` 而不是 `"…"`，≤200
③ 矩阵扫描：8 链 × 4 库 ⇒ over200=0、missing_available=0、echo_collapsed=0
   （父提交 4d737b2 基线：over200=10、echo_collapsed=5）
④ 长度回归：X*78/80/81/86/200 全 200 字符；短链 194/195/195 不得变长
⑤ 回退链语义不回退：11 条可用链全 rc=0、stderr 0 字节
⑥ 失败一律不写产物
无参数；用 `target/debug/svg_grid`；只写系统临时目录。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
BIN = SVG_GRID / "target/debug/svg_grid"
FONT_DIR = pathlib.Path.home() / "svg_grid/fontlib"
REPRO_LIB = pathlib.Path("/tmp/d5c_eval/repro_lib")
NOTO_LIB = pathlib.Path("/tmp/d5c_eval/fonts_noto7")
LONG_LIB = pathlib.Path("/tmp/d5c/long")
PANEL = pathlib.Path("/tmp/d5c_eval/panel.svg")
MAX_LINE = 200
FAILS = []
SKIPS = []

ECHO = re.compile(r'--label-fontfamily (".*?"|.*?) is not in the font library')

PANEL_SVG = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
             'viewBox="0 0 100 80"><rect data-panel-box="main" x="0" y="0" '
             'width="100" height="80" fill="none" stroke="none"/>'
             '<circle cx="50" cy="40" r="3" fill="#000"/></svg>')

CHAINS = ["X", "AB", "ABC", "ABCD", "Noto", "思源", "NoSuchFont_xyz", "Arial, NoSuch"]
LIBS = [("repro_lib", REPRO_LIB), ("fonts_noto7", NOTO_LIB),
        ("long", LONG_LIB), ("default", None)]


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        FAILS.append(label)
        if detail:
            print("      " + str(detail).replace("\n", "\n      ")[:900])


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")
    SKIPS.append(label)


OUT = pathlib.Path(tempfile.mkdtemp(prefix="r36-")) / "out.svg"


def run(args, panel, target=None):
    return subprocess.run([str(BIN), "plot-grid", "--input", str(panel),
                           "--output", str(target or OUT),
                           "--width", "300", "--ncol", "1", "--labels", *map(str, args)],
                          capture_output=True, text=True, cwd="/tmp")


def longest(stderr):
    return max((len(line) for line in stderr.splitlines()), default=0)


def echo_of(stderr):
    found = ECHO.search(stderr)
    return found.group(1) if found else ""


def main():
    if not BIN.exists():
        print(f"FAIL  前置：缺二进制 {BIN} —— 先 cargo build")
        return 1
    if not (FONT_DIR.is_dir() and any(FONT_DIR.glob("*.ttf"))):
        print(f"FAIL  前置：字体库 {FONT_DIR} 为空 —— 先跑 fontlib/setup.sh")
        return 1
    if not PANEL.exists():
        print(f"FAIL  前置：缺面板 {PANEL}")
        return 1
    panel = PANEL

    # ── ① 阻断 1：单字符链 + 长族名库，available: 不许整段丢
    print("== ① 单字符链 + 7×100 字符族名库 ⇒ 保留 available:、回显 \"X\"、≤200 ==")
    if not (REPRO_LIB.is_dir() and any(REPRO_LIB.glob("*.ttf"))):
        skip("① 单字符链 + 长族名库", f"缺 {REPRO_LIB}")
    else:
        got = run(["--font-dir", REPRO_LIB, "--label-fontfamily", "X"], panel)
        check(got.returncode == 3, "① rc=3", f"rc={got.returncode}")
        check(longest(got.stderr) <= MAX_LINE,
              f"① 单行最长 {longest(got.stderr)} ≤ {MAX_LINE}", got.stderr[:400])
        check(" (available: " in got.stderr,
              "① 仍保留 ` (available: `（D5c 整段丢失）", got.stderr[:400])
        check(echo_of(got.stderr) == '"X"',
              f"① 链回显仍是 \"X\"（实得 {echo_of(got.stderr)!r}）", got.stderr[:400])

    # ── ② 阻断 2：长库不许把链回显压成 "…"
    print("\n== ② `Noto` + 真实 Noto CJK 长名库 ⇒ 回显 \"Noto\" 而非 \"…\"、≤200 ==")
    if not (NOTO_LIB.is_dir() and any(NOTO_LIB.glob("*.ttf"))):
        skip("② Noto 长名库回显", f"缺 {NOTO_LIB}")
    else:
        for chain in ("Noto", "AB", "ABC", "ABCD"):
            got = run(["--font-dir", NOTO_LIB, "--label-fontfamily", chain], panel)
            check(got.returncode == 3, f"② [{chain}] rc=3", f"rc={got.returncode}")
            check(longest(got.stderr) <= MAX_LINE,
                  f"② [{chain}] 单行最长 {longest(got.stderr)} ≤ {MAX_LINE}", got.stderr[:400])
            check(echo_of(got.stderr) == f'"{chain}"',
                  f"② [{chain}] 回显完整（实得 {echo_of(got.stderr)!r}）", got.stderr[:400])
            check("available:" in got.stderr, f"② [{chain}] 仍列出可用族", got.stderr[:400])

    # ── ③ 矩阵扫描
    print("\n== ③ 矩阵扫描 8 链 × 4 库 ⇒ over200=0、missing_available=0、echo_collapsed=0 ==")
    missing_libs = [name for name, path in LIBS
                    if path is not None and not (path.is_dir() and any(path.glob("*.ttf")))]
    if missing_libs:
        skip("③ 矩阵", f"缺字体库 {missing_libs}")
    else:
        over = absent = collapsed = 0
        cells = []
        for lib_name, lib_path in LIBS:
            for chain in CHAINS:
                extra = [] if lib_path is None else ["--font-dir", lib_path]
                got = run([*extra, "--label-fontfamily", chain], panel)
                if got.returncode == 0:
                    cells.append(f"{lib_name}/{chain}=rc0")
                    continue
                line = longest(got.stderr)
                has = "available:" in got.stderr
                echo = echo_of(got.stderr)
                fell = echo.strip('"') == "…"
                over += line > MAX_LINE
                absent += not has
                collapsed += fell
                cells.append(f"{lib_name}/{chain}={line}{'' if has else '!'}"
                             f"{'~' if fell else ''}")
        print("      " + "  ".join(cells))
        check(over == 0, f"③ over200={over}（父 4d737b2 基线 10）", "  ".join(cells))
        check(absent == 0, f"③ missing_available={absent}", "  ".join(cells))
        check(collapsed == 0,
              f"③ echo_collapsed={collapsed}（父 4d737b2 基线 5）", "  ".join(cells))

    # ── ④ 长度回归
    print("\n== ④ 长度回归：X*78/80/81/86/200 全 200；短链 194/195/195 ==")
    for n in (78, 80, 81, 86, 200):
        target = OUT.with_name(f"x{n}.svg")
        target.unlink(missing_ok=True)
        got = run(["--label-fontfamily", "X" * n], panel, target)
        check(got.returncode == 3, f"④ X*{n} ⇒ rc=3", f"rc={got.returncode}")
        check(longest(got.stderr) == MAX_LINE,
              f"④ X*{n} 单行 {longest(got.stderr)} == {MAX_LINE}", got.stderr[:400])
        check(not target.exists(), f"④ X*{n} 不写产物")
    for value, expected in (("NoSuchFont_xyz", 194),
                            ("Helvetica, Liberation Sans, DejaVu Sans", 195),
                            ("A,B,C,D,E,F,G,H", 195)):
        got = run(["--label-fontfamily", value], panel)
        check(got.returncode == 3, f"④ [{value}] ⇒ rc=3", f"rc={got.returncode}")
        check(longest(got.stderr) == expected,
              f"④ [{value}] 单行 {longest(got.stderr)} == {expected}（不得变长）", got.stderr[:400])
        check("is not in the font library" in got.stderr and "available:" in got.stderr
              and "--list-fonts" in got.stderr,
              f"④ [{value}] 三个必需片段仍在", got.stderr[:400])

    # ── ⑤ 回退链语义不回退
    print("\n== ⑤ 回退链语义不回退：可用链全 rc=0、stderr 0 字节 ==")
    for value in ("Helvetica, Arial, sans-serif", "Liberation Sans, Arial", "serif, NoSuch",
                  "Arial, Liberation Sans, sans-serif", "Arial, NoSuch", "NoSuch, Arial",
                  "arial", " Arial , sans-serif ", "", ",,", '"Times New Roman"'):
        got = run(["--label-fontfamily", value], panel)
        check(got.returncode == 0 and got.stderr == "",
              f"⑤ [{value!r}] ⇒ rc=0、stderr 0 字节",
              f"rc={got.returncode} {got.stderr[:300]}")

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
