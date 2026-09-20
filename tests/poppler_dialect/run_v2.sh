#!/usr/bin/env bash
# V2 runner: poppler-dialect fixtures, end to end.
#   bash run_v2.sh
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PY=/home/nihao/micromamba/envs/python-bio/bin/python
CONVERT=/home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert
GRID=/home/nihao/bioagentforge/tools/svg_grid/target/release/svg_grid
cd "$HERE"

echo "== 0. regenerate the genuine pdftocairo product (needs pdftocairo) =="
if command -v pdftocairo >/dev/null; then
    "$PY" make_poppler.py real_poppler.svg
else
    echo "pdftocairo not found; using the committed real_poppler.svg"
fi

echo
echo "== 1. convert every fixture (must exit 0, no dialect residue) =="
for f in fixture_glyph_hole_nz fixture_clip_rect_path fixture_image_matrix real_poppler; do
    code=0
    "$CONVERT" --input "$f.svg" --output "$f.tagged.svg" >"$f.convert.log" 2>&1 || code=$?
    residue=$( { grep -oE '<path|<use|<symbol|transform=' "$f.tagged.svg" || true; } | wc -l)
    wellformed=$("$PY" -c "import xml.etree.ElementTree as ET; ET.parse('$f.tagged.svg'); print('ok')")
    echo "  $f: exit=$code residue=$residue xml=$wellformed"
    test "$code" = 0
    test "$residue" = 0
done

echo
echo "== 2. #1 glyph-with-hole: objective hole-survival (render + sample the hole centre) =="
"$GRID" plot-grid --input fixture_glyph_hole_nz.tagged.svg --output comp_hole_nz.svg \
    --output-png comp_hole_nz.png --width 100 --height 100 --ncol 1 --plot-margin 0 2>stderr_hole_nz.txt
echo "  nz compose: exit=$? stderr=$(wc -c < stderr_hole_nz.txt) bytes"
"$PY" check_glyph_hole.py fixture_glyph_hole_nz.tagged.svg comp_hole_nz.png 51 102 204

echo
echo "  -- and the genuine pdftocairo 'O' glyph --"
"$GRID" plot-grid --input real_poppler.tagged.svg --output comp_hole_poppler.svg \
    --output-png comp_hole_poppler.png --width 288 --height 216 --ncol 1 --plot-margin 0 2>stderr_hole_pop.txt
echo "  poppler compose: exit=$? stderr=$(wc -c < stderr_hole_pop.txt) bytes"
"$PY" check_glyph_hole.py real_poppler.tagged.svg comp_hole_poppler.png 0 0 0

echo
echo "== 3. #2 clip rect path and #3 image matrix: render + sample =="
"$GRID" plot-grid --input fixture_clip_rect_path.tagged.svg --output comp_clip.svg \
    --output-png comp_clip.png --width 100 --height 80 --ncol 1 --plot-margin 0 2>stderr_clip.txt
echo "  clip compose: exit=$? stderr=$(wc -c < stderr_clip.txt) bytes"
"$GRID" plot-grid --input fixture_image_matrix.tagged.svg --output comp_img.svg \
    --output-png comp_img.png --width 1000 --height 400 --ncol 1 --plot-margin 0 2>stderr_img.txt
echo "  image compose: exit=$? stderr=$(wc -c < stderr_img.txt) bytes"
"$PY" check_clip_and_image.py comp_clip.png comp_img.png
