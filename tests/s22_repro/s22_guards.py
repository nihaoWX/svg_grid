#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_s22` 第 4b 节：回归守卫（三种曾经上线过的失败模式）。"""

import os
import re

import numpy as np
from PIL import Image

IMAGE_RE = re.compile(r"<image\b[^>]*>")
TEXT_RE = re.compile(r"<text\b([^>]*)>(.*?)</text>", re.S)
FACTOR_ORDER = [f"F{i}" for i in range(7)]

# reference panels that carry exactly one legend frame
LEGEND_REFERENCE = {
    "fig1_pairwise_matched_cos": "fig1_pairwise_matched_cos",
    "fig2_shared_ecotype_count": "fig2_shared_ecotype_count",
    "fig3_empty_factors": "fig3_empty_factors",
    "fig4_transferability": "fig4_transferability",
}
REFERENCE_PANELS = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/"
                    "05_空间深化/spatial_transcription/nmf_k_selection/figure_panels")


def _vertical_run(mask):
    """per-column longest run of consecutive True (mask: 2D bool, rows first)."""
    idx = np.arange(mask.shape[0])[:, None]
    last_false = np.where(~mask, idx, -1)
    np.maximum.accumulate(last_false, axis=0, out=last_false)
    return np.where(mask, idx - last_false, 0).max(axis=0)


def legend_ratio(image):
    """legend-frame width / axes width, measured from a rendered RGB panel.

    Identical rule for our render and the project's reference panel, so their
    ratios are comparable:

      * axes spines : near-black (gray < 128) vertical runs covering most of the
                      crop -> the only dark runs that long (the legend frame is
                      light gray, the grid is lighter still);
      * legend frame: matplotlib's default legend edge is '0.8', a light-gray
                      rounded rectangle -> the largest connected component of
                      light-gray pixels; its bounding box is the legend box.
    """
    from scipy import ndimage as ndi

    a = np.asarray(image.convert("RGB"), dtype=float).mean(axis=2)
    col_run = _vertical_run(a < 128)
    spines = np.where(col_run >= 0.55 * col_run.max())[0]
    axes_w = int(spines.max()) - int(spines.min())
    assert axes_w > 0, "no axes spines found in the panel crop"

    gray = (a >= 190) & (a <= 228)
    labels, count = ndi.label(gray)
    areas = ndi.sum(gray, labels, range(1, count + 1))
    best = None
    for index, sl in enumerate(ndi.find_objects(labels)):
        if sl is None:
            continue
        ys, xs = sl
        w, h = xs.stop - xs.start, ys.stop - ys.start
        if w > 50 and h > 50 and (best is None or areas[index] > best[0]):
            best = (areas[index], w)
    assert best is not None, "no legend frame found in the panel crop"
    return best[1] / axes_w


def assert_raster_rendered(png_path, figure_svg, min_colors=100):
    """Every embedded <image> (the colour bars) must actually be decoded."""
    svg = open(figure_svg, encoding="utf-8").read()
    image = Image.open(png_path).convert("RGB")
    results = []
    for tag in IMAGE_RE.findall(svg):
        box = re.search(
            r'\bx="([0-9.]+)"\s+y="([0-9.]+)"\s+width="([0-9.]+)"\s+height="([0-9.]+)"',
            tag,
        )
        x, y, w, h = (float(v) for v in box.groups())
        crop = image.crop((int(x), int(y), int(round(x + w)), int(round(y + h))))
        colours = np.asarray(crop).reshape(-1, 3)
        unique = len(np.unique(colours, axis=0))
        results.append(((x, y, w, h), unique, colours.mean(axis=0)))
        assert unique > min_colors, (
            f"<image> box {(x, y, w, h)} rendered only {unique} unique colours "
            f"(<= {min_colors}); the raster was not decoded (resvg raster-images?)"
        )
    assert results, "no <image> element found in the composed SVG"
    return results


def assert_legend_ratios(png_path, cell_boxes, max_rel_err=0.15):
    """Our legend/plot-area ratio must match the reference panel's."""
    image = Image.open(png_path).convert("RGB")
    results = []
    for panel, ref_name in LEGEND_REFERENCE.items():
        ref_path = os.path.join(REFERENCE_PANELS, ref_name + ".png")
        x, y, w, h = cell_boxes[panel]
        crop = image.crop((int(round(x)), int(round(y)),
                           int(round(x + w)), int(round(y + h))))
        ours = legend_ratio(crop)
        reference = legend_ratio(Image.open(ref_path))
        rel_err = abs(ours - reference) / reference
        results.append((panel, ours, reference, rel_err))
        assert rel_err < max_rel_err, (
            f"{panel}: legend width / plot width = {ours:.4f} vs reference "
            f"{reference:.4f} ({rel_err * 100:.1f} % relative error)"
        )
    return results


def assert_factor_order(figure_svg, f_cell):
    """The F panel's y tick text, top-to-bottom, must read F0..F6 (like imshow)."""
    svg = open(figure_svg, encoding="utf-8").read()
    _, cell_y, _, cell_h = f_cell
    rows = []
    for match in TEXT_RE.finditer(svg):
        text = match.group(2).strip()
        if text in FACTOR_ORDER:
            y = float(re.search(r'\by="([0-9.\-]+)"', match.group(1)).group(1))
            if cell_y <= y <= cell_y + cell_h:
                rows.append((y, text))
    rows.sort()
    order = [text for _, text in rows]
    assert order == FACTOR_ORDER, (
        f"F-cell y ticks top-to-bottom are {order}, expected {FACTOR_ORDER} "
        f"(pcolormesh must not mirror the imshow panel)"
    )
    return rows
