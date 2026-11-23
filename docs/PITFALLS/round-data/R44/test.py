#!/usr/bin/env python3
"""R44 evidence: a figure whose panels repeat a letter must fail closed.

The contract (`INSTRUCTIONS.md`, "必要信息"): when the letters that end up in the
finished figure are not unique — an input's own `data-svg-grid-label` letters plus
whatever this call assigns — `plot-grid` must exit 2, print one line naming the
letter and every input/cell it lands in, and write nothing.

Before R44 the *only* guard was W3 ("this call letters an input that already has
letters"). Composing two composites that were lettered **separately**, with no
`--labels` this time, silently produced `data-svg-grid-label` = 0,1,0 (visible
A,B,A) with rc=0 and an empty stderr.

Families:
  A  the report's three-step repro: rc=2, named letter + inputs + cells, line cap,
     no output file, an existing output is not truncated
  B  `--plan` fails the same way and prints no plan
  C  no false positives: A,B + C / + plain / alone / `--labels A,B` on plain /
     `--labels A,A` (W4b warning) / `--labels ,C`
  D  a single input carrying the same letter twice is rejected too
  E  the repository's own tagged figures (A_heatmap+B_A, then B_B alone)
  F  fail-closed: the same repro against `SVG_GRID_OLD_BIN` must reproduce the
     silent 0,1,0 output, otherwise F fails (SKIP when no old binary is given)

Run: `python3 docs/PITFALLS/round-data/R44/test.py`
Env: SVG_GRID_BIN (default `<repo>/target/debug/svg_grid`), SVG_GRID_OLD_BIN.
Exit code 0 only when every check passes. Offline, no repository writes (temp dirs
only).
"""

from __future__ import annotations

import os
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None

PASSED: list[str] = []
FAILED: list[str] = []

PANEL = """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
  <circle cx="50" cy="40" r="20" fill="#000000"/>
</svg>
"""


def check(ok: bool, label: str, detail: str = "") -> bool:
    if ok:
        PASSED.append(label)
    else:
        FAILED.append(f"{label}: {detail}")
    return ok


def run(binary: pathlib.Path, args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(binary), *args],
        capture_output=True,
        text=True,
        timeout=180,
    )


def compose(binary: pathlib.Path, inputs: list[pathlib.Path], out: pathlib.Path, extra: list[str]) -> subprocess.CompletedProcess:
    args: list[str] = ["plot-grid"]
    for path in inputs:
        args += ["--input", str(path)]
    args += ["--output", str(out), *extra]
    return run(binary, args)


def line_lengths(text: str) -> list[int]:
    return [len(line) for line in text.splitlines()]


def labels_of(svg: str) -> list[str]:
    return re.findall(r'data-svg-grid-label="([^"]*)"', svg)


def main() -> int:
    if not BIN.is_file():
        print(f"FAIL binary not found: {BIN} (build it, or set SVG_GRID_BIN)")
        return 1
    work = pathlib.Path(tempfile.mkdtemp(prefix="r44_"))
    p = work / "p.svg"
    p2 = work / "p2.svg"
    p3 = work / "p3.svg"
    for path in (p, p2, p3):
        path.write_text(PANEL, encoding="utf-8")

    # --- Build the report's starting point: two composites, lettered separately.
    ab = work / "ab.svg"
    c = work / "c.svg"
    first = compose(BIN, [p, p2], ab, ["--width", "600", "--ncol", "2", "--labels"])
    second = compose(BIN, [p3], c, ["--width", "600", "--ncol", "1", "--labels"])
    check(
        first.returncode == 0 and second.returncode == 0,
        "A0 两个组合图各自加字母",
        f"rc={first.returncode}/{second.returncode} {first.stderr}{second.stderr}",
    )

    # --- A. The silent-duplicate composition must now fail closed.
    abc = work / "abc.svg"
    bad = compose(BIN, [ab, c], abc, ["--width", "800", "--ncol", "2"])
    stderr = bad.stderr
    check(bad.returncode == 2, "A1 跨输入重号 rc=2", f"rc={bad.returncode} {stderr}")
    check("duplicate panel letters" in stderr, "A2 文案点名重号", stderr)
    check('"A"' in stderr, "A3 文案点名字母", stderr)
    check("#0" in stderr and "#1" in stderr, "A4 文案点名 input#", stderr)
    check("cell" in stderr and stderr.count("cell") >= 2, "A5 文案点名 cell", stderr)
    check(bool(stderr.strip()), "A6 默认就出声（不依赖 --verbose）", stderr)
    check(all(n <= 200 for n in line_lengths(stderr)), "A7 单行 ≤200 字符", str(line_lengths(stderr)))
    check(not abc.exists(), "A8 不写产物", f"exists={abc.exists()}")
    keep = work / "keep.svg"
    keep.write_text("SENTINEL", encoding="utf-8")
    clobber = compose(BIN, [ab, c], keep, ["--width", "800", "--ncol", "2"])
    check(
        clobber.returncode == 2 and keep.read_text(encoding="utf-8") == "SENTINEL",
        "A9 既有产物不被截断/覆盖",
        f"rc={clobber.returncode} content={keep.read_text(encoding='utf-8')!r}",
    )

    # --- B. `--plan` has the same verdict and prints nothing.
    plan = run(
        BIN,
        [
            "plot-grid",
            "--input", str(ab),
            "--input", str(c),
            "--width", "800",
            "--ncol", "2",
            "--plan",
        ],
    )
    check(plan.returncode == 2, "B1 --plan rc=2", f"rc={plan.returncode} {plan.stderr}")
    check(plan.stdout == "", "B2 --plan 不写计划", plan.stdout)
    check("duplicate panel letters" in plan.stderr, "B3 --plan 同文案", plan.stderr)

    # --- C. No false positives.
    c_only = work / "c_only.svg"
    distinct = compose(BIN, [p3], c_only, ["--width", "600", "--ncol", "1", "--labels", "C"])
    ok_distinct = compose(BIN, [ab, c_only], work / "ok_distinct.svg", ["--width", "800", "--ncol", "2"]) if distinct.returncode == 0 else distinct
    check(
        ok_distinct.returncode == 0,
        "C1 A,B + C 合法",
        f"rc={ok_distinct.returncode} {ok_distinct.stderr}",
    )
    ok_mixed = compose(BIN, [ab, p], work / "ok_mixed.svg", ["--width", "800", "--ncol", "2"])
    check(ok_mixed.returncode == 0, "C2 A,B + 无字母输入合法", f"rc={ok_mixed.returncode} {ok_mixed.stderr}")
    alone = compose(BIN, [ab], work / "alone.svg", ["--width", "600"])
    check(alone.returncode == 0, "C3 单个已带字母的组合图单独跑合法", f"rc={alone.returncode} {alone.stderr}")
    plain_labels = compose(BIN, [p, p2], work / "plain_labels.svg", ["--width", "600", "--ncol", "2", "--labels", "A,B"])
    check(plain_labels.returncode == 0, "C4 两个无字母输入 + --labels A,B 走原路径", f"rc={plain_labels.returncode} {plain_labels.stderr}")
    w4b = compose(BIN, [p, p2], work / "w4b.svg", ["--width", "600", "--ncol", "2", "--labels", "A,A"])
    check(
        w4b.returncode == 0,
        "C5 --labels A,A 保持警告（用户显式写重）",
        f"rc={w4b.returncode} {w4b.stderr}",
    )
    skip_slot = compose(BIN, [ab, p2], work / "skip_slot.svg", ["--width", "800", "--ncol", "2", "--labels", ",C"])
    check(skip_slot.returncode == 0, "C6 --labels ,C 仍放行", f"rc={skip_slot.returncode} {skip_slot.stderr}")

    # --- D. One input carrying the same letter twice.
    dupin = work / "dupin.svg"
    dupin.write_text(
        """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
  <text data-svg-grid-label="0" data-scale="position" x="5" y="12">A</text>
  <text data-svg-grid-label="1" data-scale="position" x="50" y="12">A</text>
</svg>
""",
        encoding="utf-8",
    )
    inner = compose(BIN, [dupin, p], work / "inner.svg", ["--width", "800", "--ncol", "2"])
    check(inner.returncode == 2, "D1 同一输入内部重号 rc=2", f"rc={inner.returncode} {inner.stderr}")
    check("duplicate panel letters" in inner.stderr, "D2 内部重号文案", inner.stderr)

    # --- E. The repository's own tagged figures.
    tagged = ROOT / "tests" / "fig2_mixed" / "out" / "tagged"
    a, b, bee = tagged / "A_heatmap.svg", tagged / "B_A.svg", tagged / "B_B.svg"
    if a.exists() and b.exists() and bee.exists():
        t_ab = work / "t_ab.svg"
        t_c = work / "t_c.svg"
        ok_ab = compose(BIN, [a, b], t_ab, ["--width", "800", "--ncol", "2", "--labels"])
        ok_c = compose(BIN, [bee], t_c, ["--width", "600", "--ncol", "1", "--labels"])
        corpus = compose(BIN, [t_ab, t_c], work / "t_abc.svg", ["--width", "800", "--ncol", "2"])
        check(
            ok_ab.returncode == 0 and ok_c.returncode == 0,
            "E0 真语料两个组合图各自加字母",
            f"{ok_ab.returncode}/{ok_c.returncode}",
        )
        check(corpus.returncode == 2, "E1 真语料组合 rc=2", f"rc={corpus.returncode} {corpus.stderr}")
        check("duplicate panel letters" in corpus.stderr, "E2 真语料文案", corpus.stderr)
    else:
        print("SKIP  E  真语料 tests/fig2_mixed/out/tagged 未生成")

    # --- F. Fail-closed: the old binary must still produce the silent 0,1,0 figure.
    if OLD_BIN is None:
        print("SKIP  F  未设置 SVG_GRID_OLD_BIN（fail-closed 段需要旧二进制）")
    elif not OLD_BIN.is_file():
        check(False, "F0 旧二进制存在", f"not a file: {OLD_BIN}")
    else:
        old_work = pathlib.Path(tempfile.mkdtemp(prefix="r44_old_"))
        o1, o2, o3 = old_work / "p.svg", old_work / "p2.svg", old_work / "p3.svg"
        for path in (o1, o2, o3):
            path.write_text(PANEL, encoding="utf-8")
        o_ab = old_work / "ab.svg"
        o_c = old_work / "c.svg"
        compose(OLD_BIN, [o1, o2], o_ab, ["--width", "600", "--ncol", "2", "--labels"])
        compose(OLD_BIN, [o3], o_c, ["--width", "600", "--ncol", "1", "--labels"])
        o_abc = old_work / "abc.svg"
        old_run = compose(OLD_BIN, [o_ab, o_c], o_abc, ["--width", "800", "--ncol", "2"])
        duplicates = o_abc.exists() and len(labels_of(o_abc.read_text(encoding="utf-8"))) != len(
            set(labels_of(o_abc.read_text(encoding="utf-8")))
        )
        check(
            old_run.returncode == 0 and old_run.stderr.strip() == "" and duplicates,
            "F1 旧二进制仍然静默重号（rc=0、空 stderr、重复 label）",
            f"rc={old_run.returncode} stderr={old_run.stderr!r} exists={o_abc.exists()}",
        )

    total = len(PASSED) + len(FAILED)
    for failure in FAILED:
        print(f"FAIL  {failure}")
    if FAILED:
        print(f"\n== {len(FAILED)} 条不成立（共 {total}）==")
        return 1
    print(f"== 全部成立（{total} 条）==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
