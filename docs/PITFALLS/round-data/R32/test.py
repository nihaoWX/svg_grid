#!/usr/bin/env python3
"""R32 evidence: `plot-grid` converts an untagged input in memory.

Run from this directory:  python3 test.py
Exit status 0 = every assertion passed.  Requires target/debug/svg_grid (or
`svg_grid` on PATH).  Only writes under the system temporary directory.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = ROOT / "target" / "debug" / "svg_grid"
if not BIN.exists():
    found = shutil.which("svg_grid")
    BIN = pathlib.Path(found) if found else BIN

PASS = 0
FAIL = 0


def check(name: str, condition: bool, detail: str = "") -> None:
    global PASS, FAIL
    if condition:
        PASS += 1
        print(f"PASS  {name}")
    else:
        FAIL += 1
        print(f"FAIL  {name}" + (f": {detail}" if detail else ""))


def run(*args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run([str(BIN), *args], capture_output=True)


def plot(src: pathlib.Path, out: pathlib.Path, *extra: str) -> subprocess.CompletedProcess[bytes]:
    return run("plot-grid", "--input", str(src), "--output", str(out), *extra)


def plan(*args: str) -> subprocess.CompletedProcess[bytes]:
    return run("plot-grid", *args, "--plan")


SVGLITE = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="288pt" height="216pt" viewBox="0 0 288 216">
<style>.svglite circle { stroke: #000; fill: none; }</style>
<rect width="100%" height="100%" style="stroke: none; fill: #FFFFFF;"/>
<circle cx="60.82" cy="135.96" r="4" style="stroke-width: 1.5;"/>
</svg>"""

MULTI_SUBPATH = """<svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
<path d="M 10 10 L 30 10 L 30 30 L 10 30 Z M 60 60 L 90 60 L 90 90 L 60 90 Z M 10 60 L 25 60 L 25 75 L 10 75 Z" fill="#ff0000" stroke="#000000" stroke-width="2" fill-rule="evenodd"/>
</svg>"""

UNSUPPORTED = """<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<symbol id="s"><rect width="10" height="10"/></symbol>
<circle cx="50" cy="40" r="5"/>
</svg>"""


def cell_sizes(stdout: bytes) -> list[tuple[float, float]]:
    sizes = []
    for line in stdout.decode().splitlines():
        if not line.startswith("#"):
            continue
        token = line.split("cell ", 1)[1].split()[0]
        w, h = token.split("x")
        sizes.append((float(w), float(h)))
    return sizes


def main() -> int:
    if not BIN.exists():
        print(f"SKIP: no svg_grid binary at {BIN}")
        return 0
    with tempfile.TemporaryDirectory(prefix="svg-grid-r32-") as tmp:
        tmp = pathlib.Path(tmp)

        # 1. an untagged svglite panel: rc=0, cell present, byte-equal to manual convert
        svglite = tmp / "svglite.svg"
        svglite.write_text(SVGLITE)
        tagged_manual = tmp / "svglite.tagged.svg"
        converted = run("convert", "--input", str(svglite), "--output", str(tagged_manual))
        manual_out = tmp / "svglite.manual.svg"
        auto_out = tmp / "svglite.auto.svg"
        manual = plot(tagged_manual, manual_out, "--width", "500", "--ncol", "1", "--plot-margin", "0")
        auto = plot(svglite, auto_out, "--width", "500", "--ncol", "1", "--plot-margin", "0")
        check("svglite manual convert rc=0", converted.returncode == 0)
        check("svglite automatic rc=0", auto.returncode == 0, auto.stderr.decode())
        check(
            "svglite auto output is byte-identical to manual convert",
            manual_out.read_bytes() == auto_out.read_bytes(),
        )
        check("svglite output has cell 0", b'data-svg-grid-cell="0"' in auto_out.read_bytes())

        # 2. a real matplotlib 3.11 probe (11 glyph paths), byte-equal to manual convert
        mpl = ROOT / "tests/fig2_mixed/out/probe/probe_imshow_cb.svg"
        if mpl.exists():
            mpl_tagged = tmp / "mpl.tagged.svg"
            mpl_manual = tmp / "mpl.manual.svg"
            mpl_auto = tmp / "mpl.auto.svg"
            mpl_conv = run("convert", "--input", str(mpl), "--output", str(mpl_tagged))
            plot(mpl_tagged, mpl_manual, "--width", "600", "--ncol", "1", "--plot-margin", "0")
            mpl_run = plot(mpl, mpl_auto, "--width", "600", "--ncol", "1", "--plot-margin", "0")
            check("matplotlib manual convert rc=0", mpl_conv.returncode == 0)
            check("matplotlib automatic rc=0", mpl_run.returncode == 0, mpl_run.stderr.decode())
            check(
                "matplotlib auto output is byte-identical to manual convert",
                mpl_manual.read_bytes() == mpl_auto.read_bytes(),
            )
        else:
            check("matplotlib probe fixture present", False, str(mpl))

        # 3. a flattened multi-subpath fill keeps its three M subpaths (holes intact)
        multi = tmp / "multi.svg"
        multi.write_text(MULTI_SUBPATH)
        multi_out = tmp / "multi.auto.svg"
        multi_run = plot(multi, multi_out, "--width", "400", "--ncol", "1", "--plot-margin", "0")
        multi_svg = multi_out.read_text() if multi_out.exists() else ""
        check("multi-subpath automatic rc=0", multi_run.returncode == 0, multi_run.stderr.decode())
        check("multi-subpath keeps at least three contours", multi_svg.count(" M ") >= 2)

        # 4. a tagged input is never converted (byte-identical to the pre-D4 baseline)
        tagged = ROOT / "tests/fig2_mixed/out/tagged/A_heatmap.svg"
        tagged_out = tmp / "tagged.svg"
        tagged_run = plot(tagged, tagged_out, "--width", "600", "--ncol", "1")
        check("tagged input rc=0", tagged_run.returncode == 0, tagged_run.stderr.decode())
        check("tagged input stderr is empty", tagged_run.stderr == b"")
        check(
            "tagged output matches the pre-D4 baseline",
            hashlib.sha256(tagged_out.read_bytes()).hexdigest()
            == "ca2f6503a18e58c52bb8991d6a8fcd97ee3d96e40e5e6c1f8b903552df51a59f"
            if tagged_out.exists()
            else False,
        )

        # 5. an unsupported untagged input fails closed: rc=2, reason + fix, no output
        unsupported = tmp / "unsupported.svg"
        unsupported.write_text(UNSUPPORTED)
        no_output = tmp / "unsupported.out.svg"
        bad = plot(unsupported, no_output, "--width", "400", "--ncol", "1", "--plot-margin", "0")
        bad_err = bad.stderr.decode()
        check("unsupported rc=2", bad.returncode == 2)
        check("unsupported writes no output", not no_output.exists())
        check("unsupported error names auto-conversion", "auto-conversion failed for" in bad_err)
        check("unsupported error names the finding", "svg/symbol" in bad_err)
        check("unsupported error names the fix", "svg_grid convert" in bad_err)
        bad_plan = plan("--input", str(unsupported), "--width", "400", "--ncol", "1", "--plot-margin", "0")
        check("unsupported --plan rc=2", bad_plan.returncode == 2)
        check(
            "unsupported --plan explains auto-conversion",
            "auto-conversion failed for" in bad_plan.stderr.decode(),
        )

        # 6. --plan on an untagged input has the same canvas/cell/s as the run
        plan_run = plan("--input", str(svglite), "--width", "500", "--ncol", "1", "--plot-margin", "0")
        plan_text = plan_run.stdout.decode()
        check("plan rc=0", plan_run.returncode == 0, plan_run.stderr.decode())
        check("plan stderr is empty by default", plan_run.stderr == b"")
        check("plan canvas is 500x375", "canvas 500x375" in plan_text, plan_text)
        check("plan cell is 500x375", cell_sizes(plan_run.stdout) == [(500.0, 375.0)])
        check("plan scale is 1.736x1.736", "s=1.736x1.736" in plan_text)

        # 7. source bytes/SHA-256/mtime unchanged; default silent, --verbose explains
        src = tmp / "source.svg"
        src.write_text(SVGLITE)
        before = src.read_bytes()
        before_hash = hashlib.sha256(before).hexdigest()
        before_mtime = src.stat().st_mtime_ns
        quiet_out = tmp / "quiet.svg"
        quiet = plot(src, quiet_out, "--width", "500", "--ncol", "1", "--plot-margin", "0")
        after = src.read_bytes()
        check("default run rc=0", quiet.returncode == 0)
        check("default stderr is 0 bytes", quiet.stderr == b"")
        check("source bytes unchanged", before == after)
        check("source SHA-256 unchanged", before_hash == hashlib.sha256(after).hexdigest())
        check("source mtime unchanged", before_mtime == src.stat().st_mtime_ns)
        loud_out = tmp / "loud.svg"
        loud = plot(src, loud_out, "--width", "500", "--ncol", "1", "--plot-margin", "0", "--verbose")
        loud_err = loud.stderr.decode()
        check("--verbose restores the auto-conversion warning", "auto-converted" in loud_err)
        check("--verbose adds the in-memory note", "svg_grid: note:" in loud_err)
        check("--verbose changes no output byte", quiet_out.read_bytes() == loud_out.read_bytes())
        check(
            "every stderr line is at most 200 characters",
            all(len(line) <= 200 for line in loud_err.splitlines()),
        )

        # 8. mixed tagged + untagged: two cells, equal height, full available width
        mixed_out = tmp / "mixed.svg"
        mixed = run(
            "plot-grid",
            "--input",
            str(tagged),
            "--input",
            str(svglite),
            "--output",
            str(mixed_out),
            "--width",
            "800",
            "--ncol",
            "2",
            "--plot-margin",
            "0",
        )
        mixed_text = mixed_out.read_text() if mixed_out.exists() else ""
        mixed_plan = plan(
            "--input",
            str(tagged),
            "--input",
            str(svglite),
            "--width",
            "800",
            "--ncol",
            "2",
            "--plot-margin",
            "0",
        )
        mixed_cells = cell_sizes(mixed_plan.stdout)
        check("mixed run rc=0", mixed.returncode == 0, mixed.stderr.decode())
        check("mixed output has both cells", 'data-svg-grid-cell="0"' in mixed_text and 'data-svg-grid-cell="1"' in mixed_text)
        check("mixed plan has two cells", len(mixed_cells) == 2)
        check(
            "mixed rows are equal height",
            len(mixed_cells) == 2 and abs(mixed_cells[0][1] - mixed_cells[1][1]) < 1e-3,
        )
        check(
            "mixed columns consume the full width",
            len(mixed_cells) == 2 and abs(sum(w for w, _ in mixed_cells) - 800.0) < 1e-3,
        )

    print(f"\nR32: {FAIL} failure(s), {PASS} pass")
    return 1 if FAIL else 0


if __name__ == "__main__":
    sys.exit(main())
