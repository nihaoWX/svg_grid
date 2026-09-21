#!/usr/bin/env bash
# RDKit-dialect runner: generate a genuine RDKit `MolDraw2DSVG` panel, convert it,
# and prove the composed PNG has real ink.
#
# The dialect is pure-vector and carries unit-bearing lengths — root
# `width='300px' height='200px'` and `stroke-width:2.0px` on every bond path. That
# used to fail the post-conversion self-check outright ("22 unsupported node(s)",
# exit 1), so the whole panel could not enter the pipeline. This runner locks in
# that it now converts (exit 0, no `px` residue) and composes to a non-blank image.
#
#   bash run.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY=/home/nihao/micromamba/envs/python-bio/bin/python
CONVERT=/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert
GRID=/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid
cd "$HERE"

echo "== 1. render the RDKit MolDraw2DSVG panel (p-toluic acid, 300x200) =="
"$PY" make_rdkit.py mol.svg

echo
echo "== 2. convert (must exit 0; the unit-bearing lengths must become plain numbers) =="
"$CONVERT" --input mol.svg --output mol.tagged.svg
px=$(grep -c 'px' mol.tagged.svg || true)
echo "  residue: $px \`px\` occurrence(s) (want 0)"
test "$px" = 0
echo "  stroke-width values:"
grep -oE 'stroke-width="[^"]*"' mol.tagged.svg | sort | uniq -c

echo
echo "== 3. compose one cell + rasterise =="
"$GRID" plot-grid --input mol.tagged.svg --output mol.grid.svg --output-png mol.grid.png \
    --width 300 --height 200 --ncol 1 --plot-margin 0

echo
echo "== 4. objective ink measurement (a blank render would mean a silent failure) =="
"$PY" check_ink.py mol.grid.png
