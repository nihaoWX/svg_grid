#!/usr/bin/env bash
# V1 runner: regenerate the matplotlib dialect panel, convert it, prove geometric
# equivalence and confirm the render-layer ink placement.
#
#   bash run_v1.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY=/home/nihao/micromamba/envs/python-bio/bin/python
CONVERT=/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert
GRID=/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid
cd "$HERE"

echo "== 1. render the matplotlib dialect panel (log axes + LogNorm cb + mathtext + multi-line) =="
"$PY" make_panel.py panel.svg

echo
echo "== 2. convert (must exit 0; the only stderr is the benign <style> note) =="
"$CONVERT" --input panel.svg --output panel.tagged.svg --report

echo
echo "== 3. geometry equivalence (original vs converted) =="
"$PY" check_geometry.py panel.svg panel.tagged.svg

echo
echo "== 4. compose one cell + rasterise; check exit/stderr =="
"$GRID" plot-grid --input panel.tagged.svg --output composed.svg --output-png composed.png \
    --png-dpi 600 --width 460.8 --height 345.6 --ncol 1 --plot-margin 0 >stdout.txt 2>stderr.txt
echo "compose exit=0  stderr=$(wc -c < stderr.txt) bytes"
if [ -s stderr.txt ]; then echo "FAIL: non-empty stderr"; cat stderr.txt; exit 1; fi

echo
echo "== 5. render-layer ink placement =="
"$PY" check_render.py composed.svg composed.png
