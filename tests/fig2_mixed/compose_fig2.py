#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Assemble the project's main Figure 2 with `svg_grid plot-grid` from a MIXED
vector + bitmap panel set, and verify it.

Pipeline
--------
1. ``svg_grid_convert`` on every panel SVG built by ``make_panels.py``
   (vector matplotlib panel + 13 bitmap wrappers) -> ``out/tagged/``;
2. compose the three bands, each in its own call (that is where the panel letters
   are added), -> ``out/blocks/``:
       A  1 x 1   (full width)     labels "A"
       B  1 x 4   (full width)     labels "B,,,"
       C  3 x 3   (full width)     labels "C,,,,,,,,"
3. stack the three bands with ``--ncol 1 --plot-margin 0`` (no letters) ->
   ``out/Figure2.svg`` + ``out/Figure2.png`` (--png-dpi 600);
4. verification: per-cell ``sx``/``sy`` from the composed SVG, panel-cell aspect,
   dialect residue, letter ink width, ink coverage, and a stacked equal-width
   comparison with the project's existing ``Figure 2.png``.  The white-band
   (dead-band) rule is **imported from the single script**
   ``tools/svg_grid/scripts/check_deliverable.py`` (``check_dead_band``) instead
   of keeping a local copy.

Every ``plot-grid`` / ``svg_grid_convert`` stderr stream is captured verbatim.

Usage:
    micromamba run -n python-bio python compose_fig2.py

实现按现有职责分布在同目录的辅助模块里：`fig2_common`（路径 / 版面几何 / 块规格）、
`fig2_compose`（convert + 逐块组图 + stack）、`fig2_verify`（letter/纸上规则 +
内嵌栅格 + 对比）。本文件只保留入口 `main()`。
"""

import os
import re
import sys

import numpy as np
from PIL import Image, ImageFont

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

from fig2_common import BLOCKS, LETTER_GAP, ORIGINAL, OUT, SERIF, W   # noqa: E402
from fig2_compose import compose, convert_panels, stack               # noqa: E402
from fig2_verify import assert_raster_rendered, compare, paper_rules  # noqa: E402


# --------------------------------------------------------------------------- #
def main():
    font = ImageFont.truetype(SERIF, 100)
    # reproduce make_panels.py's size_for_width("A", 0.020*W)
    lo, hi = 8, 4000
    while lo < hi:
        mid = (lo + hi) // 2
        bbox = ImageFont.truetype(SERIF, mid).getbbox("A")
        if bbox[2] - bbox[0] < 0.020 * W:
            lo = mid + 1
        else:
            hi = mid
    letter_size = lo
    letter_vjust = -LETTER_GAP / letter_size
    del font

    print("=" * 78)
    print("1. convert every panel into the tag protocol")
    print("=" * 78)
    converted = convert_panels()
    print(f"{'panel':16s} {'canvas':>12s} {'path':>4s} {'use':>4s} {'sym':>4s} "
          f"{'g_tf':>4s} {'img_tf':>6s} {'text':>5s} {'image':>5s} {'polyline':>8s} "
          f"{'polygon':>7s} {'rect':>4s}")
    for row in converted:
        canvas = f"{row['canvas'][0]:.0f}x{row['canvas'][1]:.0f}"
        print(f"{row['panel']:16s} {canvas:>12s} {row['path']:4d} {row['use']:4d} "
              f"{row['symbol']:4d} {row['gtransform']:4d} {row['imgtf']:6d} "
              f"{row['text']:5d} {row['image']:5d} {row['polyline']:8d} "
              f"{row['polygon']:7d} {row['rect']:4d}")
        assert row["exit"] == 0
        assert (row["path"], row["use"], row["symbol"], row["gtransform"],
                row["imgtf"]) == (0, 0, 0, 0, 0), f"{row['panel']} has dialect residue"

    print("\n" + "=" * 78)
    print("2. compose the three bands (letters are added here)")
    print("=" * 78)
    report = compose(letter_size, letter_vjust)
    for entry in report:
        block = entry["block"]
        print(f"\nband {block['name']}: {len(block['inputs'])} panel(s), ncol={block['ncol']}, "
              f"margin={block['margin']}, height={block['height']}")
        print(f"  {'panel':16s} {'cell x,y,w,h':>34s} {'panel box x,y':>16s} "
              f"{'cell asp':>9s} {'canvas asp':>10s} {'sx':>8s} {'sy':>8s} "
              f"{'sx_coord':>9s} {'sy_coord':>9s}")
        for m in entry["measured"]:
            box = m["panel_box"]
            sx_c = "n/a" if m["sx_coord"] is None else f"{m['sx_coord']:9.5f}"
            sy_c = "n/a" if m["sy_coord"] is None else f"{m['sy_coord']:9.5f}"
            print(f"  {m['panel']:16s} "
                  f"{m['cell'][0]:8.2f},{m['cell'][1]:7.2f},{m['cell'][2]:8.2f},{m['cell'][3]:8.2f} "
                  f"{box[0]:8.2f},{box[1]:7.2f}   "
                  f"{m['aspect_cell']:9.5f} {m['aspect_canvas']:10.5f} "
                  f"{m['sx']:8.5f} {m['sy']:8.5f} {sx_c} {sy_c}")
            assert abs(m["panel_box"][0] - m["cell"][0]) < 0.05
            assert abs(m["panel_box"][1] - m["cell"][1]) < 0.05
            assert abs(m["aspect_cell"] - m["aspect_canvas"]) < 1e-4, "cell distorted"
            assert abs(m["sx"] - 1) < 1e-4 and abs(m["sy"] - 1) < 1e-4, "scale != 1"
            for key in ("sx_coord", "sy_coord"):
                if m[key] is not None:
                    assert abs(m[key] - 1) < 1e-4, f"{key} != 1"

    print("\n" + "=" * 78)
    print("3. stack the bands -> Figure2.svg + Figure2.png")
    print("=" * 78)
    figure_svg, stack_stderr = stack()
    svg = open(figure_svg, encoding="utf-8").read()
    print(f"  Figure2.svg  {os.path.getsize(figure_svg) / 1e6:.2f} MB  "
          f"text={svg.count('<text')}  image={svg.count('<image')}  "
          f"cells={svg.count('data-svg-grid-cell')}  labels={svg.count('data-svg-grid-label')}")
    print(f"  residue: path={svg.count('<path')} use={svg.count('<use')} "
          f"symbol={svg.count('<symbol')} "
          f"g_tf={len(re.findall(r'<g\\b[^>]*\\btransform=', svg))} "
          f"image_tf={len(re.findall(r'<image\\b[^>]*\\btransform=', svg))}")
    assert svg.count("<path") == 0 and svg.count("<use") == 0 and svg.count("<symbol") == 0
    assert len(re.findall(r"<g\b[^>]*\btransform=", svg)) == 0
    assert len(re.findall(r"<image\b[^>]*\btransform=", svg)) == 0
    assert "preserveAspectRatio" in svg

    print("\n" + "=" * 78)
    print("4. paper rules, on the rendered PNG")
    print("=" * 78)
    png = os.path.join(OUT, "Figure2.png")
    offsets, running = [], 0
    for block in BLOCKS:
        offsets.append(running)
        running += block["height"]
    anchors = [(char, offsets[i] + block["margin"][0])
               for i, (char, block) in enumerate(zip("ABC", BLOCKS))]
    print(f"  band offsets in the stacked canvas: "
          f"{ {b['name']: o for b, o in zip(BLOCKS, offsets)} }  sum={running}")
    rules = paper_rules(png, letter_size, anchors)
    print(f"  Figure2.png  {rules['size'][0]}x{rules['size'][1]} px @600 dpi  "
          f"ink(non-white)={rules['ink_fraction'] * 100:.2f} %")
    print(f"  A4 check: {rules['size'][0] / rules['size'][1]:.5f} "
          f"(A4 = 0.70711), height/width = {rules['size'][1] / rules['size'][0]:.5f}")
    print(f"  max all-white row band = {rules['max_white_row']} px, "
          f"col band = {rules['max_white_col']} px (both must be < 200)")
    print(f"  dead (all-white) 200 px row bands: {rules['dead_bands']}")
    print(f"  fully-white rows={rules['full_white_rows']} cols={rules['full_white_cols']}")
    for letter in rules["letters"]:
        print(f"  letter {letter['char']}: ink {letter['ink_w']}x{letter['ink_h']} px at "
              f"({letter['left']},{letter['top']}) = {letter['w_frac'] * 100:.2f} % of canvas "
              f"width (band 1.8-2.2)  fill={letter['fill']:.3f} "
              f"(serif regular {letter['reg_fill']:.3f} vs bold {letter['bold_fill']:.3f})")
        assert abs(letter["fill"] - letter["reg_fill"]) < abs(letter["fill"] - letter["bold_fill"])
        assert letter["top"] > 0, "letter clipped at the canvas top"
    widest = max(rules["letters"], key=lambda L: L["w_frac"])
    narrowest = min(rules["letters"], key=lambda L: L["w_frac"])
    assert 0.018 <= widest["w_frac"] <= 0.022, f"widest letter {widest['char']} out of 1.8-2.2 %"
    assert narrowest["w_frac"] >= 0.015, f"narrowest letter {narrowest['char']} below 1.5 %"
    # the dead-band criterion is asserted inside paper_rules() via
    # check_deliverable.check_dead_band (single source of truth)

    print("\n" + "=" * 78)
    print("5. embedded rasters really rendered")
    print("=" * 78)
    rasters = assert_raster_rendered(png, figure_svg)
    for box, unique in rasters:
        print(f"  <image> box {tuple(round(v, 1) for v in box)} unique colours={unique}")
        assert unique > 100

    print("\n" + "=" * 78)
    print("6. comparison with the existing Figure 2")
    print("=" * 78)
    path, size = compare(png)
    print(f"  ours {size[0]}x{size[1]}; stacked equal-width -> {os.path.relpath(path, _HERE)}")

    bands = [("A", 0, 1053), ("B", 1053, 1053 + 924), ("C", 1977, 7016)]
    original = np.asarray(Image.open(ORIGINAL).convert("L"), dtype=np.int16)
    ours = np.asarray(Image.open(png).convert("L"), dtype=np.int16)
    print(f"  {'band':6s} {'RMS':>8s} {'>30 diff %':>11s} {'ours-only ink':>14s} "
          f"{'original-only ink':>18s}")
    for name, y0, y1 in bands:
        a, b = original[y0:y1], ours[y0:y1]
        diff = np.abs(a - b)
        ours_only = ((b < 200) & (a > 245)).mean() * 100
        orig_only = ((a < 200) & (b > 245)).mean() * 100
        print(f"  {name:6s} {np.sqrt((diff.astype(float) ** 2).mean()):8.2f} "
              f"{(diff > 30).mean() * 100:11.3f} {ours_only:14.3f} {orig_only:18.3f}")
    print(f"\nstack stderr bytes = {len(stack_stderr)}")


if __name__ == "__main__":
    main()
