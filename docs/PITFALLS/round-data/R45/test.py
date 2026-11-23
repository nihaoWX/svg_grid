#!/usr/bin/env python3
"""R45 evidence: every warning code `doctor` classifies is produced by a real message.

`doctor` maps stderr phrases to the stable ids of `ARCHITECTURE/warnings.md` §3
(`W1`…`W9`, `V1`) and judges two of them as FAIL (`W7`, `W9`). Before R45 five of
the eleven table entries could never match the product's text:

  W4a  table had `` `--labels` has `` (backticks), the message has `--labels has `
  W5   table had only `may be clipped by the canvas`; the main path says
       `was moved inside the canvas`
  W7   table had `could not be rewritten and were left exactly as written`, the
       message says `could not be rewritten, kept verbatim` ⇒ the W7 FAIL gate was
       dead code (fail-open)
  W3   `already carry letters` is right, but the line is `svg_grid: invalid input:`
       (a hard error, rc=2), which `is_warning_line` skips
  W8   same: `svg_grid: font library error:` (rc=3)

R45 moved the table next to the messages (`src/warnings/codes.rs`), gave a code
several phrases (W5), dropped the two hard-error codes (judged by exit code), and
added the guard test `every_warning_code_still_matches_its_message`, which builds
each message with the production `describe_*` function.

Families:
  A  every classified code is produced by a real CLI run and lands in
     `doctor --json`'s `warning_codes` (W1, W2, W4a, W4b, W5, W6, W7, W9, V1)
  B  the W7 gate is alive: a fixture with `x="inf"` FAILs doctor (rc=1) and the
     machine report carries W7
  C  W3 / W8 are hard errors, not warnings: their lines are not counted and do
     not appear in `warning_codes`; doctor judges them by exit code
  D  the shipped corpus is unchanged: `doctor` still reports 57 pass / 0 fail /
     1 skip (no fixture accidentally trips W7)
  E  fail-closed: with `SVG_GRID_OLD_BIN` pointing at the pre-fix binary the same
     W7 fixture PASSES and `warning_codes` lacks W7 (SKIP when not given)

Run: `python3 docs/PITFALLS/round-data/R45/test.py`
Env: SVG_GRID_BIN (default `<repo>/target/debug/svg_grid`), SVG_GRID_OLD_BIN.
Exit code 0 only when every check passes. Offline, no repository writes (temp dirs
only).
"""

from __future__ import annotations

import json
import os
import pathlib
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

# One fixture per code, each a *real* input that makes the product print it.
FIXTURES = {
    # Untagged dialect SVGs: `doctor` runs the converter first, and a file that
    # already carries a panel box but no `data-scale` group is a half-finished
    # conversion (rc=1) rather than a fixture that reaches the composer.
    "w6": """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect x="-10" y="10" width="40" height="30" fill="#000000"/>
</svg>
""",
    "w7": """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect x="10" y="10" width="40" height="30" fill="#000000"/>
  <rect x="inf" y="10" width="20" height="20" fill="#000000"/>
</svg>
""",
    # W9 must reach the composer: the converter refuses a dangling reference in a
    # dialect SVG (rc=1, "add the missing definition"), so this fixture is already
    # tagged and the converter passes it through untouched.
    "w9": """<svg width="100pt" height="80pt" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#nope)"/></g>
</svg>
""",
    "v1": """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <style>.a{stroke-linejoin:round}</style>
  <rect x="10" y="10" width="80" height="60" fill="#000000"/>
</svg>
""",
}


def check(ok: bool, label: str, detail: str = "") -> bool:
    if ok:
        PASSED.append(label)
    else:
        FAILED.append(f"{label}: {detail}")
    return ok


def run(binary: pathlib.Path, args: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run([str(binary), *args], capture_output=True, text=True, timeout=300)


def doctor_json(binary: pathlib.Path, corpus: pathlib.Path) -> tuple[int, dict | None, str]:
    result = run(binary, ["doctor", "--fixtures", str(corpus), "--json"])
    try:
        return result.returncode, json.loads(result.stdout), result.stderr
    except Exception:  # noqa: BLE001 - the raw text is the evidence
        return result.returncode, None, result.stderr + result.stdout


def codes_of(report: dict | None) -> set[str]:
    if not report:
        return set()
    return {code for fixture in report["fixtures"] for code in fixture["warning_codes"]}


def write_corpus(directory: pathlib.Path, name: str, svg: str) -> pathlib.Path:
    corpus = directory / name
    corpus.mkdir(parents=True, exist_ok=True)
    (corpus / f"{name}.svg").write_text(svg, encoding="utf-8")
    (corpus / "MANIFEST.toml").write_text(
        f'[[fixture]]\nid = "{name}"\npackage = "r45"\nexpect = "pass"\n', encoding="utf-8"
    )
    return corpus


def main() -> int:
    if not BIN.is_file():
        print(f"FAIL binary not found: {BIN} (build it, or set SVG_GRID_BIN)")
        return 1
    work = pathlib.Path(tempfile.mkdtemp(prefix="r45_"))
    panel = work / "p.svg"
    panel2 = work / "p2.svg"
    for path in (panel, panel2):
        path.write_text(PANEL, encoding="utf-8")

    # --- A. Every classified code, produced by a real run.
    # W1: the layout rescales the panel.
    w1 = run(BIN, ["plot-grid", "--input", str(panel), "--width", "400", "--ncol", "1",
                   "--verbose", "--output", str(work / "w1.svg")])
    check("scaled by the layout" in w1.stderr, "A1 W1 真实文案", w1.stderr[-300:])
    # W2: the cell's aspect ratio differs from the panel's.
    w2 = run(BIN, ["plot-grid", "--input", str(panel), "--width", "400", "--height", "100",
                   "--ncol", "1", "--verbose", "--output", str(work / "w2.svg")])
    check("stretched" in w2.stderr, "A2 W2 真实文案", w2.stderr[-300:])
    # W4a: more --labels entries than inputs.
    w4a = run(BIN, ["plot-grid", "--input", str(panel), "--input", str(panel2), "--width", "400",
                    "--ncol", "2", "--labels", "A,B,C", "--verbose",
                    "--output", str(work / "w4a.svg")])
    check("--labels has 3 entries but the grid has 2 inputs" in w4a.stderr,
          "A3 W4a 真实文案（无反引号）", w4a.stderr[-300:])
    # W4b: --labels repeats a letter.
    w4b = run(BIN, ["plot-grid", "--input", str(panel), "--input", str(panel2), "--width", "400",
                    "--ncol", "2", "--labels", "A,A", "--verbose",
                    "--output", str(work / "w4b.svg")])
    check("repeats some letters" in w4b.stderr, "A4 W4b 真实文案", w4b.stderr[-300:])
    # W5: an explicit --plot-margin too small for the letter band.
    w5 = run(BIN, ["plot-grid", "--input", str(panel), "--width", "200", "--ncol", "1",
                   "--plot-margin", "2", "--labels", "--verbose",
                   "--output", str(work / "w5.svg")])
    check("was moved inside the canvas" in w5.stderr, "A5 W5 主路径文案", w5.stderr[-400:])

    # W6/W7/W9/V1 through doctor, so the classification is the product's own.
    for code, name in (("W6", "w6"), ("W7", "w7"), ("W9", "w9"), ("V1", "v1")):
        corpus = write_corpus(work, name, FIXTURES[name])
        rc, report, stderr = doctor_json(BIN, corpus)
        found = codes_of(report)
        check(code in found, f"A6 {code} 出现在 doctor --json 的 warning_codes",
              f"rc={rc} codes={sorted(found)} {stderr[-300:]}")

    # --- B. The W7 gate is alive.
    w7_corpus = write_corpus(work, "w7", FIXTURES["w7"])
    rc, report, stderr = doctor_json(BIN, w7_corpus)
    check(rc == 1, "B1 W7 fixture 让 doctor rc=1（门禁不再是死码）", f"rc={rc} {stderr[-300:]}")
    check("W7" in codes_of(report), "B2 --json 里带 W7", sorted(codes_of(report)))
    text = run(BIN, ["doctor", "--fixtures", str(w7_corpus)])
    check("stderr contains W7" in text.stdout, "B3 文本报告点名 W7 判据", text.stdout[-400:])
    check("kept verbatim" in text.stdout, "B4 报告复述真实文案", text.stdout[-400:])

    # --- C. W3 / W8 are hard errors, not warnings.
    # W3: --labels over an already-lettered composite (rc=2).
    lettered = work / "lettered.svg"
    first = run(BIN, ["plot-grid", "--input", str(panel), "--width", "200", "--ncol", "1",
                      "--labels", "--output", str(lettered)])
    w3 = run(BIN, ["plot-grid", "--input", str(lettered), "--width", "200", "--ncol", "1",
                   "--labels", "B", "--output", str(work / "w3.svg")])
    check(first.returncode == 0 and w3.returncode == 2, "C1 W3 是硬错误 rc=2",
          f"{first.returncode}/{w3.returncode} {w3.stderr[-300:]}")
    check(w3.stderr.startswith("svg_grid: invalid input:"), "C2 W3 行前缀是 invalid input",
          w3.stderr[:200])
    check("already carry letters" in w3.stderr, "C3 W3 文案仍在", w3.stderr[-300:])
    # W8: a label family that is not in the library (rc=3).
    w8 = run(BIN, ["plot-grid", "--input", str(panel), "--width", "200", "--ncol", "1",
                   "--labels", "A", "--label-fontfamily", "NoSuchFamilyXYZ",
                   "--output", str(work / "w8.svg")])
    check(w8.returncode == 3, "C4 W8 是硬错误 rc=3", f"rc={w8.returncode} {w8.stderr[-300:]}")
    check(w8.stderr.startswith("svg_grid: font library error:"), "C5 W8 行前缀是 font library error",
          w8.stderr[:200])
    # Neither line is a warning line, so neither is counted or classified.
    check("warning:" not in w3.stderr and "warning:" not in w8.stderr,
          "C6 两条硬错误都不是 warning 行", f"{w3.stderr[:120]} | {w8.stderr[:120]}")

    # --- D. The shipped corpus is unchanged.
    fixtures = ROOT / "tests" / "fixtures"
    if fixtures.is_dir():
        rc, report, stderr = doctor_json(BIN, fixtures)
        summary = report["summary"] if report else None
        check(summary == {"pass": 57, "fail": 0, "skip": 1},
              "D1 全量语料仍是 57 pass / 0 fail / 1 skip", f"{summary} {stderr[-300:]}")
        check("W7" not in codes_of(report) and "W9" not in codes_of(report),
              "D2 没有 fixture 意外触发 W7/W9", sorted(codes_of(report)))
    else:
        print("SKIP  D  仓库里没有 tests/fixtures")

    # --- E. Fail-closed against the pre-fix binary.
    if OLD_BIN is None:
        print("SKIP  E  未设置 SVG_GRID_OLD_BIN（fail-closed 段需要旧二进制）")
    elif not OLD_BIN.is_file():
        check(False, "E0 旧二进制存在", f"not a file: {OLD_BIN}")
    else:
        old_work = pathlib.Path(tempfile.mkdtemp(prefix="r45_old_"))
        old_corpus = write_corpus(old_work, "w7", FIXTURES["w7"])
        rc, report, stderr = doctor_json(OLD_BIN, old_corpus)
        old_codes = codes_of(report)
        check(rc == 0, "E1 旧二进制放过 W7 fixture（rc=0，门禁是死码）",
              f"rc={rc} {stderr[-300:]}")
        check("W7" not in old_codes, "E2 旧二进制 --json 里没有 W7", sorted(old_codes))
        # W4a / W5 were dead too: the old table cannot classify them.
        old_w4a = run(OLD_BIN, ["plot-grid", "--input", str(panel), "--input", str(panel2),
                                "--width", "400", "--ncol", "2", "--labels", "A,B,C",
                                "--verbose", "--output", str(old_work / "w4a.svg")])
        check("--labels has 3 entries" in old_w4a.stderr,
              "E3 旧二进制同样打印 W4a 文案（片段却匹配不上）", old_w4a.stderr[-300:])

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
