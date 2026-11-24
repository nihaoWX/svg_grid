#!/usr/bin/env python3
"""R57 gate demo: does the R19 test-count gate still have teeth?

R55's review proved the old gate was toothless by feeding it synthetic counts:
it asserted `>=` against a stale snapshot (190 / 60 / 57 / 73), so deleting 388
of the 579 tests still went green. R57 changed the gate to **exact equality**
against the previous round's real counts (read from round-data/R19/test.py).

This demo does not touch the repository and does not run `cargo test`: it
imports `round-data/R19/test.py` as a module, replaces `cargo_test_targets`
with a synthetic count table, and calls `family_two()` — the same code path the
real evidence uses. Every row below must be RED except the exact snapshot.

Usage:  python3 docs/PITFALLS/round-data/R57/gate_demo.py
Exit:   0 = the gate behaves as the table says; 1 = it does not (a real defect).
"""

import contextlib
import importlib.util
import io
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
R19 = HERE.parent / "R19" / "test.py"

LIB = "unittests src/lib.rs"
MAIN = "unittests src/main.rs"
SELF = "tests/selftest/main.rs"


def load_r19():
    spec = importlib.util.spec_from_file_location("r19_evidence", R19)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeRun:
    """Stand-in for `subprocess.CompletedProcess`: only `returncode` is read."""

    returncode = 0
    stdout = ""
    stderr = ""


def run_gate(module, counts):
    """Feed `counts` (a dict target -> passed) to R19's family_two()."""
    targets = [(name, passed, 0) for name, passed in counts.items()]

    def fake_targets():
        return targets, FakeRun()

    module.cargo_test_targets = fake_targets
    module.FAILS.clear()
    buffer = io.StringIO()
    with contextlib.redirect_stdout(buffer):
        module.family_two()
    return list(module.FAILS), buffer.getvalue()


def main():
    if not R19.is_file():
        print(f"FAIL  R19 evidence missing: {R19}")
        return 1
    module = load_r19()
    snapshot = {LIB: module.COUNTS[LIB], MAIN: module.COUNTS[MAIN], SELF: module.COUNTS[SELF]}
    baseline = module.BASELINE
    print(f"# R19 snapshot: BASELINE={baseline} COUNTS={snapshot}")
    if baseline != sum(snapshot.values()):
        print(f"FAIL  snapshot is internally inconsistent: {baseline} != {sum(snapshot.values())}")
        return 1

    total = sum(snapshot.values())
    rows = [
        # (label, counts, must_be_red)
        ("exact snapshot", dict(snapshot), False),
        ("total -1", {**snapshot, LIB: snapshot[LIB] - 1}, True),
        ("total +1", {**snapshot, LIB: snapshot[LIB] + 1}, True),
        ("lib -1", {**snapshot, LIB: snapshot[LIB] - 1}, True),
        ("lib +1", {**snapshot, LIB: snapshot[LIB] + 1}, True),
        ("main -1", {**snapshot, MAIN: snapshot[MAIN] - 1}, True),
        ("main +1", {**snapshot, MAIN: snapshot[MAIN] + 1}, True),
        ("selftest -1", {**snapshot, SELF: snapshot[SELF] - 1}, True),
        ("selftest +1", {**snapshot, SELF: snapshot[SELF] + 1}, True),
        # R55's toothless snapshot: 190/60/57/73 as lower bounds.
        ("R55 lower-bound snapshot", {LIB: 60, MAIN: 57, SELF: 73}, True),
        ("all tests deleted", {LIB: 0, MAIN: 0, SELF: 0}, True),
    ]

    bad = []
    print(f"\n{'row':38} {'total':>5}  {'verdict':7}  red assertions")
    for label, counts, must_be_red in rows:
        fails, _ = run_gate(module, counts)
        red = bool(fails)
        verdict = "RED" if red else "GREEN"
        print(f"{label:38} {sum(counts.values()):5}  {verdict:7}  {len(fails)}")
        for name in fails:
            print(f"    · {name}")
        if red != must_be_red:
            bad.append(f"{label}: expected {'RED' if must_be_red else 'GREEN'}, got {verdict}")

    print()
    if bad:
        for line in bad:
            print("FAIL  " + line)
        return 1
    print(f"PASS  gate has teeth: only the exact snapshot {total} = "
          f"{snapshot[LIB]} + {snapshot[MAIN]} + {snapshot[SELF]} is GREEN; "
          f"{len(rows) - 1} synthetic tables are RED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
