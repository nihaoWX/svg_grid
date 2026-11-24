#!/usr/bin/env python3
"""R63 evidence — W7 ("kept verbatim") is now visible without `--verbose`,
while every other warning family stays silent; the R62 review's two surviving
test gaps (ellipse `ry` axis, three-layer gradient `href` chain) are covered by
lib tests that go red on the mutants.

Families
  A  visibility: a W7-only run prints exactly its W7 line plus the footer with
     no `--verbose`; W1/W2/W5 are silent without it and loud with it (paired
     fixtures); notes stay verbose-only; stdout/rc/output bytes unchanged.
  B  normal path: every tracked `tests/**/*.svg` (214) through `convert` and
     every `tests/fixtures/**/*.svg` (58) through `plot-grid --width 600
     --ncol 1` — no warning line on stderr, every rc=0 run has 0-byte stderr,
     and rc + output bytes equal the pre-R63 binary (/tmp/r50base @e0c6033).
  C  rc + output bytes parity on 9 named representative runs (convert/plot-grid,
     including a fail-closed input), pre-fix vs this build.
  T  teeth: the default pre-R63 build at /tmp/r63_before_bin (built from
     552eeef) stays silent on the W7 fixture — and because family A runs on
     `SVG_GRID_PIN_BIN` when it is set, that invocation exits 1.

Run:  python3 docs/PITFALLS/round-data/R63/test.py
      SVG_GRID_PIN_BIN=/tmp/r63_before_bin python3 .../R63/test.py   # must be red
Exit: 0 all green, 1 at least one check red, 2 missing binary (fail closed).
Env:  SVG_GRID_BIN (this build), SVG_GRID_PIN_BIN (pre-fix build, default
      /tmp/r63_before_bin; when set, family A runs on it), SVG_GRID_PRE_BIN
      (parity baseline, default /tmp/r50base/target/release/svg_grid).
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
DATA = Path(__file__).resolve().parent / "data"
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r63_before_bin"))
# Family A is the "visibility" face of the contract: point it at the pre-fix
# build and the W7 assertions MUST go red (that is the teeth, T only records it).
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
TMP = Path(tempfile.mkdtemp(prefix="r63-evidence-"))
FOOTER = ("svg_grid: warning: non-fatal warning(s) — the exit code and the "
          "output file are unchanged")
W7 = "could not be rewritten, kept verbatim"
PANEL = DATA / "panel.svg"
W7SVG = DATA / "w7.svg"
PASSED, FAILED = [], []


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + ((" — " + str(detail)) if detail and not ok else ""))
    return ok


def run(binary, args):
    result = subprocess.run([str(binary)] + [str(a) for a in args],
                            capture_output=True)
    return result.returncode, result.stdout, result.stderr


def grid(binary, name, inputs, extra):
    out = TMP / (name + ".svg")
    args = ["plot-grid"]
    for source in inputs:
        args += ["--input", source]
    args += ["--output", out] + list(extra)
    rc, stdout, stderr = run(binary, args)
    return rc, out.read_bytes() if out.exists() else None, stderr, stdout


def conv(binary, name, source):
    out = TMP / (name + ".svg")
    rc, stdout, stderr = run(binary, ["convert", "--input", source,
                                      "--output", out, "--force"])
    return rc, out.read_bytes() if out.exists() else None, stderr, stdout


def warning_lines(stderr):
    text = stderr.decode("utf-8", "replace")
    return [line for line in text.splitlines()
            if line.startswith(("svg_grid: warning:", "svg_grid convert: warning:"))]


def family_a():
    # A1: a W7-only run is loud without --verbose, with the unchanged footer.
    rc, data, err, _ = grid(VIS, "a1-w7-quiet", [W7SVG],
                            ["--width", "100", "--height", "80", "--ncol", "1",
                             "--plot-margin", "0"])
    lines = err.decode("utf-8", "replace").splitlines()
    check(rc == 0 and data is not None, "A1/W7 quiet rc=0 and output written", str(rc))
    check(len(lines) == 2 and lines[1] == FOOTER,
          "A1/W7 quiet prints exactly the W7 line + footer", lines)
    check(len(lines) == 2 and lines[0].startswith(
              "svg_grid: warning: 2 attributes could not be rewritten, kept verbatim: "),
          "A1/W7 quiet names the two verbatim attributes", lines[:1])
    check("note:" not in err.decode("utf-8", "replace"),
          "A1/W7 quiet must not print the note", lines)

    # A2: --verbose still expands W7 fully and adds the note.
    rc_v, data_v, err_v, _ = grid(VIS, "a1-w7-verbose", [W7SVG],
                                  ["--width", "100", "--height", "80", "--ncol", "1",
                                   "--plot-margin", "0", "--verbose"])
    check(rc_v == 0 and data_v == data,
          "A1/verbose keeps rc and output bytes identical", (rc_v, data_v == data))
    check(W7 in err_v.decode("utf-8", "replace")
          and "svg_grid: note:" in err_v.decode("utf-8", "replace"),
          "A1/verbose expands the W7 line and the note", err_v[-300:])

    # A3: the other families stay silent by default but are loud with --verbose.
    pairs = [
        ("W1", [PANEL], ["--width", "1000", "--height", "800", "--ncol", "1",
                         "--plot-margin", "0"], "scaled by the layout"),
        ("W2", [PANEL, PANEL], ["--width", "300", "--ncol", "2",
                                "--rel-widths", "2,1", "--plot-margin", "0"],
         "are stretched"),
        ("W5", [PANEL] * 5, ["--width", "500", "--height", "160", "--ncol", "5",
                             "--plot-margin", "0", "--labels"],
         "was moved inside the canvas"),
    ]
    for code, inputs, extra, phrase in pairs:
        _, _, quiet, _ = grid(VIS, "a3-" + code, inputs, extra)
        _, _, loud, _ = grid(VIS, "a3-" + code + "-v", inputs, extra + ["--verbose"])
        check(quiet == b"", "A3/%s is silent without --verbose" % code, quiet[:200])
        check(phrase in loud.decode("utf-8", "replace"),
              "A3/%s is loud with --verbose (pair is not vacuous)" % code, loud[-200:])

    # A4: convert shares emit(), but a W7-free input stays silent.
    rc_c, _, err_c, _ = conv(VIS, "a4-convert-panel", PANEL)
    check(rc_c == 0 and err_c == b"",
          "A4/convert of a clean panel is still 0-byte stderr", err_c[:200])


def family_b():
    tracked = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--",
                              "tests/**/*.svg"], capture_output=True,
                             text=True, check=True).stdout.split()
    fixtures = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--",
                               "tests/fixtures/**/*.svg"], capture_output=True,
                              text=True, check=True).stdout.split()
    check(len(tracked) == 214, "B/tracked tests corpus is 214", len(tracked))
    check(len(fixtures) == 58, "B/fixtures corpus is 58", len(fixtures))

    silenced, failing, diffs, warned = 0, 0, [], []
    silenced_fix, failing_fix, hard_errors = 0, 0, []
    for index, rel in enumerate(tracked):
        source = ROOT / rel
        rc, data, stderr, _ = conv(BIN, "b-c%d" % index, source)
        rc_old, data_old, _, _ = conv(PRE, "b-o%d" % index, source)
        if (rc, data) != (rc_old, data_old):
            diffs.append(rel)
        if warning_lines(stderr):
            warned.append(rel)
        if rc == 0 and not stderr:
            silenced += 1
        if rc != 0:
            failing += 1
            if not stderr.startswith(b"svg_grid convert: "):
                hard_errors.append(rel)
    for index, rel in enumerate(fixtures):
        source = ROOT / rel
        rc, data, stderr, _ = grid(BIN, "b-p%d" % index, [source],
                                   ["--width", "600", "--ncol", "1"])
        rc_old, data_old, _, _ = grid(PRE, "b-q%d" % index, [source],
                                      ["--width", "600", "--ncol", "1"])
        if (rc, data) != (rc_old, data_old):
            diffs.append(rel)
        if warning_lines(stderr):
            warned.append(rel)
        if rc == 0 and not stderr:
            silenced_fix += 1
        elif rc != 0:
            failing_fix += 1
            if not stderr.startswith(b"svg_grid: "):
                hard_errors.append(rel)
    check(not warned, "B/no warning line on the normal path (272 runs)", warned[:5])
    check(not hard_errors,
          "B/every fail-closed run reports a hard error, not a warning", hard_errors[:3])
    check(silenced == len(tracked) - failing,
          "B/every rc=0 convert run has 0-byte stderr",
          "silent=%d failing=%d" % (silenced, failing))
    check(silenced_fix == len(fixtures) - failing_fix,
          "B/every rc=0 fixture plot-grid run has 0-byte stderr",
          "silent=%d failing=%d" % (silenced_fix, failing_fix))
    check(not diffs, "B/rc and output bytes equal the pre-R63 build (272 runs)",
          "differences=%d first=%s" % (len(diffs), diffs[:3]))
    print("     (B: convert=%d rc0=%d fail-closed=%d; fixtures=58 rc0=%d "
          "fail-closed=%d)" % (len(tracked), silenced, failing, silenced_fix,
                               failing_fix))


def parity_cases():
    return [
        ("c-w7-quiet", W7SVG, ["--width", "100", "--height", "80", "--ncol", "1",
                               "--plot-margin", "0"]),
        ("c-w1", PANEL, ["--width", "1000", "--height", "800", "--ncol", "1",
                         "--plot-margin", "0"]),
        ("c-w2", [PANEL, PANEL], ["--width", "300", "--ncol", "2",
                                  "--rel-widths", "2,1", "--plot-margin", "0"]),
        ("c-w5", [PANEL] * 5, ["--width", "500", "--height", "160", "--ncol", "5",
                               "--plot-margin", "0", "--labels"]),
        ("c-w7-verbose", W7SVG, ["--width", "100", "--height", "80", "--ncol", "1",
                                 "--plot-margin", "0", "--verbose"]),
    ]


def family_c():
    for name, inputs, extra in parity_cases():
        inputs = inputs if isinstance(inputs, list) else [inputs]
        rc, data, _, _ = grid(BIN, name, inputs, extra)
        rc_old, data_old, _, _ = grid(PRE, name + "-old", inputs, extra)
        check((rc, data) == (rc_old, data_old),
              "C/plot-grid rc+bytes unchanged: " + name, (rc, rc_old))
    conv_cases = [
        ("c-convert-w7", W7SVG),
        ("c-convert-area", ROOT / "tests/fixtures/ggplot2/area_ribbon.svg"),
        ("c-convert-gradient", ROOT / "tests/fixtures/ggplot2/gradient_fill.svg"),
        ("c-convert-mathtext", ROOT / "tests/fixtures/matplotlib/mathtext.svg"),
    ]
    for name, source in conv_cases:
        rc, data, _, _ = conv(BIN, name, source)
        rc_old, data_old, _, _ = conv(PRE, name + "-old", source)
        check((rc, data) == (rc_old, data_old),
              "C/convert rc+bytes unchanged: " + name, (rc, rc_old))


def family_t():
    check(PIN.exists(), "T/pre-fix build exists: %s" % PIN, PIN)
    rc, _, err, _ = grid(PIN, "t-pin-w7", [W7SVG],
                         ["--width", "100", "--height", "80", "--ncol", "1",
                          "--plot-margin", "0"])
    check(rc == 0 and err == b"",
          "T/pre-fix build is silent on the W7 fixture (teeth)", err[:200])
    # The suite is red either way when the pin is injected: A1 runs on VIS=PIN.
    print("     (T: SVG_GRID_PIN_BIN is %s; with it set, family A must be red)"
          % ("set -> family A uses the pin" if os.environ.get("SVG_GRID_PIN_BIN")
             else "unset, family A uses SVG_GRID_BIN"))


def main():
    for binary, label in ((BIN, "SVG_GRID_BIN"), (PRE, "pre-fix baseline")):
        if not binary.exists():
            print("missing %s: %s" % (label, binary), file=sys.stderr)
            return 2
    print("BIN=%s\nPRE=%s\nVIS=%s\n" % (BIN, PRE, VIS))
    family_a()
    family_b()
    family_c()
    family_t()
    print("\n%d passed, %d failed" % (len(PASSED), len(FAILED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
