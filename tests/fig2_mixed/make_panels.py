#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Build the Figure 2 panels as inputs the `svg_grid` tag protocol can eat.

Figure 2 (the project's main figure, `make_Figure2_v6.py`) is THREE full-width
bands at A4 portrait 4961 x 7016 px @600 dpi:

    A  9x4 docking-score heatmap              (matplotlib)     -> VECTOR
    B  4 ligand 2D structures, 1x4            (RDKit -> Cairo) -> BITMAP
    C  9 binding-pocket close-ups, 3x3        (ChimeraX)       -> BITMAP

This script turns every band into standalone panel SVGs:

  * **A** is re-drawn from matplotlib with ``svg.fonttype = 'none'`` so the text
    stays ``<text>`` and the heatmap body is a ``pcolormesh`` (vector polygons);
    only the colour bar arrives as an embedded ``<image>`` (matplotlib always
    rasterises a colour bar).  The figure is drawn **at the exact size of the
    grid cell it will occupy** (4661 x 809 canvas units) and every point-valued
    size is multiplied by ``k = 600/72`` — a panel drawn at its design point size
    and then blown up into a cell keeps its ``font-size`` (the tag protocol never
    rescales it), so drawing it at cell scale is what makes ``s = cell/canvas``
    come out at 1.
  * **B** and **C** are the ChimeraX / RDKit rasters, wrapped in a **bitmap
    panel SVG**: an ``<image>`` filling an axis-aligned box with
    ``preserveAspectRatio="none"`` (so the box is filled, never letterboxed).
    The bitmap wrapper canvas is the grid cell itself, so ``s = 1`` and the
    ``<image>`` box is 1:1 with the embedded PNG (the PNG is resampled by the
    same ``contain()`` rule the project's own script uses, so the comparison with
    the existing Figure 2 is apples to apples).

Every panel carries only what the project's own script puts in it (target names
above the C cells, ligand chips below the B structures).  The research project is
only ever READ from; all outputs land in ``out/``.

Usage:
    micromamba run -n python-bio python make_panels.py
"""

import base64
import json
import math
import os
import shutil

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out")
PANELS = os.path.join(OUT, "panels")
RASTERS = os.path.join(OUT, "rasters")

# --- research project inputs (READ-ONLY) ----------------------------------- #
STEP = "/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/02_分子对接"
PANEL_DIR = os.path.join(STEP, "figure_panels")
SDFDIR = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/08_手稿/"
          "Supplemantary-Materials/Supplemantary-Materials-3/02_compound_structures")

# --------------------------------------------------------------------------- #
# Layout: verbatim copy of `make_Figure2_v6.py`'s geometry (so the composed
# figure and the existing Figure 2 can be compared panel for panel).
# --------------------------------------------------------------------------- #
W, H = 4961, 7016            # A4 @600 dpi
DPI = 600
M_L, M_R = 150, 150
M_T, M_B = 80, 80
UW = W - M_L - M_R           # 4661
UH = H - M_T - M_B           # 6856
X0 = M_L

LETTER_H = 130               # panel-letter row; the letter sits OUTSIDE the panel
GAP = 34                     # gap between bands

TARGETS = ["CYP1B1", "GC", "KIF11", "MMP9", "SERPINA1", "HSD11B1", "ALB", "BACE1", "GSTA1"]
BEST = {"CYP1B1": "C", "GC": "C", "KIF11": "A", "MMP9": "C", "SERPINA1": "A",
        "HSD11B1": "C", "ALB": "C", "BACE1": "C", "GSTA1": "D"}
SCORES = {
    "CYP1B1": [-11.04, -10.66, -11.55, -10.53],
    "GC": [-7.04, -6.10, -7.15, -6.24],
    "KIF11": [-7.32, -6.81, -6.68, -6.67],
    "MMP9": [-5.00, -5.34, -6.97, -5.16],
    "SERPINA1": [-8.22, -6.44, -7.83, -6.66],
    "HSD11B1": [-8.23, -7.71, -8.96, -5.50],
    "ALB": [-3.89, -0.02, -4.66, -4.47],
    "BACE1": [-4.16, -4.05, -6.59, -1.82],
    "GSTA1": [-3.31, -5.46, -1.58, -6.33],
}
LCOL = {"A": "#5FA8A0", "B": "#E0907E", "C": "#CFA15A", "D": "#9E9AC8"}
LIG_LETTERS = ["A", "B", "C", "D"]
LIG_FILES = {"A": "A_resveratrol_CID445154.sdf", "B": "B_stilbene_CID11502.sdf",
             "C": "C_triphenylethylene_CID6025.sdf", "D": "D_1,1-diphenylethylene_CID10740.sdf"}

# --- band geometry --- #
C_GAP = 19                              # C: 3 cols -> 3*1541 + 2*19 = 4661 = UW
C_CELL = (UW - 2 * C_GAP) // 3          # 1541
C_HDR = 56                              # target-name row above each C cell
H_C = 3 * (C_CELL + C_HDR) + 2 * C_GAP  # 4829

B_GAP = 39                              # B: 4 cols -> 4*1136 + 3*39 = 4661 = UW
B_CELL = (UW - 3 * B_GAP) // 4          # 1136
B_PAD_X, B_PAD_T = 26, 16
B_IMG_H = 560
B_CHIP_GAP, B_CHIP_H = 30, 96
B_CHIP_W = 180
H_B = B_PAD_T + B_IMG_H + B_CHIP_GAP + B_CHIP_H + 58            # 760

H_A = UH - (3 * LETTER_H + 2 * GAP) - H_B - H_C                 # 809

# --- fonts (DejaVu Serif regular, exactly as the project script uses) --- #
_FONT_DIR = "/usr/share/fonts/truetype/dejavu"
SERIF = os.path.join(_FONT_DIR, "DejaVuSerif.ttf")
SERIF_BOLD = os.path.join(_FONT_DIR, "DejaVuSerif-Bold.ttf")
PANEL_LETTER_PX = 120        # v6's panel letter
TARGET_FONT_PX = 62          # v6's target-name / ligand-chip font
LIG_DRAW_W, LIG_DRAW_H = 1100, 700      # v6's RDKit canvas

# --- panel A is re-drawn at cell scale: k = 600/72 --- #
K = DPI / 72.0               # 8.3333 : design pt (on a 600 dpi canvas) -> canvas units
HM_TICK_PT, HM_VAL_PT, HM_CB_PT = 8.5, 7.6, 8.0      # v6's typography
HM_AX = [0.105, 0.135, 0.815, 0.815]

POINT_RCPARAMS = [
    "font.size", "axes.linewidth", "axes.labelsize", "axes.titlesize", "axes.labelpad",
    "axes.titlepad", "lines.linewidth", "lines.markersize", "lines.markeredgewidth",
    "grid.linewidth", "patch.linewidth", "xtick.labelsize", "ytick.labelsize",
    "xtick.major.size", "xtick.minor.size", "xtick.major.width", "xtick.minor.width",
    "ytick.major.size", "ytick.minor.size", "ytick.major.width", "ytick.minor.width",
    "xtick.major.pad", "xtick.minor.pad", "ytick.major.pad", "ytick.minor.pad",
    "figure.titlesize", "errorbar.capsize",
]


# --------------------------------------------------------------------------- #
# helpers (the `contain()` / glyph-metric rules are copied from the project's
# own `make_Figure2_v6.py` and `compose_s22.py`)
# --------------------------------------------------------------------------- #
def nonwhite_bbox(img, thr=248, pad=8):
    a = np.asarray(img.convert("RGB"))
    mask = (a < thr).any(axis=2)
    ys, xs = np.where(mask)
    if len(xs) == 0:
        return (0, 0, img.size[0], img.size[1])
    return (max(0, xs.min() - pad), max(0, ys.min() - pad),
            min(a.shape[1], xs.max() + pad), min(a.shape[0], ys.max() + pad))


def contain(path, tw, th, trim=True):
    """v6's rule verbatim: trim the white border, fit inside tw x th, centre."""
    img = Image.open(path).convert("RGB")
    if trim:
        img = img.crop(nonwhite_bbox(img, pad=4))
    sw, sh = img.size
    sc = min(tw / sw, th / sh)
    img = img.resize((max(1, int(sw * sc + 0.5)), max(1, int(sh * sc + 0.5))), Image.LANCZOS)
    canvas = Image.new("RGB", (tw, th), "white")
    canvas.paste(img, ((tw - img.size[0]) // 2, (th - img.size[1]) // 2))
    return canvas, sc


def b64_png(path):
    with open(path, "rb") as handle:
        return "data:image/png;base64," + base64.b64encode(handle.read()).decode("ascii")


def glyph_metrics(text, size, fontpath=SERIF):
    """(ink bbox relative to the PIL 'la' anchor, ascent) at ``size`` px."""
    font = ImageFont.truetype(fontpath, size)
    bbox = font.getbbox(text)
    ascent = font.getmetrics()[0]
    return bbox, ascent


def baseline_for_ink_top(top, text, size, fontpath=SERIF):
    """SVG baseline y so the glyph's ink top sits at ``top``."""
    bbox, ascent = glyph_metrics(text, size, fontpath)
    return top + ascent - bbox[1]


def ink_width(text, size, fontpath=SERIF):
    bbox, _ = glyph_metrics(text, size, fontpath)
    return bbox[2] - bbox[0]


def ink_height(text, size, fontpath=SERIF):
    bbox, _ = glyph_metrics(text, size, fontpath)
    return bbox[3] - bbox[1]


def size_for_width(text, target_w, fontpath=SERIF):
    lo, hi = 8, 4000
    while lo < hi:
        mid = (lo + hi) // 2
        if ink_width(text, mid, fontpath) < target_w:
            lo = mid + 1
        else:
            hi = mid
    return lo


def write_png(img, name):
    os.makedirs(RASTERS, exist_ok=True)
    path = os.path.join(RASTERS, name)
    img.save(path, optimize=True)
    return path


def write_svg(name, body, canvas_w, canvas_h):
    os.makedirs(PANELS, exist_ok=True)
    svg = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'xmlns:xlink="http://www.w3.org/1999/xlink" '
        f'width="{canvas_w}" height="{canvas_h}" viewBox="0 0 {canvas_w} {canvas_h}">\n'
        f'{body}\n</svg>\n'
    )
    path = os.path.join(PANELS, name)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(svg)
    return path


def fmt(value):
    """Shortest round-trippable decimal, e.g. 1541 -> '1541', 93.2 -> '93.2'."""
    if float(value).is_integer():
        return str(int(value))
    return f"{value:.4f}".rstrip("0").rstrip(".")


# --------------------------------------------------------------------------- #
# panel A: the docking-score heatmap, re-emitted as SVG (vector)
# --------------------------------------------------------------------------- #
def build_panel_a():
    import matplotlib
    matplotlib.use("Agg")
    matplotlib.rcParams["svg.fonttype"] = "none"      # keep <text>, do not outline
    matplotlib.rcParams.update({
        key: float(matplotlib.rcParamsDefault[key]) * K
        for key in POINT_RCPARAMS
        if not isinstance(matplotlib.rcParamsDefault[key], str)
    })
    import matplotlib.pyplot as plt
    from matplotlib.colors import LinearSegmentedColormap

    data = np.array([SCORES[t] for t in TARGETS])
    cmap = LinearSegmentedColormap.from_list(
        "dock", ["#1a4a5a", "#5FA8A0", "#e8e4d8", "#E0907E"], N=256)

    fig = plt.figure(figsize=(UW / 72.0, H_A / 72.0))          # -> viewBox = 4661 x 809
    ax = fig.add_axes(HM_AX)
    # pcolormesh (not imshow): keeps the heatmap body as vector polygons.
    mesh = ax.pcolormesh(data, vmin=-12, vmax=0, cmap=cmap,
                         edgecolors="none", shading="nearest")
    ax.invert_yaxis()                                          # CYP1B1 on top, like imshow
    ax.set_xticks(range(4))
    ax.set_xticklabels(LIG_LETTERS)
    ax.set_yticks(range(9))
    ax.set_yticklabels(TARGETS)
    ax.tick_params(labelsize=HM_TICK_PT * K)
    for lab, t in zip(ax.get_xticklabels(), LIG_LETTERS):
        lab.set_bbox(dict(facecolor=LCOL[t], edgecolor="none", boxstyle="round,pad=0.35"))
    for i in range(9):
        for j in range(4):
            v = data[i, j]
            ax.text(j, i, f"{v:.2f}", ha="center", va="center", fontsize=HM_VAL_PT * K,
                    color="white" if v < -6.5 else "black")
            if abs(v - data[i].min()) < 1e-9:
                ax.add_patch(plt.Rectangle((j - 0.5, i - 0.5), 1, 1, fill=False,
                                           edgecolor="black", lw=1.8 * K))
    ax.set_xlabel("compound", fontsize=HM_TICK_PT * K)
    for spine in ax.spines.values():
        spine.set_visible(True)
    cax = fig.add_axes([0.938, HM_AX[1], 0.020, HM_AX[3]])
    cb = fig.colorbar(mesh, cax=cax)
    cb.set_label("kcal/mol", fontsize=HM_CB_PT * K)
    cb.ax.tick_params(labelsize=(HM_CB_PT - 1) * K)

    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "A_heatmap.svg")
    fig.savefig(path, format="svg")
    plt.close(fig)
    return path, (UW, H_A)


# --------------------------------------------------------------------------- #
# panel B: the four ligand 2D structures -> bitmap wrappers
# --------------------------------------------------------------------------- #
def render_ligand_pngs():
    from rdkit import Chem
    from rdkit.Chem import AllChem, Draw
    os.makedirs(RASTERS, exist_ok=True)
    paths = {}
    for label, filename in LIG_FILES.items():
        supplier = Chem.SDMolSupplier(os.path.join(SDFDIR, filename), removeHs=False)
        mol = next(m for m in supplier if m is not None)
        mol = Chem.RemoveHs(mol)
        AllChem.Compute2DCoords(mol)
        drawer = Draw.MolDraw2DCairo(LIG_DRAW_W, LIG_DRAW_H)
        drawer.drawOptions().bondLineWidth = 6
        drawer.drawOptions().multipleBondOffset = 0.14
        drawer.drawOptions().padding = 0.05
        drawer.DrawMolecule(mol)
        drawer.FinishDrawing()
        path = os.path.join(RASTERS, f"lig_{label}_native.png")
        with open(path, "wb") as handle:
            handle.write(drawer.GetDrawingText())
        paths[label] = path
    return paths


def build_panel_b(lig_native):
    """One 1136 x 760 bitmap panel per ligand: <image> + vector chip."""
    panels = []
    for index, label in enumerate(LIG_LETTERS):
        box_w, box_h = B_CELL - 2 * B_PAD_X, B_IMG_H
        fitted, scale = contain(lig_native[label], box_w, box_h, trim=True)
        png = write_png(fitted, f"lig_{label}.png")

        cx = B_CELL / 2.0
        chip_y = B_PAD_T + B_IMG_H + B_CHIP_GAP
        ink_top = chip_y + (B_CHIP_H - ink_height(label, TARGET_FONT_PX)) / 2.0
        baseline = baseline_for_ink_top(ink_top, label, TARGET_FONT_PX)

        body = (
            f'<image x="{fmt(B_PAD_X)}" y="{fmt(B_PAD_T)}" width="{fmt(box_w)}" '
            f'height="{fmt(box_h)}" preserveAspectRatio="none" xlink:href="{b64_png(png)}"/>\n'
            f'<rect x="{fmt(cx - B_CHIP_W / 2)}" y="{fmt(chip_y)}" '
            f'width="{B_CHIP_W}" height="{B_CHIP_H}" rx="14" fill="{LCOL[label]}"/>\n'
            f'<text x="{fmt(cx)}" y="{fmt(baseline)}" text-anchor="middle" '
            f'font-family="DejaVu Serif" font-size="{TARGET_FONT_PX}" '
            f'fill="black">{label}</text>'
        )
        panels.append(dict(
            label=label, svg=write_svg(f"B_{label}.svg", body, B_CELL, H_B),
            native=(LIG_DRAW_W, LIG_DRAW_H), box=(box_w, box_h),
            contain_scale=scale,
            image_px=fitted.size,
        ))
    return panels


# --------------------------------------------------------------------------- #
# panel C: the nine close-ups -> bitmap wrappers (with their target name)
# --------------------------------------------------------------------------- #
def build_panel_c():
    panels = []
    for target in TARGETS:
        native = os.path.join(PANEL_DIR, f"closeup_{target}_{BEST[target]}.png")
        native_size = Image.open(native).size
        fitted, scale = contain(native, C_CELL, C_CELL, trim=True)
        png = write_png(fitted, f"closeup_{target}_{BEST[target]}.png")

        top = C_HDR - 50.0            # v6 draws the name's ascender top 50 px above the cell
        baseline = baseline_for_ink_top(top, target, TARGET_FONT_PX)
        body = (
            f'<text x="{fmt(C_CELL / 2)}" y="{fmt(baseline)}" text-anchor="middle" '
            f'font-family="DejaVu Serif" font-size="{TARGET_FONT_PX}" '
            f'fill="black">{target}</text>\n'
            f'<image x="0" y="{C_HDR}" width="{C_CELL}" height="{C_CELL}" '
            f'preserveAspectRatio="none" xlink:href="{b64_png(png)}"/>'
        )
        panels.append(dict(
            target=target, svg=write_svg(f"C_{target}.svg", body, C_CELL, C_CELL + C_HDR),
            native=native_size, box=(C_CELL, C_CELL), contain_scale=scale,
            image_px=fitted.size,
        ))
    return panels


# --------------------------------------------------------------------------- #
def main():
    os.makedirs(OUT, exist_ok=True)
    for directory in (PANELS, RASTERS):
        if os.path.isdir(directory):
            shutil.rmtree(directory)
        os.makedirs(directory)

    letter_size = size_for_width("A", 0.020 * W)
    glyph_h = ink_height("A", letter_size)

    manifest = dict(
        canvas=dict(W=W, H=H, DPI=DPI, M_T=M_T, M_B=M_B, M_L=M_L, M_R=M_R,
                    UW=UW, UH=UH, LETTER_H=LETTER_H, GAP=GAP, H_A=H_A, H_B=H_B, H_C=H_C),
        blocks=[
            dict(name="A", height=M_T + LETTER_H + H_A + GAP, margin=[M_T + LETTER_H, M_R, GAP, M_L],
                 ncol=1, gap=0, labels="A"),
            dict(name="B", height=LETTER_H + H_B + GAP, margin=[LETTER_H, M_R, GAP, M_L],
                 ncol=4, gap=B_GAP, labels="B,,,"),
            dict(name="C", height=LETTER_H + H_C + M_B, margin=[LETTER_H, M_R, M_B, M_L],
                 ncol=3, gap=C_GAP, labels="C,,,,,,,,"),
        ],
        letter=dict(size_units=letter_size, ink_w=0.020 * W, glyph_ink_h=glyph_h,
                    gap=22.0, vjust=-22.0 / letter_size),
    )

    print("== panel A (vector, matplotlib) ==")
    a_path, a_canvas = build_panel_a()
    manifest["A"] = dict(svg=a_path, canvas=a_canvas,
                         k=K, font_pt=dict(axis=HM_TICK_PT, value=HM_VAL_PT, cbar=HM_CB_PT))
    print(f"  {os.path.basename(a_path)}  canvas={a_canvas}  k={K:.4f}")

    print("== panel B (bitmap wrappers, RDKit -> Cairo PNG) ==")
    lig_native = render_ligand_pngs()
    manifest["B"] = build_panel_b(lig_native)
    for entry in manifest["B"]:
        print(f"  {os.path.basename(entry['svg'])}  native={entry['native']} "
              f"box={entry['box']}  contain_scale={entry['contain_scale']:.4f} "
              f"image_px={entry['image_px']}")

    print("== panel C (bitmap wrappers, ChimeraX PNG) ==")
    manifest["C"] = build_panel_c()
    for entry in manifest["C"]:
        print(f"  {os.path.basename(entry['svg'])}  native={entry['native']} "
              f"box={entry['box']}  contain_scale={entry['contain_scale']:.4f} "
              f"image_px={entry['image_px']}")

    with open(os.path.join(OUT, "manifest.json"), "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2, ensure_ascii=False)
    print(f"\nmanifest -> {os.path.join(OUT, 'manifest.json')}")
    print(f"letter: size={letter_size} units, ink {0.020 * W:.1f} px "
          f"({0.020 * 100:.2f} % of {W}); vjust={-22.0 / letter_size:.5f}")
    print(f"blocks: " + "  ".join(f"{b['name']} h={b['height']}" for b in manifest["blocks"]))
    print(f"sum of block heights = {sum(b['height'] for b in manifest['blocks'])} (H={H})")


if __name__ == "__main__":
    main()
