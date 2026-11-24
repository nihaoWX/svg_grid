#!/usr/bin/env python3
"""R53 evidence: `classify` must collect every hit code, so user content cannot mask W9/W7.

R45 moved the warning-phrase table into `src/warnings/codes.rs`; `classify` kept
`WARNING_CODES.iter().find(...)`, i.e. it reported only the table-order FIRST hit.
W9 prints the dangling id verbatim and W7 prints the kept attribute value verbatim,
so a phrase of any earlier code inside that user content stole W9/W7 and doctor
(fail-open) turned a FAIL into a PASS.  R53 collects all hits, fixes the footer
exclusion in `is_warning_line`, derives the note-guard phrases from the table,
removes R17's dead `KNOWN_CODE_WORDS`, and appends the R45 line-drift findings.

Families
  A  the three repros, one fixture per corpus so doctor's rc is biting:
     `--labels has 3 entries` / `was moved inside the canvas` inside a dangling id
     must still fail the W9 criterion; `--labels has 3` as a kept value must still
     fail W7.  rc AND the exact `--json` code list are asserted.
  B  single source of truth: `rules_tests.rs` derives its phrases from
     `WARNING_CODES` (no stale fourth copy, no `already carry letters`).
  C  R17's dead `KNOWN_CODE_WORDS` is gone.
  D  (SVG_GRID_OLD_BIN) the pre-R53 binary must be RED on A: it passes the W9
     fixtures and the W7-mask fixture with the stolen code.  Skipped when unset.
  E  file sizes of every touched Rust file stay under the 20000 B gate.

Env
  SVG_GRID_BIN      binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN  pre-fix binary; supplies family D (skipped when unset)
"""

import json
import os
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None

PASSED = FAILED = SKIPPED = 0


def check(ok, label, detail=""):
    global PASSED, FAILED
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        FAILED += 1
        if detail:
            print("      " + detail.replace("\n", "\n      "))
    else:
        PASSED += 1


def skip(label, why):
    global SKIPPED
    SKIPPED += 1
    print(f"SKIP  {label} ({why})")


PANEL = (
    '<svg width="100pt" height="80pt" viewBox="0 0 100 80" '
    'xmlns="http://www.w3.org/2000/svg">\n'
    '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>\n'
    "{BODY}\n</svg>\n"
)

W9_BODY = (
    '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" '
    'fill="url(#{ID})"/></g>'
)
W7_BODY = (
    '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="#3366cc"/>'
    '<rect x="inf" y="10" width="20" height="20" fill="#000000"/>'
    '<path d="M0 0 --labels has 3" fill="none" stroke="#000000"/></g>'
)


def make_fixture(tmp, name, body):
    """One-fixture corpus: <dir>/<pkg>/<name>.svg + MANIFEST.toml."""
    directory = tmp / name
    (directory / "ggplot2").mkdir(parents=True)
    (directory / "MANIFEST.toml").write_text(
        f'[[fixture]]\nid = "ggplot2/{name}"\npackage = "ggplot2"\nexpect = "pass"\n',
        encoding="utf-8",
    )
    (directory / "ggplot2" / f"{name}.svg").write_text(
        PANEL.replace("{BODY}", body), encoding="utf-8"
    )
    return directory


def doctor(binary, directory, as_json=True):
    args = [str(binary), "doctor", "--fixtures", str(directory)]
    if as_json:
        args.append("--json")
    result = subprocess.run(args, capture_output=True, text=True, cwd=ROOT)
    payload = None
    if as_json:
        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError:
            payload = None
    return result, payload


def codes_of(payload, fid):
    for fixture in (payload or {}).get("fixtures", []):
        if fixture["id"] == fid:
            return fixture["warning_codes"]
    return None


def assert_repro(tmp, name, body, expected_codes, criterion):
    directory = make_fixture(tmp, name, body)
    result, _ = doctor(BIN, directory, as_json=False)
    payload_result, payload = doctor(BIN, directory)
    fid = f"ggplot2/{name}"
    check(
        result.returncode == 1 and f"FAIL  {fid}  check" in result.stdout,
        f"A/{name} fails at the check step",
        f"rc={result.returncode}\n{result.stdout}\n{result.stderr}",
    )
    check(
        f"\nfailures:\n  {fid}  (step=check, expect=pass)" in result.stdout,
        f"A/{name} is named in the failures block",
        result.stdout,
    )
    check(
        criterion in result.stdout,
        f"A/{name} names the stolen criterion {criterion}",
        result.stdout,
    )
    check(
        payload_result.returncode == 1
        and codes_of(payload, fid) == expected_codes
        and f'"warning_codes": {json.dumps(expected_codes)}' in payload_result.stdout,
        f"A/{name} reports exactly {expected_codes}",
        f"codes={codes_of(payload, fid)}",
    )
    return directory, payload_result, payload


def main():
    if not BIN.exists():
        print(f"missing binary: {BIN}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory(prefix="r53-evidence-") as raw:
        tmp = pathlib.Path(raw)

        # ── A: the three repros, plus the clean W7 control ───────────────────
        _, w9_result, w9_payload = assert_repro(
            tmp, "w9", W9_BODY.replace("{ID}", "--labels has 3 entries"), ["W4a", "W9"], "stderr contains W9"
        )
        assert_repro(
            tmp, "w9w5", W9_BODY.replace("{ID}", "was moved inside the canvas"), ["W5", "W9"], "stderr contains W9"
        )
        assert_repro(tmp, "w7mask", W7_BODY, ["W6", "W4a", "W7"], "stderr contains W7")
        # the clean W7 fixture alone must keep failing for W7 (not be resurrected as W4a)
        clean = make_fixture(
            tmp,
            "w7clean",
            '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="#3366cc"/>'
            '<rect x="inf" y="10" width="20" height="20" fill="#000000"/></g>',
        )
        clean_result, payload = doctor(BIN, clean)
        check(
            clean_result.returncode == 1 and codes_of(payload, "ggplot2/w7clean") == ["W6", "W7"],
            "A/w7clean still fails W7 without any masking phrase",
            f"rc={clean_result.returncode} codes={codes_of(payload, 'ggplot2/w7clean')}",
        )

        # ── B: the note guard derives from the table, not a hand-written copy ─
        rules = (ROOT / "src" / "warnings" / "rules_tests.rs").read_text(encoding="utf-8")
        check(
            "WARNING_CODES" in rules and "fn warning_phrases()" in rules,
            "B/rules_tests phrases derive from WARNING_CODES",
            "warning_phrases() or WARNING_CODES missing",
        )
        check(
            "already carry letters" not in rules and "could not be rewritten, kept verbatim" not in rules,
            "B/rules_tests keeps no stale phrase copy",
            "a stale hand-written phrase is still in rules_tests.rs",
        )

        # ── C: R17's dead variable is gone ───────────────────────────────────
        r17 = (ROOT / "docs/PITFALLS/round-data/R17/test.py").read_text(encoding="utf-8")
        check(
            "KNOWN_CODE_WORDS" not in r17,
            "C/R17 dead KNOWN_CODE_WORDS removed",
            "KNOWN_CODE_WORDS still present",
        )

        # ── D: the pre-fix binary must be RED on A ───────────────────────────
        if OLD_BIN is None:
            skip("D/pre-fix binary is RED on the repros", "SVG_GRID_OLD_BIN unset")
        else:
            for name, body, expected in (
                ("w9", W9_BODY.replace("{ID}", "--labels has 3 entries"), ["W4a"]),
                ("w9w5", W9_BODY.replace("{ID}", "was moved inside the canvas"), ["W5"]),
                ("w7mask", W7_BODY, ["W6", "W4a"]),
            ):
                directory = make_fixture(tmp, name, body)
                result, payload = doctor(OLD_BIN, directory)
                fid = f"ggplot2/{name}"
                check(
                    result.returncode == 0 and codes_of(payload, fid) == expected,
                    f"D/pre-fix {name}: rc=0 and stolen {expected}",
                    f"rc={result.returncode} codes={codes_of(payload, fid)}",
                )
            # the W9 fixture's JSON on the fixed binary must list BOTH codes, not just the
            # later one: that is what makes "first match" impossible to restore.
            check(
                codes_of(w9_payload, "ggplot2/w9") == ["W4a", "W9"] and w9_result.returncode == 1,
                "D/fixed binary keeps the later code W9 next to the earlier W4a",
                f"codes={codes_of(w9_payload, 'ggplot2/w9')}",
            )

        # ── E: no touched file crosses the 20000 B red line ──────────────────
        for relative in (
            "src/warnings/codes.rs",
            "src/warnings/codes_tests.rs",
            "src/warnings/rules_tests.rs",
            "tests/selftest/doctor/masking.rs",
            "tests/selftest/doctor/mod.rs",
        ):
            size = (ROOT / relative).stat().st_size
            check(size <= 20000, f"E/{relative} <= 20000 B", f"{size} B")

        codes_rs = (ROOT / "src" / "warnings" / "codes.rs").read_text(encoding="utf-8")
        check(
            "for (code, phrases) in WARNING_CODES" in codes_rs
            and ".find(|(_, phrases)" not in codes_rs,
            "E/classify collects every code (no first-match `find`)",
            "classify is back to a single first match",
        )

    print(f"\n== {PASSED} passed, {FAILED} failed, {SKIPPED} skipped ==")
    return 1 if FAILED else 0


if __name__ == "__main__":
    sys.exit(main())
