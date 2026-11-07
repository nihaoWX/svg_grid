#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Reproduce the Supplementary Figure S22 layout with `svg_grid plot-grid`.

Pipeline (all outputs land in ``out/``; the research project is only ever READ):

  1. convert every *sized* matplotlib panel in ``out/panels_sized/`` to the tag
     protocol with the `svg_grid_convert` binary (``out/tagged/``);
  2. verify no dialect residue survives (no ``<path>``/``<use>``/``<g transform>``/
     un-normalised text transform / transformed ``<image>``);
  3. compose the three blocks and stack them:
         block 1: A | B          (2 columns)
         block 2: C | D | E      (3 columns)
         block 3: F              (1 column, full content width)
     `plot-grid` only does a *regular* grid, so S22 is built by nesting: AB and
     CDE are composed first, then the three blocks are stacked with ``--ncol 1``.
     Neither level passes ``--rel-*``: the columns take the panels' own canvas
     aspect ratios (``--rel-widths`` omitted) and the stacked rows take the
     blocks' natural heights (``--rel-heights`` omitted), which is exactly what
     the hand-computed ``aspects`` / block heights used to be fed as;
  4. assert, for every block and every cell, that the composed ``cell/canvas``
     scale is ~1 — i.e. that each panel was drawn at the size of the cell it
     occupies.  The tag protocol never rescales ``font-size``, so a panel drawn
     at design size and upscaled into a cell keeps its text too small; drawing it
     already at cell size (``make_panels_svg.py --scale-to-cell``) makes
     ``sx = cell.w/canvas.w`` and ``sy = cell.h/canvas.h`` both ~1, so the text
     comes out at its design proportion.  This is checked both analytically and
     by parsing the composed SVG: each panel's full-canvas background polygon
     must land on exactly its expected cell rectangle;
  5. render ``FigureS22.png`` at 600 dpi, verify the paper rules from
     ``tools/manuscripts_skills/general-figure-guide`` (panel letters serif
     regular and 1.8-2.2 % of the canvas width; the dead-band rule is
     **imported from the single script** ``tools/svg_grid/scripts/check_deliverable.py``
     — ``check_dead_band`` etc. — instead of keeping a local copy) and build a
     stacked, equal-width comparison with the project's existing ``Figure S22.png``.

Usage:
    micromamba run -n python-bio python compose_s22.py

实现按现有职责分布在同目录的辅助模块里：`s22_common`（面板规格 / 版式几何 /
子进程封装）、`s22_convert`（第 1 节）、`s22_compose`（第 3+4 节）、`s22_guards`
（第 4b 节回归守卫）、`s22_verify`（第 5 节核验 + 对比）。本文件只保留入口 `main()`。
"""

import os
import re
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

from s22_common import CANVAS_WIDTH, LETTER_FRAC, OUT, geometry      # noqa: E402
from s22_convert import convert_panels                               # noqa: E402
from s22_compose import compose                                      # noqa: E402
from s22_guards import (                                             # noqa: E402
    assert_factor_order,
    assert_legend_ratios,
    assert_raster_rendered,
)
from s22_verify import check_paper_rules, compare                    # noqa: E402


# --------------------------------------------------------------------------- #
def main():
    print("== 1. convert (sized panels) ==")
    conversion = convert_panels()
    print(f"{'panel':44s} {'exit':>4s} {'path':>4s} {'use':>4s} {'g_tf':>4s} "
          f"{'img_tf':>6s} {'text':>4s} {'image':>5s} {'polyline':>8s} {'polygon':>7s}")
    for row in conversion:
        print(f"{row['panel']:44s} {row['exit']:4d} {row['path']:4d} {row['use']:4d} "
              f"{row['gtransform']:4d} {row['imgtf']:6d} {row['text']:4d} {row['image']:5d} "
              f"{row['polyline']:8d} {row['polygon']:7d}")
        assert row["exit"] == 0, f"{row['panel']} failed to convert"
        assert (row["path"], row["use"], row["gtransform"], row["imgtf"]) == (0, 0, 0, 0), \
            f"{row['panel']} still has dialect residue"

    geo = geometry(CANVAS_WIDTH)
    print(f"\n== 2. geometry (canvas width {CANVAS_WIDTH}) ==")
    print(f"  m_side={geo['m_side']} g_col={geo['g_col']} band={geo['band']} "
          f"(gap={geo['gap_letter']} + glyph_h={geo['glyph_h']} + toprun={geo['toprun']})")
    print(f"  label_size={geo['label_size']} canvas units  vjust={geo['vjust']:.5f}  "
          f"-> A ink ~{LETTER_FRAC * CANVAS_WIDTH:.1f} px "
          f"({LETTER_FRAC * 100:.2f} % of width)")

    prefix = "FigureS22"
    print(f"\n== 3. compose {prefix} (canvas width {CANVAS_WIDTH}) ==")
    figure_svg, total_h, scales, anchors, cell_boxes = compose(CANVAS_WIDTH, prefix, geo)
    svg = open(figure_svg, encoding="utf-8").read()
    g_tf = len(re.findall(r"<g\b[^>]*\btransform=", svg))
    image_tf = len(re.findall(r"<image\b[^>]*\btransform=", svg))
    unnormalized_text = len(re.findall(
        r"<text\b[^>]*\btransform=(?!\"rotate\(-?[0-9.]+[, ]+-?[0-9.]+[, ]+-?[0-9.]+\)\")", svg))
    print(f"  {prefix}.svg  size={os.path.getsize(figure_svg)} bytes  "
          f"height={total_h:.1f}  cells={svg.count('data-svg-grid-cell')}  "
          f"text={svg.count('<text')}")
    print(f"  residue: path={svg.count('<path')} use={svg.count('<use')} "
          f"symbol={svg.count('<symbol')} g_tf={g_tf} image_tf={image_tf} "
          f"unnormalized_text_tf={unnormalized_text}")
    assert svg.count("<path") == 0 and svg.count("<use") == 0 and svg.count("<symbol") == 0
    assert (g_tf, image_tf, unnormalized_text) == (0, 0, 0)

    print("  per-cell scale (should be 1):")
    for panel, sx, sy in scales:
        print(f"    {panel:44s} sx={sx:.6f} sy={sy:.6f}")

    png = os.path.join(OUT, prefix + ".png")
    print("\n== 4. paper-rule checks (on the rendered image) ==")
    rules = check_paper_rules(png, anchors, geo["label_size"],
                              margin=geo["m_side"], label_band=geo["band"])
    print(f"  {prefix}.png  {rules['size'][0]}x{rules['size'][1]} px @600dpi  "
          f"ink(non-white)={rules['ink_fraction'] * 100:.2f}%")
    print(f"  max all-white row band = {rules['max_white_row']} px  "
          f"(fully-white rows: {rules['full_white_rows']})")
    print(f"  max all-white col band = {rules['max_white_col']} px  "
          f"(fully-white cols: {rules['full_white_cols']})")
    print(f"  all-white row runs (y, len) = {rules['white_row_runs']}")
    for L in rules["letters"]:
        print(f"    {L['char']}: ink {L['ink_w']}x{L['ink_h']} px = "
              f"{L['w_frac'] * 100:.2f}% of canvas width  fill={L['fill']:.3f} "
              f"(DejaVu Serif regular {L['reg_fill']:.3f} vs bold {L['bold_fill']:.3f})")

    print("\n== 4b. regression guards ==")
    raster = assert_raster_rendered(png, figure_svg)
    for box, unique, mean in raster:
        print(f"  <image> box {box}  unique colours={unique}  "
              f"mean=[{mean[0]:.1f} {mean[1]:.1f} {mean[2]:.1f}]  (need > 100)")

    legend = assert_legend_ratios(png, cell_boxes)
    for panel, ours, reference, rel_err in legend:
        print(f"  {panel:32s} legend/plot = {ours:.4f}  ref = {reference:.4f}  "
              f"rel_err = {rel_err * 100:.2f} %  (need < 15 %)")

    f_cell = cell_boxes["fig6_joint_k7_composition_pcolormesh"]
    rows = assert_factor_order(figure_svg, f_cell)
    print(f"  F-cell y ticks top-to-bottom = "
          f"{','.join(text for _, text in rows)}  (need F0..F6)")

    print("\n== 5. comparison ==")
    path, original_size, ours_size = compare(png, prefix)
    print(f"  original   {original_size[0]}x{original_size[1]}")
    print(f"  ours       {ours_size[0]}x{ours_size[1]}")
    print(f"  stacked (equal width {original_size[0]}) -> {path}")


if __name__ == "__main__":
    main()
