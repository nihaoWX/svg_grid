#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_s22` 第 3+4 节：组图调用 + 逐格 scale/aspect 断言。

`plot-grid` 只做**规则**网格，S22 用嵌套搭：先组 AB / CDE，再用 `--ncol 1` 把
三块堆起来；两层都不给 `--rel-*`（列取面板自带 canvas 纵横比、行取块的自然高度）。
"""

import os

from s22_common import GRID, OUT, POINTS_RE, ROWS, TAGGED, canvas_size, run


def expected_cells(panels, aspects, width, gap, m_side, band, row_height):
    available = width - 2 * m_side - (len(aspects) - 1) * gap
    cells = []
    x = m_side
    for aspect in aspects:
        cell_w = available * aspect / sum(aspects)
        cells.append((x, band, cell_w, row_height))
        x += cell_w + gap
    assert abs(x - gap - (width - m_side)) < 1e-6, f"{panels}: row does not fill the width"
    return cells


def assert_aspects(name, aspects, cells):
    for cell, aspect in zip(cells, aspects):
        assert abs(cell[2] / cell[3] - aspect) < 1e-6, (
            f"{name}: cell aspect {cell[2] / cell[3]:.6f} != canvas aspect {aspect:.6f}"
        )


def assert_unit_scale(name, canvas, cells, tol=1e-4):
    """Each cell must be (almost exactly) the panel's own canvas → scale ~1.

    ``scale_panel_to_cell`` uses ``sx = cell.w/canvas.w`` and
    ``sy = cell.h/canvas.h`` independently; if either is far from 1 the panel is
    stretched, and (because ``font-size`` is never rescaled) its text is wrong.
    """
    for (cw, ch), (x, y, w, h) in zip(canvas, cells):
        sx, sy = w / cw, h / ch
        assert abs(sx - 1) < tol and abs(sy - 1) < tol, (
            f"{name}: cell scale ({sx:.6f}, {sy:.6f}) deviates from 1 "
            f"(canvas {cw}x{ch} -> cell {w:.3f}x{h:.3f})"
        )
    return [((w / cw), (h / ch)) for (cw, ch), (x, y, w, h) in zip(canvas, cells)]


def polygon_bboxes(svg):
    boxes = []
    for points in POINTS_RE.findall(svg):
        xs, ys = [], []
        for pair in points.split():
            x, y = pair.split(",")
            xs.append(float(x))
            ys.append(float(y))
        boxes.append((min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys)))
    return boxes


def assert_cells_land(block_svg, cells):
    """Each full-canvas background polygon must land on its expected cell box."""
    boxes = polygon_bboxes(block_svg)
    for (x, y, w, h) in cells:
        found = any(
            abs(bx - x) < 1.5 and abs(by - y) < 1.5 and abs(bw - w) < 1.5 and abs(bh - h) < 1.5
            for (bx, by, bw, bh) in boxes
        )
        assert found, f"no background polygon landed on the cell {x, y, w, h}"


def compose(width, prefix, geo):
    blocks_dir = os.path.join(OUT, "blocks")
    os.makedirs(blocks_dir, exist_ok=True)

    block_info = []
    scales = []          # (panel, sx, sy) for the report
    local_anchors = []   # (block_index, char, x, y_baseline) in block coords
    local_cells = []     # (block_index, panel, x, y, w, h) in block coords
    for block_index, row in enumerate(ROWS):
        canvas = [canvas_size(os.path.join(TAGGED, p + ".svg")) for p in row["panels"]]
        aspects = [w / h for (w, h) in canvas]

        column_gap = geo["g_col"] if len(row["panels"]) > 1 else 0
        available = width - 2 * geo["m_side"] - (len(aspects) - 1) * column_gap
        row_height = available / sum(aspects)
        cells = expected_cells(row["panels"], aspects, width, column_gap,
                               geo["m_side"], geo["band"], row_height)
        assert_aspects(row["name"], aspects, cells)
        for panel, (sx, sy) in zip(row["panels"],
                                   assert_unit_scale(row["name"], canvas, cells)):
            scales.append((panel, sx, sy))
        for label, cell in zip(row["labels"], cells):
            local_anchors.append((block_index, label, cell[0],
                                  cell[1] - geo["gap_letter"]))
        for panel, cell in zip(row["panels"], cells):
            local_cells.append((block_index, panel, cell[0], cell[1], cell[2], cell[3]))

        block_h = geo["band"] + row_height
        out = os.path.join(blocks_dir, row["name"] + ".svg")
        run([
            GRID, "plot-grid",
            *sum([["--input", os.path.join(TAGGED, p + ".svg")] for p in row["panels"]], []),
            "--output", out,
            "--width", str(width), "--height", str(block_h),
            "--ncol", str(len(row["panels"])),
            # No --rel-widths: the columns take each panel's own canvas aspect.
            "--plot-margin", f"{geo['band']},{geo['m_side']},0,{geo['m_side']}",
            "--gap", str(column_gap),
            "--labels", ",".join(row["labels"]),
            "--label-size", str(geo["label_size"]),
            "--label-fontfamily", "DejaVu Serif",
            "--label-fontface", "regular",
            "--label-x", "0", "--label-y", "1",
            "--hjust", "0", "--vjust", str(geo["vjust"]),
        ])
        assert_cells_land(open(out, encoding="utf-8").read(), cells)
        block_info.append(dict(name=row["name"], height=block_h))

    # Stack the three blocks with ncol=1 + rel-heights, no outer margin/gap: the
    # outer cell is then the block canvas itself (sx == sy == 1).
    total_h = sum(info["height"] for info in block_info)
    # block-local letter anchors -> figure coordinates (block b starts at the sum
    # of the heights of the blocks before it)
    offsets = []
    running = 0.0
    for info in block_info:
        offsets.append(running)
        running += info["height"]
    anchors = [(char, x, y + offsets[b]) for (b, char, x, y) in local_anchors]
    # every panel's cell rectangle in *figure* coordinates (blocks stacked)
    cell_boxes = {panel: (x, y + offsets[b], w, h)
                  for (b, panel, x, y, w, h) in local_cells}
    figure_svg = os.path.join(OUT, prefix + ".svg")
    run([
        GRID, "plot-grid",
        *sum([["--input", os.path.join(blocks_dir, info["name"] + ".svg")] for info in block_info], []),
        "--output", figure_svg,
        "--output-svgz", os.path.join(OUT, prefix + ".svgz"),
        "--output-png", os.path.join(OUT, prefix + ".png"),
        "--png-dpi", "600",
        "--width", str(width), "--height", str(total_h),
        "--ncol", "1",
        # No --rel-heights: each block row takes its own natural height (= the
        # block's canvas height, since blocks are W-wide), summing to total_h.
        "--plot-margin", "0",
        "--gap", "0",
    ])
    return figure_svg, total_h, scales, anchors, cell_boxes
