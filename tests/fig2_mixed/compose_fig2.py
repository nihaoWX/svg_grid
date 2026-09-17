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
   dialect residue, letter ink width, white-band and ink-coverage rules, and a
   stacked equal-width comparison with the project's existing ``Figure 2.png``.

Every ``plot-grid`` / ``svg_grid_convert`` stderr stream is captured verbatim.

Usage:
    micromamba run -n python-bio python compose_fig2.py
"""

import os
import re
import subprocess
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))          # tools/svg_grid
CONVERT = os.path.join(ROOT, "convert/target/release/svg_grid_convert")
GRID = os.path.join(ROOT, "target/release/svg_grid")

OUT = os.path.join(HERE, "out")
PANELS = os.path.join(OUT, "panels")
TAGGED = os.path.join(OUT, "tagged")
BLOCKS_DIR = os.path.join(OUT, "blocks")

ORIGINAL = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/08_手稿/"
            "Figure/main/Figure 2.png")

SERIF = "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"
SERIF_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"

W = 4961
TARGETS = ["CYP1B1", "GC", "KIF11", "MMP9", "SERPINA1", "HSD11B1", "ALB", "BACE1", "GSTA1"]
LIG_LETTERS = ["A", "B", "C", "D"]

LETTER_SIZE = None      # filled from make_panels' rule below
LETTER_GAP = 22.0
LETTER_VJUST = -LETTER_GAP / 134.0

# three bands: (name, inputs in order, block height, margins t/r/b/l, ncol, gap, labels)
BLOCKS_SPEC = None


def run(cmd, label):
    result = subprocess.run(cmd, capture_output=True, text=True)
    sys.stdout.write(f"\n$ {' '.join(cmd)}\n")
    if result.stdout.strip():
        sys.stdout.write("  [stdout] " + result.stdout.strip() + "\n")
    if result.stderr.strip():
        sys.stdout.write("  [stderr] " + result.stderr.strip() + "\n")
    else:
        sys.stdout.write("  [stderr] (empty)\n")
    if result.returncode != 0:
        raise SystemExit(f"{label}: exit code {result.returncode}")
    return result


# --------------------------------------------------------------------------- #
# geometry (mirrors make_Figure2_v6.py; make_panels.py owns the same numbers)
# --------------------------------------------------------------------------- #
M_L = M_R = 150
M_T = M_B = 80
UW = W - M_L - M_R
LETTER_H, GAP = 130, 34
C_GAP, C_CELL, C_HDR = 19, 1541, 56
H_C = 3 * (C_CELL + C_HDR) + 2 * C_GAP              # 4829
B_GAP, B_CELL = 39, 1136
H_B = 760
UH = 6856
H_A = UH - (3 * LETTER_H + 2 * GAP) - H_B - H_C     # 809

BLOCKS = [
    dict(name="A", inputs=["A_heatmap.svg"], height=M_T + LETTER_H + H_A + GAP,
         margin=(M_T + LETTER_H, M_R, GAP, M_L), ncol=1, gap=0, labels="A",
         canvas=[[UW, H_A]]),
    dict(name="B", inputs=[f"B_{L}.svg" for L in LIG_LETTERS], height=LETTER_H + H_B + GAP,
         margin=(LETTER_H, M_R, GAP, M_L), ncol=4, gap=B_GAP, labels="B,,,",
         canvas=[[B_CELL, H_B]] * 4),
    dict(name="C", inputs=[f"C_{t}.svg" for t in TARGETS], height=LETTER_H + H_C + M_B,
         margin=(LETTER_H, M_R, M_B, M_L), ncol=3, gap=C_GAP,
         labels="C" + "," * 8, canvas=[[C_CELL, C_CELL + C_HDR]] * 9),
]


# --------------------------------------------------------------------------- #
def svg_canvas(path):
    with open(path, encoding="utf-8") as handle:
        text = handle.read(4000)
    match = re.search(r'viewBox="([^"]+)"', text)
    return tuple(float(v) for v in match.group(1).split()[2:])


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


def compose():
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
            "--label-size", str(LETTER_SIZE),
            "--label-fontfamily", "DejaVu Serif", "--label-fontface", "regular",
            "--label-x", "0", "--label-y", "1", "--hjust", "0", "--vjust", f"{LETTER_VJUST:.6f}",
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


# --------------------------------------------------------------------------- #
# verification
# --------------------------------------------------------------------------- #
def render_glyph(ch, size, fontpath=SERIF):
    font = ImageFont.truetype(fontpath, size)
    image = Image.new("L", (size * 4, size * 4), 0)
    ImageDraw.Draw(image).text((size, size), ch, fill=255, font=font)
    a = np.asarray(image)
    ys, xs = np.where(a > 60)
    return a[ys.min():ys.max() + 1, xs.min():xs.max() + 1]


def max_run(flags):
    best = cur = 0
    for flag in flags:
        cur = cur + 1 if flag else 0
        best = max(best, cur)
    return best


def paper_rules(png_path, label_size, anchors):
    """``anchors``: (char, panel_top_y) — the letter sits just above the panel,
    so its ink lives in [panel_top - 1.3 * size, panel_top - 2]."""
    a = np.asarray(Image.open(png_path).convert("L"))
    height, width = a.shape
    letters = []
    for char, top in anchors:
        y0, y1 = max(0, int(top - 1.35 * label_size)), max(1, int(top - 2))
        x0, x1 = max(0, M_L - 6), min(width, int(M_L + 1.35 * label_size))
        sub = a[y0:y1, x0:x1]
        ys, xs = np.where(sub < 100)
        assert len(xs), f"letter {char} has no ink in {x0, y0, x1, y1}"
        ink = sub[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
        letters.append(dict(
            char=char, ink_w=int(xs.max() - xs.min() + 1), ink_h=int(ys.max() - ys.min() + 1),
            w_frac=float((xs.max() - xs.min() + 1) / width),
            fill=float((ink < 100).mean()),
            reg_fill=float((render_glyph(char, label_size, SERIF) > 60).mean()),
            bold_fill=float((render_glyph(char, label_size, SERIF_BOLD) > 60).mean()),
            top=int(y0 + ys.min()), left=int(x0 + xs.min())))

    white = a >= 250
    row_full, col_full = white.all(axis=1), white.all(axis=0)
    dead = [i for i in range(0, height - 199, 200) if not (a[i:i + 200] < 245).any()]
    return dict(size=(width, height), letters=letters,
                max_white_row=max_run(row_full), max_white_col=max_run(col_full),
                dead_bands=dead, ink_fraction=float((a < 250).mean()),
                full_white_rows=int(row_full.sum()), full_white_cols=int(col_full.sum()))


def assert_raster_rendered(png_path, svg_path):
    svg = open(svg_path, encoding="utf-8").read()
    image = Image.open(png_path).convert("RGB")
    boxes = images(svg)
    results = []
    for x, y, w, h in boxes:
        crop = image.crop((int(round(x)), int(round(y)),
                           int(round(x + w)), int(round(y + h))))
        unique = len(np.unique(np.asarray(crop).reshape(-1, 3), axis=0))
        results.append(((x, y, w, h), unique))
    return results


def compare(ours_png):
    ours = Image.open(ours_png).convert("RGB")
    original = Image.open(ORIGINAL).convert("RGB")
    width = original.width

    def fit(image):
        if image.width == width:
            return image
        return image.resize((width, round(image.height * width / image.width)), Image.LANCZOS)

    original, ours = fit(original), fit(ours)
    separator = 28
    canvas = Image.new("RGB", (width, original.height + separator + ours.height), (60, 60, 60))
    canvas.paste(original, (0, 0))
    canvas.paste(ours, (0, original.height + separator))
    draw = ImageDraw.Draw(canvas)
    draw.text((14, 8), "ORIGINAL  Figure 2.png  (make_Figure2_v6.py, all raster)",
              fill=(255, 60, 60))
    draw.text((14, original.height + separator + 8),
              "MIXED  Figure2.png  (svg_grid plot-grid: vector panel A + bitmap panels B/C)",
              fill=(60, 120, 255))
    path = os.path.join(OUT, "Figure2_comparison.png")
    canvas.save(path)
    return path, ours.size


# --------------------------------------------------------------------------- #
def main():
    global LETTER_SIZE, LETTER_VJUST
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
    LETTER_SIZE = lo
    LETTER_VJUST = -LETTER_GAP / LETTER_SIZE
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
    report = compose()
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
    rules = paper_rules(png, LETTER_SIZE, anchors)
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
    assert rules["max_white_row"] < 200 and rules["max_white_col"] < 200
    assert not rules["dead_bands"]

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
    print(f"  ours {size[0]}x{size[1]}; stacked equal-width -> {os.path.relpath(path, HERE)}")

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
