#!/usr/bin/env python3
"""R35 — W8 硬错误行在病态输入下仍 ≤200 字符。

覆盖 D5c（D5b 评审 FAIL 的两个阻断）：
① 长单名走 FINAL 兜底分支：X*78..X*86、X*200 ⇒ rc=3、单行 ≤200（旧 201）
② 字体库里是长族名：L80/L90/L100 ⇒ rc=3、单行 ≤200（旧 201），族名被截断而非丢弃
③ 短链回归：194/195/195 字符，不得变长
④ 回退链语义不回退：11 条可用链全 rc=0、stderr 0 字节
⑤ 失败一律不写产物
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
LONG_DIR = pathlib.Path("/tmp/d5c/long")
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


def longest(stderr):
    return max((len(line) for line in stderr.splitlines()), default=0)


def main():
    if not BIN.exists():
        print(f"FAIL  前置：缺二进制 {BIN} —— 先 cargo build")
        return 1
    if not (FONT_DIR.is_dir() and any(FONT_DIR.glob("*.ttf"))):
        print(f"FAIL  前置：字体库 {FONT_DIR} 为空 —— 先跑 fontlib/setup.sh")
        return 1
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r35-"))
    one = tmp / "one.svg"
    one.write_text(PANEL)

    def dead(args, target):
        return run(["--input", one, "--output", target, "--width", 300, "--ncol", 1,
                    "--labels", *args])

    # ── ① 长单名：clamp 后仍超时必须继续降级
    print("== ① 长单名走 FINAL 兜底分支 ⇒ 单行 ≤200（旧 201）==")
    for n in (78, 80, 81, 86, 200):
        target = tmp / f"x{n}.svg"
        got = dead(["--label-fontfamily", "X" * n], target)
        check(got.returncode == 3, f"① X*{n} ⇒ rc=3", f"rc={got.returncode}")
        check(longest(got.stderr) <= MAX_LINE,
              f"① X*{n} 单行最长 {longest(got.stderr)} ≤ {MAX_LINE}", got.stderr[:400])
        check(not target.exists(), f"① X*{n} 不写产物")
        check("is not in the font library" in got.stderr and "available:" in got.stderr
              and "--list-fonts" in got.stderr,
              f"① X*{n} 文案仍点名链、可用族与修法", got.stderr[:400])
    got = dead(["--label-fontfamily", "X" * 81], tmp / "x81b.svg")
    check(got.stderr.count("X") >= 20,
          f"① X*81 的链回显保留可辨识前缀（{got.stderr.count('X')} 个 X）", got.stderr[:400])

    # ── ② 字体库里是长族名：available: 列表也要能截断
    print("\n== ② 长可用族名 ⇒ 单行 ≤200（旧 201），族名截断而非丢弃 ==")
    if not (LONG_DIR.is_dir() and any(LONG_DIR.glob("*.ttf"))):
        skip("② 长族名字体库", f"缺 {LONG_DIR}（fontTools 造 nameID 1/4/6/16 为 L*n 的 ttf）")
    else:
        for n in (80, 90, 100):
            target = tmp / f"l{n}.svg"
            got = dead(["--font-dir", LONG_DIR, "--label-fontfamily", "NoSuchFont_xyz"], target)
            check(got.returncode == 3, f"② L{n} ⇒ rc=3", f"rc={got.returncode}")
            check(longest(got.stderr) <= MAX_LINE,
                  f"② L{n} 单行最长 {longest(got.stderr)} ≤ {MAX_LINE}", got.stderr[:400])
            check(not target.exists(), f"② L{n} 不写产物")
            check("available:" in got.stderr and "…" in got.stderr,
                  f"② L{n} 长族名被截断（保留前缀 + …）", got.stderr[:400])
            check("NoSuchFont_xyz" in got.stderr,
                  f"② L{n} 链回显仍完整（长库不挤掉链）", got.stderr[:400])

    # ── ③ 短链回归：不得变长
    print("\n== ③ 短链回归：194/195/195 字符，不得变长 ==")
    for value, expected in (("NoSuchFont_xyz", 194),
                            ("Helvetica, Liberation Sans, DejaVu Sans", 195),
                            ("A,B,C,D,E,F,G,H", 195)):
        got = dead(["--label-fontfamily", value], tmp / "short.svg")
        check(got.returncode == 3, f"③ [{value}] ⇒ rc=3", f"rc={got.returncode}")
        check(longest(got.stderr) == expected,
              f"③ [{value}] 单行 {longest(got.stderr)} == {expected}（不得变长）", got.stderr[:400])

    # ── ④ 回退链语义不回退
    print("\n== ④ 回退链语义不回退：可用链全 rc=0、stderr 0 字节 ==")
    for value in ("Helvetica, Arial, sans-serif", "Liberation Sans, Arial", "serif, NoSuch",
                  "Arial, Liberation Sans, sans-serif", "Arial, NoSuch", "NoSuch, Arial",
                  "arial", " Arial , sans-serif ", "", ",,", '"Times New Roman"'):
        got = dead(["--label-fontfamily", value], tmp / "live.svg")
        check(got.returncode == 0 and got.stderr == "",
              f"④ [{value!r}] ⇒ rc=0、stderr 0 字节",
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
