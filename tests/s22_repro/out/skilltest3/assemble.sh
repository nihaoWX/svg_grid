#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Assemble supplementary figure "Figure S"  (6 panels A-F, 2/3/1 layout)
# Canvas width 4961 px (= A4 width @600 dpi), 600 dpi PNG.
#
# Reading order A(2/1) B(2/1) C(3/1) D(3/2) E(4/*) ... see notes below.
#   row 1 : A B        (fig1 fig2)          ncol 2
#   row 2 : C D E      (fig3 fig4 fig5)     ncol 3
#   row 3 : F          (fig6)               ncol 1  (full width)
# ---------------------------------------------------------------------------
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SVG_GRID="/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid"
SVG_CONVERT="/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert"

mkdir -p "$HERE/tagged" "$HERE/rows"

# --- STEP 1: convert each raw matplotlib panel into the grid protocol -------
#   (adds <rect data-panel-box="main"> + data-scale groups; bakes <path>/<use>/
#    <g transform>; lifts stroke-width out of CSS into attributes)
for n in 1 2 3 4 5 6; do
  src="$(ls "$HERE"/panels/fig${n}_*.svg)"
  "$SVG_CONVERT" --input "$src" --output "$HERE/tagged/fig${n}.svg"
done

# --- STEP 2: one row per call.  Only content + column count are given. -------
# With --width only (no --height, no --rel-*):
#   * column widths are allocated so that cell aspect == panel canvas aspect
#     -> every panel keeps its own aspect (zero distortion) automatically;
#   * canvas height = natural row height + margins.
#
# LAYOUT ARITHMETIC (the numbers that had to be computed by hand):
#   margin = 24 (default), gap = 0 (default)
#   avail  = 4961 - 2*24 - 0 = 4913
#   cell_h = avail / sum(panel aspect)          (uniform across the row)
#   cell_w_i = avail * aspect_i / sum(aspect)
# `--normalize-input-max-side` is then set to the row's cell width so the
# panel's long side lands on the cell (s ~= 1: geometry, font-size and
# stroke-width are pre-scaled together, restoring the design text proportion).
# Row 1: aspects 1.400,1.400 -> cell_w = 4913/2                    = 2456.50
# Row 2: aspects 1.400,1.400,1.3333 (sum 4.1333) -> cell_w = 1664.11 (majority)
# Row 3: aspect 2.25 -> cell_w = 4913/1                           = 4913.00

# Row 1 : A B
"$SVG_GRID" plot-grid \
  --input "$HERE/tagged/fig1.svg" --input "$HERE/tagged/fig2.svg" \
  --output "$HERE/rows/row1.svg" \
  --width 4961 --ncol 2 --normalize-input-max-side 2456.5 \
  --labels A,B --label-fontfamily serif --label-fontface normal

# Row 2 : C D E
"$SVG_GRID" plot-grid \
  --input "$HERE/tagged/fig3.svg" --input "$HERE/tagged/fig4.svg" --input "$HERE/tagged/fig5.svg" \
  --output "$HERE/rows/row2.svg" \
  --width 4961 --ncol 3 --normalize-input-max-side 1664.11 \
  --labels C,D,E --label-fontfamily serif --label-fontface normal

# Row 3 : F
"$SVG_GRID" plot-grid \
  --input "$HERE/tagged/fig6.svg" \
  --output "$HERE/rows/row3.svg" \
  --width 4961 --ncol 1 --normalize-input-max-side 4913 \
  --labels F --label-fontfamily serif --label-fontface normal

# --- STEP 3: stack the three row blocks (one column) ------------------------
# No --labels here (that would re-number the row blocks). --width 4961 keeps the
# canvas width; height is auto (sum of the rows' natural heights + margins).
"$SVG_GRID" plot-grid \
  --input "$HERE/rows/row1.svg" --input "$HERE/rows/row2.svg" --input "$HERE/rows/row3.svg" \
  --output "$HERE/Figure.svg" --output-png "$HERE/Figure.png" --png-dpi 600 \
  --width 4961 --ncol 1

echo "done: $HERE/Figure.svg + $HERE/Figure.png"
