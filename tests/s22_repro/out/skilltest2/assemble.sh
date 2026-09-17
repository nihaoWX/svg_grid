#!/usr/bin/env bash
# Assemble the 6 scattered panel SVGs of out/skilltest2/panels/ into one supplementary figure:
#   row 1: A B        (fig1, fig2)   aspects 1.4, 1.4
#   row 2: C D E      (fig3, fig4, fig5)  aspects 1.4, 1.4, 1.333333
#   row 3: F          (fig6)         aspect 2.25  (full width)
# Canvas: 4961 x 5575 px, PNG pHYs = 600 dpi.
set -euo pipefail
cd "$(dirname "$0")"

SG=/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid
CV=/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert

# ---- geometry (px) ---------------------------------------------------------
# W=4961;  per-row margins: top=150 (panel-letter band), right=50, bottom=B, left=50; gap=80
# cell_w_i = (W - L - R - gap*(ncol-1)) * relw_i / sum(relw)     with relw_i = panel aspect
# cell_h   = available_w / sum(relw)      -> cell aspect == panel aspect exactly
# H_row    = 150 + cell_h + B
# row1: avail 4781            sum 2.8        cell 2390.500 x 1707.500    H=1897.500
# row2: avail 4701            sum 4.133333   cell 1592.274 x 1137.339    H=1327.339
# row3: avail 4861            sum 2.25       cell 4861.000 x 2160.444    H=2350.161 (bottom cut to 39.717
#                                                                            so the 3 row heights sum to 5575)

# ---- step 1: tag the raw matplotlib panels ---------------------------------
mkdir -p tagged rows
for f in panels/*.svg; do
  "$CV" --input "$f" --output "tagged/$(basename "$f")"
done

# ---- step 2: one plot-grid call per row (labels are added at this level) ----
"$SG" plot-grid \
  --input tagged/fig1_pairwise_matched_cos.svg \
  --input tagged/fig2_shared_ecotype_count.svg \
  --output rows/row1.svg --output-png rows/row1.png \
  --width 4961 --height 1897.5 --ncol 2 \
  --plot-margin 150,50,40,50 --gap 80 \
  --normalize-input-max-side 2390.5 \
  --labels A,B --label-fontfamily serif --label-fontface normal

"$SG" plot-grid \
  --input tagged/fig3_empty_factors.svg \
  --input tagged/fig4_transferability.svg \
  --input tagged/fig5_joint_nmf.svg \
  --output rows/row2.svg --output-png rows/row2.png \
  --width 4961 --height 1327.338709677 --ncol 3 \
  --plot-margin 150,50,40,50 --gap 80 \
  --normalize-input-max-side 1592.274193548 \
  --labels C,D,E --label-fontfamily serif --label-fontface normal

"$SG" plot-grid \
  --input tagged/fig6_joint_k7_composition_pcolormesh.svg \
  --output rows/row3.svg --output-png rows/row3.png \
  --width 4961 --height 2350.161 --ncol 1 \
  --plot-margin 150,50,39.717,50 --gap 0 \
  --normalize-input-max-side 4861 \
  --labels F --label-fontfamily serif --label-fontface normal

# ---- step 3: vertical stack of the three row blocks (s = 1, no rescaling) ---
"$SG" plot-grid \
  --input rows/row1.svg \
  --input rows/row2.svg \
  --input rows/row3.svg \
  --output Figure.svg --output-png Figure.png \
  --width 4961 --height 5575 --ncol 1 --nrow 3 \
  --plot-margin 0 --gap 0 --png-dpi 600
