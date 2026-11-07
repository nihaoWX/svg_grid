#!/usr/bin/env python3
"""R24 的 round-data —— I/O 报错必须点名文件（评估报告 M5 的那一半）。

判据（都可机械跑）：
  ① 读输入失败：stderr 含那个输入路径，rc=1；
  ② 四种输出（`.svg` / `.svgz` / `.png` / `.avif`）失败：stderr 含**各自的**目标路径，rc=1，且形状统一
     （`I/O error at <path>: …`）；
  ③ `.svgz` 输入解压失败：stderr 含该 `.svgz` 路径；
  ④ 退出码分档没被改坏：未转换的原始 fixture ⇒ `invalid input` rc=2；语料目录不存在 ⇒ rc=3；
  ⑤ 设计上"只有确实没有路径时才用无路径形状"：`src/` 里不再有元组形式 `SvgGridError::Io(`
     （构造一律走 `at()` 或 `from`）；单测锁住两种显示形状；
  ⑥ 回归 R14 / R15 / R17。

无参数；约 12 分钟（⑥ 的 R17 占大头）。只读：临时产物只在系统临时目录里。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
BIN = SVG_GRID / "target/debug/svg_grid"
FIXTURE = SVG_GRID / "tests/fixtures/ggplot2/bars_stacked.svg"

fails = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"  [{detail}]"))
    if not ok:
        fails.append(label)
    return ok


def run(args):
    return subprocess.run([str(BIN), *args], capture_output=True, text=True)


def main():
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r24-"))
    tagged = tmp / "tagged.svg"
    made = run(["convert", "--input", str(FIXTURE), "--output", str(tagged)])
    if made.returncode != 0:
        print(f"FAIL  前置：convert 失败 rc={made.returncode} {made.stderr[:200]}")
        return 1

    print("== ① 读输入失败：点名那个路径 ==")
    missing = tmp / "does-not-exist.svg"
    r = run(["plot-grid", "--input", str(missing), "--output", str(tmp / "o.svg")])
    check(r.returncode == 1, "rc=1（I/O 仍是运行期失败这一档）", f"rc={r.returncode}")
    check(str(missing) in r.stderr, "stderr 含输入路径", r.stderr[:200])
    check(r.stderr.count("\n") == 1, "一条失败一行（不摊开）", r.stderr[:300])

    print("\n== ② 四种输出：各自的路径 + 统一形状 ==")
    bad = tmp / "no-such-dir"
    for flag, name in [
        (None, "svg"),
        ("--output-svgz", "svgz"),
        ("--output-png", "png"),
        ("--output-avif", "avif"),
    ]:
        dest = bad / f"a.{name}"
        args = ["plot-grid", "--input", str(tagged), "--output", str(tmp / "ok.svg")]
        if flag:
            args += [flag, str(dest)]
        else:
            args[args.index("--output") + 1] = str(dest)
        r = run(args)
        line = next((l for l in r.stderr.splitlines() if "I/O error" in l), "")
        check(r.returncode == 1 and str(dest) in line,
              f"{name}：stderr 含目标路径且 rc=1", f"rc={r.returncode} {r.stderr[:200]}")
        check(re.match(r"^svg_grid: I/O error at .*: ", line) is not None,
              f"{name}：形状统一为 `I/O error at <path>: …`", line[:200])

    print("\n== ③ .svgz 输入坏掉也点名 ==")
    broken = tmp / "broken.svgz"
    broken.write_bytes(b"not gzip at all")
    r = run(["plot-grid", "--input", str(broken), "--output", str(tmp / "o2.svg")])
    check(r.returncode == 1 and str(broken) in r.stderr,
          ".svgz 解压失败：stderr 含该 .svgz 路径", f"rc={r.returncode} {r.stderr[:200]}")

    print("\n== ④ 退出码分档没被改坏 ==")
    raw = run(["plot-grid", "--input", str(FIXTURE), "--output", str(tmp / "o3.svg")])
    check(raw.returncode == 2 and "invalid input" in raw.stderr,
          "未转换的原始 fixture ⇒ `invalid input` rc=2", f"rc={raw.returncode} {raw.stderr[:160]}")
    no_corpus = run(["doctor", "--fixtures", str(tmp / "no-corpus")])
    check(no_corpus.returncode == 3, "语料目录不存在 ⇒ rc=3（配置错这一档）", f"rc={no_corpus.returncode}")

    print("\n== ⑤ 设计：没有路径才用无路径形状 ==")
    sources = list((SVG_GRID / "src").rglob("*.rs"))
    tuple_forms = [
        f"{p.relative_to(SVG_GRID)}:{n}"
        for p in sources
        for n, line in enumerate(p.read_text(encoding="utf-8").splitlines(), 1)
        if "SvgGridError::Io(" in line or "CliError::from(SvgGridError::Io(" in line
    ]
    check(not tuple_forms, "src/ 里没有元组形式 `SvgGridError::Io(`（一律走 at() / from）", tuple_forms[:5])
    at_uses = sum(p.read_text(encoding="utf-8").count("SvgGridError::at(") for p in sources)
    check(at_uses >= 6, f"有路径的 I/O 都走 at()（实测 {at_uses} 处）", at_uses)
    t = subprocess.run(["cargo", "test", "--offline", "--quiet", "--lib", "error::"],
                       cwd=str(SVG_GRID), capture_output=True, text=True)
    passed = re.search(r"(\d+) passed", t.stdout + t.stderr)
    check(t.returncode == 0 and passed and int(passed.group(1)) >= 2,
          "单测锁住两种显示形状（有路径 / 无路径）", (t.stdout + t.stderr)[-300:])

    print("\n== ⑥ 回归 ==")
    for round_id in ("R14", "R15", "R17"):
        script = SVG_GRID / f"docs/PITFALLS/round-data/{round_id}/test.py"
        r = subprocess.run([sys.executable, str(script)], capture_output=True, text=True)
        tail = [l for l in r.stdout.splitlines() if l.strip()][-1:] or [""]
        check(r.returncode == 0, f"{round_id} 仍全绿", f"rc={r.returncode} {tail[0][:160]}")

    print()
    if fails:
        print(f"== {len(fails)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
