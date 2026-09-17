#!/bin/bash
# Build the S22 k-selection supplementary figure (3 rows: 2 / 3 / 1 panels).
#
#   $1 = variant:  unif  -> one font scale for the whole figure (10 px design text -> 5.5 pt)
#                  prop  -> per-row font scale = geometric scale s (= panel drawn at cell size)
#
# Everything is written under out/skilltest/ only.
set -euo pipefail
cd "$(dirname "$0")"               # -> out/skilltest
ROOT=../../../..                   # -> tools/svg_grid
SVG="$ROOT/target/release/svg_grid"
CVT="$ROOT/convert/target/release/svg_grid_convert"
PY="micromamba run -n python-bio python"

V="${1:-unif}"
W=4961
M=170          # composition margin per side (inner rows; the outer grid uses 0)
L=132          # panel-letter nominal size (canvas units) -> ink ~100 px = 2.0 % of W
DPI=600

mkdir -p "work/scaled_$V" "work/rows_$V" work/tagged

# ---------------------------------------------------------------- geometry ---
AVAIL=$(awk -v W=$W -v M=$M 'BEGIN{print W-2*M}')
CW1=$(awk  -v a=$AVAIL 'BEGIN{print a/2}')
H1=$(awk   -v m=$M -v c=$CW1 'BEGIN{print 2*m + c/1.4}')
H2=$(awk   -v m=$M -v a=$AVAIL 'BEGIN{asum=1.4+1.4+4/3; print 2*m + a/asum}')
CW2C=$(awk -v a=$AVAIL 'BEGIN{asum=1.4+1.4+4/3; print a*1.4/asum}')
CW2E=$(awk -v a=$AVAIL 'BEGIN{asum=1.4+1.4+4/3; print a*(4/3)/asum}')
H3=$(awk   -v m=$M -v a=$AVAIL 'BEGIN{print 2*m + a/2.25}')
TOTAL=$(awk -v a="$H1" -v b="$H2" -v c="$H3" 'BEGIN{printf "%.4f", a+b+c}')
echo "== geometry: avail=$AVAIL CW1=$CW1 H1=$H1 H2=$H2 CW2C=$CW2C CW2E=$CW2E H3=$H3 total=$TOTAL"

# ------ font scales ------------------------------------------------------
CW3=$AVAIL
if [ "$V" = unif ]; then
  K1=$(awk 'BEGIN{printf "%.4f", 5.5/0.12/10}')            # 10 px design text -> 5.5 pt on paper
  K2=$K1; K3=$K1; KE=$K1
else
  K1=$(awk -v c="$CW1"  'BEGIN{printf "%.4f", c/504}')
  K2=$(awk -v c="$CW2C" 'BEGIN{printf "%.4f", c/504}')
  K3=$(awk -v c="$CW3"  'BEGIN{printf "%.4f", c/648}')
  KE=$(awk -v c="$CW2E" 'BEGIN{printf "%.4f", c/460.8}')
fi
S1=$(awk -v c="$CW1"  'BEGIN{printf "%.4f", c/504}')
S2=$(awk -v c="$CW2C" 'BEGIN{printf "%.4f", c/504}')
S3=$(awk -v c="$AVAIL" 'BEGIN{printf "%.4f", c/648}')
SE=$(awk -v c="$CW2E" 'BEGIN{printf "%.4f", c/460.8}')
echo "== font scales k: A/B=$K1 (s=$S1)  C/D=$K2 (s=$S2)  E=$KE (s=$SE)  F=$K3 (s=$S3)"

# ------------------------------------------------- 1. tag every raw panel ---
for f in panels/*.svg; do
  b=$(basename "$f" .svg)
  $CVT --input "$f" --output "work/tagged/$b.svg" >/dev/null
done

# ------------------------- 2. bake font / dash scaling into the tagged panels
while IFS=: read -r name kf sg; do
  $PY work/prescale.py "work/tagged/$name.svg" "work/scaled_$V/$name.svg" "$kf" "$sg"
done <<EOF
fig1_pairwise_matched_cos:$K1:$S1
fig2_shared_ecotype_count:$K1:$S1
fig3_empty_factors:$K2:$S2
fig4_transferability:$K2:$S2
fig5_joint_nmf:$KE:$SE
fig6_joint_k7_composition_pcolormesh:$K3:$S3
EOF

# ------------------------------------------------------- 3. compose rows -----
COMMON=(--plot-margin "$M" --gap 0 --label-size "$L" \
        --label-fontfamily "DejaVu Serif" --label-fontface regular \
        --hjust 0 --vjust -0.25)

$SVG plot-grid --input "work/scaled_$V/fig1_pairwise_matched_cos.svg" \
               --input "work/scaled_$V/fig2_shared_ecotype_count.svg" \
               --output "work/rows_$V/row1.svg" \
               --width "$W" --height "$H1" --ncol 2 --rel-widths 1,1 \
               --labels A,B "${COMMON[@]}"

$SVG plot-grid --input "work/scaled_$V/fig3_empty_factors.svg" \
               --input "work/scaled_$V/fig4_transferability.svg" \
               --input "work/scaled_$V/fig5_joint_nmf.svg" \
               --output "work/rows_$V/row2.svg" \
               --width "$W" --height "$H2" --ncol 3 --rel-widths 1.4,1.4,1.333333 \
               --labels C,D,E "${COMMON[@]}"

$SVG plot-grid --input "work/scaled_$V/fig6_joint_k7_composition_pcolormesh.svg" \
               --output "work/rows_$V/row3.svg" \
               --width "$W" --height "$H3" --ncol 1 \
               --labels F "${COMMON[@]}"

# --------------------------------------------------- 4. stack the rows -------
# Each row block was composed with a full margin below its panels; that band is
# dead space inside the finished figure, so trim it down to a small gap (the
# guide allows a supplementary figure to be cropped to fit its content).
T=24
N1=$(awk -v h="$H1" -v m="$M" -v t="$T" 'BEGIN{printf "%.3f", h-m+t}')
N2=$(awk -v h="$H2" -v m="$M" -v t="$T" 'BEGIN{printf "%.3f", h-m+t}')
N3=$(awk -v h="$H3" -v m="$M" -v t="$T" 'BEGIN{printf "%.3f", h-m+t}')
NTOTAL=$(awk -v a="$N1" -v b="$N2" -v c="$N3" 'BEGIN{printf "%.3f", a+b+c}')
$PY work/trim_row.py "work/rows_$V/row1.svg" "$N1"
$PY work/trim_row.py "work/rows_$V/row2.svg" "$N2"
$PY work/trim_row.py "work/rows_$V/row3.svg" "$N3"

if [ "$V" = unif ]; then OUT="${2:-work/Figure_unif}"; else OUT="${2:-Figure}"; fi
$SVG plot-grid --input "work/rows_$V/row1.svg" --input "work/rows_$V/row2.svg" \
               --input "work/rows_$V/row3.svg" \
               --output "$OUT.svg" --output-png "$OUT.png" --output-svgz "$OUT.svgz" \
               --png-dpi "$DPI" \
               --width "$W" --height "$NTOTAL" --ncol 1 \
               --rel-heights "$N1,$N2,$N3" --plot-margin 0 --gap 0

echo "== wrote $OUT.svg / $OUT.png / $OUT.svgz"
