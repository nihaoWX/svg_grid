#!/usr/bin/env bash
# Shape-paint regression runner:
#   * a filled *and* stroked <path> must not stroke the bridges of the
#     concatenated fill region (defect 1);
#   * open subpaths must be explicitly closed before concatenation, so the
#     bridges do not change the winding number (defect 2);
#   * a glyph's counter must survive (fill-only nested rings).
#
#   bash run.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY=/home/nihao/micromamba/envs/python-bio/bin/python
CONVERT=/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert
GRID=/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid
cd "$HERE"

echo "== convert + compose every fixture (canvas 100x100 -> cell 100x100, scale 1) =="
for f in fixture_bridge_stroke fixture_open_subpaths fixture_glyph_hole; do
    "$CONVERT" --input "$f.svg" --output "$f.tagged.svg" >/dev/null
    "$GRID" plot-grid --input "$f.tagged.svg" --output "$f.comp.svg" \
        --output-png "$f.comp.png" --width 100 --height 100 --ncol 1 --plot-margin 0 >/dev/null 2>&1
    echo "  $f: converted + composed"
done

echo
echo "== pixel probes (defects 1 and 2) =="
"$PY" check_shape_paint.py

echo
echo "== glyph hole survives (same verdict as the poppler dialect) =="
"$PY" ../poppler_dialect/check_glyph_hole.py \
    fixture_glyph_hole.tagged.svg fixture_glyph_hole.comp.png 0 0 0
