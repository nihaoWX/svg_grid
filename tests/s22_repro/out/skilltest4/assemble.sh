#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Supplementary figure assembly (panels A..F), canvas width 4961 px @ 600 dpi.
#   row1: A B        (2 columns)   A=fig1  B=fig2
#   row2: C D E      (3 columns)   C=fig3  D=fig4  E=fig5
#   row3: F          (1 column, full width)  F=fig6
# 4961 px / 600 dpi = 8.268 in = 595.3 pt  (= A4 width).
#
# Numbers I compute myself are the per-panel TARGET CELL WIDTHS (step 0 + step 2).
# They are needed because the composer rewrites coordinates but NOT font-size, so a
# panel whose canvas != the cell it lands in comes out with text proportionally
# wrong (and the tool warns).  Normalising each panel's long side to its exact cell
# width makes the layout scale factor s == 1 -> geometry, text and strokes all keep
# the designer's proportions.
# ---------------------------------------------------------------------------
set -euo pipefail

ROOT=/home/nihao/bioagentforge/tools/svg_grid
SVG="$ROOT/target/release/svg_grid"
CONV="$ROOT/convert/target/release/svg_grid_convert"
OUT=/home/nihao/bioagentforge/tools/svg_grid/tests/s22_repro/out/skilltest4
PY="micromamba run -n python-bio python"
cd "$OUT"
mkdir -p tagged norm rows

W=4961        # output canvas width in px  (@600dpi)  — given by the task
MARGIN=24     # svg_grid default plot margin (per side); letter band only grows TOP
GAP=0         # default row/column gap

# ---- step 0: compute each panel's target cell width (must be computed) --------
# interior width of a row = W - 2*margin - gap*(ncol-1); with gap=0 -> W-48 = 4913
# row with mixed aspect ratios: cell_i = avail * a_i / sum(a_i)
read -r CELL_AB CELL_CD CELL_E CELL_F <<<"$($PY -c "
W=$W; m=$MARGIN; g=$GAP
avail = W - 2*m
a_cd  = 504/360            # fig1..fig4 canvas aspect
a_e   = 460.8/345.6        # fig5 canvas aspect
a_f   = 648/288            # fig6 canvas aspect
cell_ab = (avail - 0*g)/2          # 2 equal-aspect panels
s23 = a_cd + a_cd + a_e
cell_cd = (avail - 2*g)*a_cd/s23   # fig3 / fig4 slots (majority aspect)
cell_e  = (avail - 2*g)*a_e/s23    # fig5 slot
cell_f  = avail                    # single full-width panel
print(cell_ab, cell_cd, cell_e, cell_f)
")"
echo "target cell widths: A/B=$CELL_AB  C/D=$CELL_CD  E=$CELL_E  F=$CELL_F" >&2

# ---- step 1: tag panels (svg -> protocol: panel box + data-scale groups) -------
for p in fig1_pairwise_matched_cos fig2_shared_ecotype_count fig3_empty_factors \
         fig4_transferability fig5_joint_nmf fig6_joint_k7_composition_pcolormesh; do
  $CONV --input panels/$p.svg --output tagged/$p.svg
done

# ---- step 2: normalise each panel so its long side == its target cell width ----
# (uniform scale of geometry + font-size + stroke-width => s == 1 in the row call)
norm () {  # $1 = panel stem, $2 = target cell width
  $SVG plot-grid --input tagged/$1.svg --output norm/$1.svg \
       --width "$2" --ncol 1 --plot-margin 0 --normalize-input-max-side "$2"
}
norm fig1_pairwise_matched_cos               "$CELL_AB"
norm fig2_shared_ecotype_count               "$CELL_AB"
norm fig3_empty_factors                      "$CELL_CD"
norm fig4_transferability                    "$CELL_CD"
norm fig5_joint_nmf                          "$CELL_E"
norm fig6_joint_k7_composition_pcolormesh    "$CELL_F"

# ---- step 3: one call per row; letters are added HERE, per row, explicitly -----
# guide: letters are serif / normal weight (tool default is sans-serif BOLD)
LBL=(--label-fontfamily serif --label-fontface normal)
$SVG plot-grid --input norm/fig1_pairwise_matched_cos.svg \
               --input norm/fig2_shared_ecotype_count.svg \
               --output rows/row1.svg --width $W --ncol 2 --labels A,B "${LBL[@]}"
$SVG plot-grid --input norm/fig3_empty_factors.svg \
               --input norm/fig4_transferability.svg \
               --input norm/fig5_joint_nmf.svg \
               --output rows/row2.svg --width $W --ncol 3 --labels C,D,E "${LBL[@]}"
$SVG plot-grid --input norm/fig6_joint_k7_composition_pcolormesh.svg \
               --output rows/row3.svg --width $W --ncol 1 --labels F "${LBL[@]}"

# ---- step 4: stack the row blocks; outer call carries NO labels, margin 0 ------
$SVG plot-grid --input rows/row1.svg --input rows/row2.svg --input rows/row3.svg \
     --output Figure.svg --output-png Figure.png --png-dpi 600 \
     --ncol 1 --width $W --plot-margin 0
