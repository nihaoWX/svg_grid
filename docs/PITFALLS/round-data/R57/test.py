#!/usr/bin/env python3
"""R57 evidence: give the R19 test-count gate teeth back, and make temp_path() unique.

Families
  A  the R19 snapshot is EXACT: `BASELINE` / `COUNTS` in round-data/R19/test.py
     equal what a real `cargo test --offline` prints this round (789 = 491 + 62 + 236),
     and the assertions use `==`. A lower bound (`>=`) is the R55 defect: it tolerated
     deleting 388 of 579 tests and still went green (R55 review P1-1).
  B  the gate has teeth: `gate_demo.py` feeds synthetic count tables into R19's
     `family_two()` and every table except the exact snapshot must be RED
     (total ± 1, lib ± 1, main ± 1, selftest ± 1, R55's 190 shape,
     all tests deleted).
  C  `temp_path()` is unique per process: 8 concurrent runs that all report the same
     pid (an `LD_PRELOAD` shim pins `getpid()`), run the same test with one shared
     `TMPDIR`, and must not delete each other's files. Every process must pass.
     Set `R57_PRE_FIX_SELFTEST=<selftest binary built before R57>` to run the same
     8-run table against the old pid-only naming: it must go RED.
  D  the R55 metadata was corrected: corpus 214 (not 220), the 20000 B claim is scoped
     to the R8/R9 gated set, and both known residuals are registered.

Ground truth: every number below is measured on this checkout, not assumed. The
A family runs the real suite; B reuses the same code path the evidence runs; C
observes real concurrent processes; D greps the file the review P3 pointed at.

Fail-closed: a missing release binary, a missing selftest build, or a missing C
compiler for the pid shim is a hard failure (exit 1) unless `--allow-skip`.

Usage
  python3 docs/PITFALLS/round-data/R57/test.py
  R57_PRE_FIX_SELFTEST=/tmp/r57old/target/debug/deps/selftest-<hash> python3 ...
  python3 docs/PITFALLS/round-data/R57/test.py --allow-skip
"""

import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[4]
ROUND = ROOT / "docs" / "PITFALLS" / "round-data"
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "release" / "svg_grid")
PRE_FIX = os.environ.get("R57_PRE_FIX_SELFTEST", "")
ALLOW_SKIP = "--allow-skip" in sys.argv
TMP = pathlib.Path(tempfile.mkdtemp(prefix="r57-"))

LIB = "unittests src/lib.rs"
MAIN = "unittests src/main.rs"
SELF = "tests/selftest/main.rs"
TARGETS = [LIB, MAIN, SELF]
# The test the R55-era flake hit: it composes two panels through the CLI into a
# PNG and reads it back, several times per run, under names that only the pid
# used to separate.
FILTER = "display_none_cannot_be_restored_by_a_descendant"
RUNS = 8

PASSED, FAILED, SKIPPED = [], [], []


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("ok   " if ok else "FAIL ") + label + (f"  [{detail}]" if detail else ""))
    return ok


def skip(label, why):
    if ALLOW_SKIP:
        SKIPPED.append(label)
        print(f"skip {label}  [{why}]")
        return True
    return check(False, label, f"refusing to skip: {why} (pass --allow-skip)")


def read_r19():
    return (ROUND / "R19" / "test.py").read_text(encoding="utf-8")


def r19_snapshot(source):
    baseline = re.search(r"^BASELINE = (\d+)", source, re.M)
    pairs = re.findall(r'"(unittests src/lib\.rs|unittests src/main\.rs|tests/selftest/main\.rs)": (\d+)', source)
    if not baseline or len(pairs) != 3:
        return None, None
    return int(baseline.group(1)), {name: int(value) for name, value in pairs}


def cargo_test_counts():
    """Run the real suite and read what cargo prints per target."""
    result = subprocess.run(
        ["cargo", "test", "--offline"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    counts, current = {}, None
    for line in result.stdout.splitlines():
        found = re.match(r"\s+Running (unittests src/(?:lib|main)\.rs|tests/selftest/main\.rs)", line)
        if found:
            current = found.group(1)
            continue
        found = re.match(r"test result: ok\. (\d+) passed; 0 failed", line)
        if found and current:
            counts[current] = int(found.group(1))
            current = None
    return result, counts


def selftest_executable():
    """Ask cargo where the selftest binary is (no glob races on old builds)."""
    result = subprocess.run(
        ["cargo", "test", "--offline", "--test", "selftest", "--no-run"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    for line in reversed((result.stdout + result.stderr).splitlines()):
        found = re.search(r"Executable .*\((?P<path>[^)]+)\)", line)
        if found:
            path = pathlib.Path(found.group("path"))
            return path if path.is_absolute() else ROOT / path
    return None


def fake_pid_shim():
    """A C shim that pins `getpid()`: 8 runs then look like one reused pid."""
    if shutil.which("cc") is None:
        return None
    source = TMP / "fakepid.c"
    source.write_text(
        "#include <sys/types.h>\n#include <unistd.h>\n"
        "pid_t getpid(void) { return 4242; }\n",
        encoding="utf-8",
    )
    shim = TMP / "fakepid.so"
    result = subprocess.run(
        ["cc", "-shared", "-fPIC", "-O2", "-o", str(shim), str(source)],
        capture_output=True,
        text=True,
    )
    return shim if result.returncode == 0 else None


def run_eight(binary, shim, label):
    """8 concurrent same-pid runs sharing one TMPDIR; returns (rcs, outputs)."""
    tempdir = TMP / f"tmp-{label}"
    tempdir.mkdir()
    env = dict(os.environ, LD_PRELOAD=str(shim), TMPDIR=str(tempdir))
    procs = [
        subprocess.Popen(
            [str(binary), FILTER],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            env=env,
        )
        for _ in range(RUNS)
    ]
    outputs = [proc.communicate(timeout=180)[0] for proc in procs]
    return [proc.returncode for proc in procs], outputs


def family_a():
    print("\n── A: the R19 snapshot equals the real suite ──")
    source = read_r19()
    baseline, snapshot = r19_snapshot(source)
    if baseline is None:
        return check(False, "A/R19 snapshot parses (BASELINE + three COUNTS)")
    check(True, "A/R19 snapshot parses", f"BASELINE={baseline} COUNTS={snapshot}")
    check(
        baseline == sum(snapshot.values()),
        "A/R19 snapshot is internally consistent",
        f"{baseline} == {sum(snapshot.values())}",
    )
    check("== BASELINE" in source, "A/R19 total asserts exact equality (`== BASELINE`)")
    check(">= BASELINE" not in source, "A/R19 has no lower-bound total (`>= BASELINE` gone)")
    check("== want" in source, "A/R19 per-target asserts exact equality (`== want`)")
    check('">= 190"' not in source, "A/R55's lower-bound snapshot (190) is gone")

    if not BIN.is_file():
        return skip("A/real suite count", f"missing release binary {BIN}")
    result, counts = cargo_test_counts()
    detail = f"rc={result.returncode} counts={counts}"
    if not check(
        result.returncode == 0 and sorted(counts) == sorted(TARGETS),
        "A/real `cargo test --offline` has the three targets",
        detail,
    ):
        return False
    check(True, "A/real suite counts", f"{sum(counts.values())} = " + " + ".join(str(counts[t]) for t in TARGETS))
    check(counts[SELF] == 236, "A/selftest target is 236", str(counts[SELF]))
    for target in TARGETS:
        check(
            counts[target] == snapshot[target],
            f"A/{target} snapshot is exact this round",
            f"snapshot {snapshot[target]} vs measured {counts[target]}",
        )
    check(
        sum(counts.values()) == baseline == 789,
        "A/total snapshot is exact this round",
        f"snapshot {baseline} vs measured {sum(counts.values())}",
    )
    return True


def family_b():
    print("\n── B: the gate is RED for every table but the exact snapshot ──")
    demo = ROUND / "R57" / "gate_demo.py"
    if not demo.is_file():
        return check(False, "B/gate_demo.py exists", str(demo))
    result = subprocess.run([sys.executable, str(demo)], capture_output=True, text=True, cwd=ROOT)
    print("\n".join("    " + line for line in result.stdout.rstrip().splitlines()[-4:]))
    if not check(result.returncode == 0, "B/gate_demo exits 0", (result.stdout + result.stderr).strip()[-300:]):
        return False
    rows = {}
    for line in result.stdout.splitlines():
        found = re.match(r"^(?P<label>.+?)\s+(?P<total>\d+)\s{2}(?P<verdict>RED|GREEN)\s+(?P<red>\d+)$", line)
        if found:
            rows[found.group("label").strip()] = (found.group("verdict"), int(found.group("red")))
    expected = {
        "exact snapshot": "GREEN",
        "total -1": "RED",
        "total +1": "RED",
        "lib -1": "RED",
        "lib +1": "RED",
        "main -1": "RED",
        "main +1": "RED",
        "selftest -1": "RED",
        "selftest +1": "RED",
        "R55 lower-bound snapshot": "RED",
        "all tests deleted": "RED",
    }
    for label, want in expected.items():
        got = rows.get(label)
        check(
            got is not None and got[0] == want,
            f"B/{label} is {want}",
            f"row={got}",
        )
    check(
        len([1 for label, (verdict, _) in rows.items() if verdict == "GREEN"]) == 1,
        "B/only one synthetic table is GREEN",
        f"{sum(1 for v, _ in rows.values() if v == 'GREEN')} green of {len(rows)}",
    )
    return True


def family_c():
    print("\n── C: temp_path() survives 8 same-pid processes ──")
    source = (ROOT / "tests" / "selftest" / "common" / "mod.rs").read_text(encoding="utf-8")
    check("process_tag()" in source, "C/temp_path uses process_tag()")
    check(
        "SystemTime::now()" in source and "as_nanos()" in source,
        "C/process_tag carries a nanosecond stamp",
        "the pid alone repeats across processes",
    )
    if not BIN.is_file():
        return skip("C/concurrent temp_path", f"missing release binary {BIN}")
    binary = selftest_executable()
    if binary is None or not binary.is_file():
        return skip("C/concurrent temp_path", f"no selftest binary ({binary})")
    shim = fake_pid_shim()
    if shim is None:
        return skip("C/concurrent temp_path", "no C compiler for the getpid() shim")

    rcs, outputs = run_eight(binary, shim, "fixed")
    check(
        all(rc == 0 for rc in rcs) and all("1 passed; 0 failed" in out for out in outputs),
        f"C/{RUNS} same-pid runs all pass (no file is deleted by a sibling)",
        f"rcs={rcs}",
    )
    check(
        len(set(rcs)) == 1 and rcs[0] == 0,
        "C/no run is red",
        f"rcs={rcs}",
    )

    if PRE_FIX:
        print(f"\n── C-red: the same table against a pre-R57 selftest ({PRE_FIX}) ──")
        old = pathlib.Path(PRE_FIX)
        if not old.is_file():
            return check(False, "C/pre-R57 selftest exists", PRE_FIX)
        old_rcs, old_outputs = run_eight(old, shim, "prefix")
        red = [rc for rc in old_rcs if rc != 0]
        check(
            bool(red),
            "C/pre-R57 naming is RED under one pinned pid",
            f"{len(red)}/{RUNS} red rcs={old_rcs}",
        )
        names = sorted(
            {
                line.split("panicked at ")[1].split(":", 1)[0]
                for out in old_outputs
                for line in out.splitlines()
                if "panicked at " in line
            }
        )
        check(bool(names), "C/pre-R57 panics name the file", ", ".join(names[:2]))
    else:
        print("note C-red skipped: set R57_PRE_FIX_SELFTEST=<pre-R57 selftest binary> to run it")
    return True


def family_d():
    print("\n── D: the R55 metadata matches what the review measured ──")
    meta = (ROUND / "R55" / "meta.toml").read_text(encoding="utf-8")
    check("语料 214 文件" in meta, "D/R55 corpus says 214")
    check("语料 220 文件" not in meta, "D/R55 has no stale 220")
    check(
        "受 R8/R9 门禁的集合" in meta,
        "D/R55 20000 B claim is scoped to the gated set",
    )
    check(
        "round-data/*/test.py` 不在该门禁内" in meta,
        "D/R55 names the evidence scripts as out of gate",
    )
    check("approx_equals(total)" in meta and "1656/28" in meta, "D/residual 1 is registered", "pure scale(2 3)")
    check("matrix(2 0 0 3 24 60)" in meta and "924/28" in meta and "12762/255" in meta, "D/residual 1b is registered")
    check(
        "href 基元素上的 transform 渲染器不认" in meta and "未被像素验证" in meta,
        "D/residual 2 is registered",
        "href inheritance is not pixel-verified",
    )
    return True


def main():
    print(f"R57 evidence  root={ROOT}")
    print(f"  binary   {BIN} (exists={BIN.is_file()})")
    print(f"  selftest pre-fix probe: {PRE_FIX or '(unset)'}")
    family_a()
    family_b()
    family_c()
    family_d()
    print(f"\n== {len(PASSED)} passed, {len(FAILED)} failed, {len(SKIPPED)} skipped ==")
    for label in FAILED:
        print("   red: " + label)
    shutil.rmtree(TMP, ignore_errors=True)
    return 1 if FAILED else 0


if __name__ == "__main__":
    sys.exit(main())
