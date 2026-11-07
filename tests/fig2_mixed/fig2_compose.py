#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_fig2` 的组图与转换：convert / 逐块 compose / stack。

第 1 节（`svg_grid_convert`）与第 2、3 节（三次 `plot-grid`）都在这里。
"""

import os
import re
import subprocess
import sys

from fig2_common import (
    BLOCKS,
    BLOCKS_DIR,
    CONVERT,
    GRID,
    HERE,
    OUT,
    PANELS,
    TAGGED,
    W,
    run,
    svg_canvas,
)


def convert_panels():
    os.makedirs(TAGGED, exist_ok=True)
    rows = []
    for block in BLOCKS:
        for name in block["inputs"]:
            src = os.path.join(PANELS, name) if os.path.exists(os.path.join(PANELS, name)) \
                else os.path.join(OUT, name)
            dst = os.path.join(TAGGED, name)
            result = subprocess.run([CONVERT, "--input", src, "--output", dst, "--report"],
                                    capture_output=True, text=True)
            svg = open(dst, encoding="utf-8").read() if os.path.exists(dst) else ""
            rows.append(dict(
                panel=name, src=os.path.relpath(src, HERE), canvas=svg_canvas(src)
                if src.endswith(".svg") else None,
                exit=result.returncode,
                stderr=result.stderr.strip(),
                path=svg.count("<path"), use=svg.count("<use"),
                symbol=svg.count("<symbol"),
                gtransform=len(re.findall(r"<g\b[^>]*\btransform=", svg)),
                imgtf=len(re.findall(r"<image\b[^>]*\btransform=", svg)),
                text=svg.count("<text"), image=svg.count("<image"),
                polyline=svg.count("<polyline"), polygon=svg.count("<polygon"),
                rect=svg.count("<rect"),
            ))
            if result.returncode != 0:
                sys.stdout.write(f"\n$ {CONVERT} --input {src} --output {dst}\n")
                sys.stdout.write("  [stderr] " + result.stderr.strip() + "\n")
                raise SystemExit(f"convert failed for {name}")
    sys.stdout.write(f"\nconverted {len(rows)} panels -> {os.path.relpath(TAGGED, HERE)}\n")
    return rows


PANEL_BOX_RE = re.compile(
    r'<rect\s+data-panel-box="main"\s+x="([0-9.\-]+)"\s+y="([0-9.\-]+)"\s+'
    r'width="([0-9.\-]+)"\s+height="([0-9.\-]+)"')
IMAGE_RE = re.compile(
    r'<image\b[^>]*?\bx="([0-9.\-]+)"\s+y="([0-9.\-]+)"\s+width="([0-9.\-]+)"\s+'
    r'height="([0-9.\-]+)"')


def panel_boxes(svg):
    """Per-cell panel boxes, document order. The composer also emits one
    canvas-level `data-panel-box` rect before the cells; splitting on the cell
    marker skips it."""
    boxes = []
    for chunk in svg.split('data-svg-grid-cell="')[1:]:
        match = PANEL_BOX_RE.search(chunk)
        if match:
            boxes.append(tuple(float(v) for v in match.groups()))
    return boxes


def images(svg):
    return [tuple(float(v) for v in match) for match in IMAGE_RE.findall(svg)]


def analytic_cells(block):
    """cell rect for every panel in a band, in the band's own canvas."""
    top, right, bottom, left = block["margin"]
    n = len(block["inputs"])
    ncol = block["ncol"]
    nrow = (n + ncol - 1) // ncol
    available_w = W - left - right - (ncol - 1) * block["gap"]
    available_h = block["height"] - top - bottom - (nrow - 1) * block["gap"]
    canvas_w, canvas_h = block["canvas"][0]
    # columns take the panels' own aspect ratios (all panels of a band share one)
    aspects = [cw / ch for cw, ch in block["canvas"][:ncol]]
    total = sum(aspects)
    cells = []
    for index in range(n):
        row, col = divmod(index, ncol)
        x = left + sum(available_w * a / total for a in aspects[:col]) + col * block["gap"]
        cell_w = available_w * aspects[col] / total
        y = top + row * (canvas_h + block["gap"])
        cells.append((x, y, cell_w, canvas_h))
    return cells


def compose(letter_size, letter_vjust):
    os.makedirs(BLOCKS_DIR, exist_ok=True)
    report = []
    for block in BLOCKS:
        expected = analytic_cells(block)
        out = os.path.join(BLOCKS_DIR, block["name"] + ".svg")
        cmd = [GRID, "plot-grid"]
        for name in block["inputs"]:
            cmd += ["--input", os.path.join(TAGGED, name)]
        cmd += [
            "--output", out,
            "--width", str(W), "--height", str(block["height"]),
            "--ncol", str(block["ncol"]), "--gap", str(block["gap"]),
            "--plot-margin", ",".join(str(v) for v in block["margin"]),
            "--labels", block["labels"],
            "--label-size", str(letter_size),
            "--label-fontfamily", "DejaVu Serif", "--label-fontface", "regular",
            "--label-x", "0", "--label-y", "1", "--hjust", "0", "--vjust", f"{letter_vjust:.6f}",
        ]
        result = run(cmd, f"block {block['name']}")

        svg = open(out, encoding="utf-8").read()
        boxes = panel_boxes(svg)
        imgs = images(svg)
        measured = []
        for index, (name, cell, canvas) in enumerate(
                zip(block["inputs"], expected, block["canvas"])):
            box = boxes[index] if index < len(boxes) else None
            # sx/sy from the embedded <image> box (present in every panel: a bitmap
            # wrapper's bitmap, or panel A's colour-bar raster).
            sx = sy = sx_coord = sy_coord = None
            if index < len(imgs):
                tagged_img = images(open(os.path.join(TAGGED, name), encoding="utf-8").read())
                if tagged_img:
                    ix, iy, iw, ih = tagged_img[0]
                    ox, oy, ow, oh = imgs[index]
                    sx, sy = ow / iw, oh / ih
                    # manual's solve on a `data-scale="xy"` element's coordinates
                    if abs(ix) > 1e-6:
                        sx_coord = (ox - cell[0]) / ix
                    if abs(iy) > 1e-6:
                        sy_coord = (oy - cell[1]) / iy
            measured.append(dict(panel=name, cell=cell, panel_box=box, canvas=canvas,
                                 aspect_cell=cell[2] / cell[3], aspect_canvas=canvas[0] / canvas[1],
                                 sx=sx, sy=sy, sx_coord=sx_coord, sy_coord=sy_coord))
        report.append(dict(block=block, svg=out, stderr=result.stderr, measured=measured))
    return report


def stack():
    svg = os.path.join(OUT, "Figure2.svg")
    cmd = [GRID, "plot-grid"]
    for block in BLOCKS:
        cmd += ["--input", os.path.join(BLOCKS_DIR, block["name"] + ".svg")]
    cmd += ["--output", svg, "--output-png", os.path.join(OUT, "Figure2.png"),
            "--png-dpi", "600", "--width", str(W), "--ncol", "1",
            "--plot-margin", "0", "--gap", "0"]
    result = run(cmd, "stack")
    return svg, result.stderr
