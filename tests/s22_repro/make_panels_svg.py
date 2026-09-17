#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Re-render the six NMF k-selection panels of Supplementary Figure S22 as SVG.

Test asset for `tools/svg_grid/`: it exists so the *matplotlib* dialect can be
fed through `tools/svg_grid/convert/` and composed by `svg_grid plot-grid`.

Provenance: a faithful copy of
`research/白藜芦醇胰腺癌/steps/05_空间深化/spatial_transcription/nmf_k_selection/figure_scripts/make_figures_hires.py`
(the plotting code is unchanged; styling, axis limits and data are identical).
Only the output side differs:

  * output is SVG, with ``svg.fonttype = 'none'`` so text stays ``<text>``;
  * panel F is emitted twice: once with ``imshow`` (as in the original) and once
    with ``pcolormesh`` (vector-friendly);
  * everything is written under this directory — the research project is only
    ever READ (its JSON inputs), never written.

Two output modes
----------------

**native** (default) — every panel at its design ``figsize`` (A/B/C/D
``504x360`` pt, E ``460.8x345.6`` pt, F ``648x288`` pt), written to ``out/panels``.

**sized** (``--scale-to-cell --target-width 4961``) — every panel re-rendered at
the size of the S22 grid cell it will occupy, written to ``out/panels_sized``.

Why the sized mode exists: the ``svg_grid`` tag protocol never rescales
``font-size`` (``tools/svg_grid/src/transform.rs`` only rewrites geometry,
``stroke-width`` and radii).  A panel drawn at its design size and then blown up
into a cell therefore keeps its text at the design point size while its geometry
grows, so the text ends up far too small relative to the panel.  The fix is to
draw the panel *already* at cell size: multiply ``figsize`` AND every
point-valued rcParam (font sizes, line widths, marker sizes, tick sizes/pads,
legend metrics) by one factor ``k_p``.  The panel is then scaled
*isotropically* — it looks identical to the native one, only its coordinate
system (and thus the SVG ``viewBox``) grows to exactly the target cell, and the
compose-time ``cell/canvas`` scale is ~1.

``k_p`` is derived analytically from the layout (not fitted): with
``available = W - 2*M_SIDE - (n-1)*G_COL`` for a row of ``n`` panels,
``row_h = available / sum(aspects)`` and ``cell_w = available*aspect_i/sum``,
so ``k_p = cell_w / (figsize_w_inch * 72)``.

Usage:
    micromamba run -n python-bio python make_panels_svg.py [--out DIR]
    micromamba run -n python-bio python make_panels_svg.py \\
        --scale-to-cell --target-width 4961
"""

import argparse
import json
import os

import numpy as np
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

# --- research project inputs (READ-ONLY) -----------------------------------
BASE = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/05_空间深化/"
        "spatial_transcription/nmf_k_selection")
DATA = os.path.join(BASE, "data")

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_OUT = os.path.join(HERE, "out", "panels")
DEFAULT_SIZED_OUT = os.path.join(HERE, "out", "panels_sized")

# --- S22 layout, mirrored from the project's make_FigureS22.py (see
#     compose_s22.py, which owns the same constants) -------------------------- #
CANVAS_WIDTH = 4961
M_SIDE_FRAC = 40 / 4961        # outer left/right margin
G_COL_FRAC = 40 / 4961         # horizontal gap between panels in a row

# native figsize (inches) of every panel: matplotlib writes the SVG viewBox in
# points, so the native canvas is figsize * 72.
PANEL_FIGSIZE = {
    "fig1_pairwise_matched_cos": (7, 5),          # A
    "fig2_shared_ecotype_count": (7, 5),          # B
    "fig3_empty_factors": (7, 5),                 # C
    "fig4_transferability": (7, 5),               # D
    "fig5_joint_nmf": tuple(matplotlib.rcParams["figure.figsize"]),  # E (default)
    "fig6_joint_k7_composition_imshow": (9, 4),   # F (imshow variant)
    "fig6_joint_k7_composition_pcolormesh": (9, 4),  # F (vector variant)
}

# the S22 grid: which panel occupies which cell (same structure as
# compose_s22.py).  panel F's imshow twin shares F's cell.
ROWS = [
    ["fig1_pairwise_matched_cos", "fig2_shared_ecotype_count"],
    ["fig3_empty_factors", "fig4_transferability", "fig5_joint_nmf"],
    ["fig6_joint_k7_composition_pcolormesh"],
]
# panel -> the panel whose cell it shares (its k_p is reused)
CELL_TWIN = {"fig6_joint_k7_composition_imshow":
             "fig6_joint_k7_composition_pcolormesh"}

ALL_PANELS = [p for row in ROWS for p in row] + list(CELL_TWIN)

# Every rcParam that is a length in points.  matplotlib resolves the relative
# string sizes ('medium'/'large') from ``font.size`` at draw time, so scaling
# ``font.size`` covers axis labels, ticks, titles and legends; the explicit
# entries below cover the ones that are not derived from it.
#
# IMPORTANT (units): never put a *font-relative* rcParam in this list.  matplotlib
# defines legend.handlelength/handleheight/handletextpad/borderpad/labelspacing/
# columnspacing/borderaxespad in "font-size units" -- their absolute size is
# ``value * font.size`` resolved at draw time.  Because ``font.size`` is already
# multiplied by k, scaling one of these by k would scale it by k^2 in absolute
# terms (the legend handle/padding would grow one extra factor of k, and only in
# the ``--scale-to-cell`` output where k > 1).  They are therefore deliberately
# absent; only true point lengths belong here.
POINT_RCPARAMS = [
    "font.size",                    # pt (base for all 'medium'/'large' sizes)
    "axes.linewidth",               # pt
    "axes.labelsize",               # pt (default 'medium' -> resolved from font.size)
    "axes.titlesize",               # pt ('large')
    "axes.labelpad",                # pt
    "axes.titlepad",                # pt
    "lines.linewidth",              # pt
    "lines.markersize",             # pt
    "lines.markeredgewidth",        # pt
    "grid.linewidth",               # pt
    "patch.linewidth",              # pt
    "xtick.labelsize",              # pt ('medium')
    "ytick.labelsize",              # pt ('medium')
    "xtick.major.size",             # pt
    "xtick.minor.size",             # pt
    "xtick.major.width",            # pt
    "xtick.minor.width",            # pt
    "ytick.major.size",             # pt
    "ytick.minor.size",             # pt
    "ytick.major.width",            # pt
    "ytick.minor.width",            # pt
    "xtick.major.pad",              # pt
    "xtick.minor.pad",              # pt
    "ytick.major.pad",              # pt
    "ytick.minor.pad",              # pt
    # legend.fontsize is a *font-relative* size ('medium') -> handled by font.size.
    # legend.handlelength/handleheight/handletextpad/borderpad/labelspacing/
    # columnspacing/borderaxespad are in font-size units -> also handled by
    # font.size.  None of them may be scaled here.
    "figure.titlesize",             # pt ('large')
    "errorbar.capsize",             # pt
]


# captured once, from the pristine defaults, so repeated scaling never compounds
BASE_POINT_RCPARAMS = {key: matplotlib.rcParamsDefault[key]
                       for key in POINT_RCPARAMS}


def scaled_rcparams(k):
    """Point-valued rcParams multiplied by ``k`` (strings are left alone)."""
    return {key: float(value) * k
            for key, value in BASE_POINT_RCPARAMS.items()
            if isinstance(value, (int, float)) and not isinstance(value, bool)}


def apply_scale(k):
    """Set every point-valued rcParam to its default times ``k``."""
    matplotlib.rcParams.update(scaled_rcparams(k))


def cell_scales(target_width):
    """Analytic per-panel ``k_p`` (and the resulting cell size) for S22.

    ``available`` is the width a row has to fill after the outer margins and the
    in-row gaps; the row is split by panel aspect ratio, exactly like the
    composer's ``--rel-widths``.
    """
    m_side = round(target_width * M_SIDE_FRAC)
    g_col = round(target_width * G_COL_FRAC)
    available = target_width - 2 * m_side

    scales = {}
    for row in ROWS:
        aspects = [PANEL_FIGSIZE[p][0] / PANEL_FIGSIZE[p][1] for p in row]
        row_available = available - (len(row) - 1) * g_col
        total = sum(aspects)
        row_h = row_available / total
        for panel, aspect in zip(row, aspects):
            fig_w_in, _ = PANEL_FIGSIZE[panel]
            cell_w = row_available * aspect / total
            scales[panel] = dict(k=cell_w / (fig_w_in * 72),
                                 cell_w=cell_w, cell_h=row_h)
    for panel, twin in CELL_TWIN.items():
        scales[panel] = dict(scales[twin])
    return scales


def load(out_dir):
    os.makedirs(out_dir, exist_ok=True)
    matplotlib.rcParams["svg.fonttype"] = "none"   # keep <text>, do not outline
    with open(os.path.join(DATA, "reproducibility_results.json")) as f:
        R = json.load(f)
    with open(os.path.join(DATA, "summary_tables.json")) as f:
        S = json.load(f)
    with open(os.path.join(DATA, "joint_nmf_selection.json")) as f:
        J = json.load(f)
    with open(os.path.join(DATA, "transferability.json")) as f:
        T = json.load(f)
    with open(os.path.join(DATA, "joint_factor_compositions.json")) as f:
        JC = json.load(f)
    return out_dir, R, S, J, T, JC


def save(out_dir, name):
    path = os.path.join(out_dir, name)
    plt.savefig(path, format="svg")
    plt.close()
    print("  ", name)
    return path


def render(out_dir, R, S, J, T, JC, scales):
    ks = list(range(5, 10))
    ks_j = list(range(5, 11))
    panel_figsize = {p: (w * scales[p]["k"], h * scales[p]["k"])
                     for p, (w, h) in PANEL_FIGSIZE.items()}
    k6 = scales["fig6_joint_k7_composition_pcolormesh"]["k"]

    def kk(panel):
        return scales[panel]["k"]

    # ---- Fig 1: pairwise matched cosine + null (unchanged) ----
    apply_scale(kk("fig1_pairwise_matched_cos"))
    plt.figure(figsize=panel_figsize["fig1_pairwise_matched_cos"])
    obs = [np.mean(R["pair_repro"][str(k)]) for k in ks]
    null_m = [np.mean(R["null_repro"][str(k)]) for k in ks]
    null_95 = [np.percentile(R["null_repro"][str(k)], 95) for k in ks]
    plt.plot(ks, obs, "-o", label="observed matched cos")
    plt.plot(ks, null_m, "--o", label="null mean (permuted)")
    plt.plot(ks, null_95, ":o", label="null 95th pct")
    plt.xlabel("k")
    plt.ylabel("mean matched cosine")
    plt.title("Cross-sample factor reproducibility (Hungarian matching)")
    plt.legend()
    plt.grid(alpha=.3)
    plt.tight_layout()
    save(out_dir, "fig1_pairwise_matched_cos.svg")

    # ---- Fig 2: shared ecotype count at thresholds (unchanged) ----
    apply_scale(kk("fig2_shared_ecotype_count"))
    plt.figure(figsize=panel_figsize["fig2_shared_ecotype_count"])
    for th in (0.6, 0.7, 0.8):
        n = [next(r["n_ecotypes(>=11samp)"] for r in S["cluster"]
                  if r["k"] == k and abs(r["thresh"] - th) < 1e-6) for k in ks]
        plt.plot(ks, n, "-o", label=f"cos>={th}")
    plt.xlabel("k")
    plt.ylabel("# shared ecotypes (>=11/21 samples)")
    plt.title("Shared ecotypes by threshold (avg-linkage clustering)")
    plt.legend()
    plt.grid(alpha=.3)
    plt.tight_layout()
    save(out_dir, "fig2_shared_ecotype_count.svg")

    # ---- Fig 3: empty factors (unchanged) ----
    apply_scale(kk("fig3_empty_factors"))
    plt.figure(figsize=panel_figsize["fig3_empty_factors"])
    empt = [S["purity"][i]["empty"] for i in range(5)]
    plt.plot(ks, empt, "-o", label="empty factors")
    plt.xlabel("k")
    plt.ylabel("total empty factors (21 samples)")
    plt.title("Empty factors vs k")
    plt.grid(alpha=.3)
    plt.legend()
    plt.tight_layout()
    save(out_dir, "fig3_empty_factors.svg")

    # ---- Fig 4: transferability (unchanged) ----
    apply_scale(kk("fig4_transferability"))
    plt.figure(figsize=panel_figsize["fig4_transferability"])
    diag = [T["results"][str(k)]["diag_mean"] for k in ks]
    cross = [T["results"][str(k)]["cross_mean"] for k in ks]
    plt.plot(ks, diag, "-o", label="within-sample R2")
    plt.plot(ks, cross, "-s", label="cross-sample R2 (A factors on B)")
    plt.xlabel("k")
    plt.ylabel("transfer R2")
    plt.title("Subspace transferability (does A's ecotype space fit B?)")
    plt.legend()
    plt.grid(alpha=.3)
    plt.tight_layout()
    save(out_dir, "fig4_transferability.svg")

    # ---- Fig 5: joint NMF metrics, twin axes (unchanged) ----
    apply_scale(kk("fig5_joint_nmf"))
    fig, ax1 = plt.subplots(figsize=panel_figsize["fig5_joint_nmf"])
    r2j = [J[str(k)]["r2"] for k in ks_j]
    coph = [J[str(k)]["cophenetic"] for k in ks_j]
    ax1.plot(ks_j, r2j, "-o", color="C0", label="R2 (joint)")
    ax1.set_xlabel("k")
    ax1.set_ylabel("R2", color="C0")
    ax1.tick_params(axis="y", labelcolor="C0")
    ax2 = ax1.twinx()
    ax2.plot(ks_j, coph, "-s", color="C3", label="cophenetic")
    ax2.set_ylabel("cophenetic corr", color="C3")
    ax2.tick_params(axis="y", labelcolor="C3")
    plt.title("Joint NMF: R2 & cophenetic")
    fig.tight_layout()
    path = os.path.join(out_dir, "fig5_joint_nmf.svg")
    fig.savefig(path, format="svg")
    plt.close()
    print("  ", "fig5_joint_nmf.svg")

    # ---- Fig 6: joint k=7 composition, two dialects ----
    k7 = JC["7"]
    ct = ["Acinar", "Bcell", "CAF", "CD4T", "CD8T", "DC", "Ductal", "Ductal_tumor",
          "Endocrine", "Endothelial", "ILC", "Macro", "Mast", "Mono", "NK",
          "Schwann", "Stellate", "gdT"]
    M = np.zeros((7, len(ct)))
    for i, row in enumerate(k7):
        for j, c in enumerate(ct):
            M[i, j] = row["composition"].get(c, 0)

    def fig6(draw, figsize, origin):
        apply_scale(k6)
        plt.figure(figsize=figsize)
        im = draw(M)
        plt.colorbar(im, label="composition share")
        if origin == "upper":
            # imshow's default origin='upper' already puts row F0 at the top, and
            # the integer row indices are the pixel centres.
            plt.yticks(range(7), [f"F{i}" for i in range(7)])
        else:
            # pcolormesh's default origin='lower' draws row F0 at the *bottom*, so
            # the naive version is a vertical mirror of the imshow panel.  Mirror
            # the y axis so the rendered top-to-bottom row order is F0..F6, and put
            # the tick labels on the cell centres (0.5 .. 6.5) so they stay aligned
            # with the cells instead of the cell edges.
            ax = plt.gca()
            ax.invert_yaxis()
            ax.set_yticks([i + 0.5 for i in range(7)])
            ax.set_yticklabels([f"F{i}" for i in range(7)])
        plt.xticks(range(18), ct, rotation=90, fontsize=7 * k6)
        plt.title("Joint NMF k=7 factor composition")
        plt.tight_layout()

    fig6(lambda m: plt.imshow(m, aspect="auto", cmap="viridis"),
         panel_figsize["fig6_joint_k7_composition_imshow"], origin="upper")
    save(out_dir, "fig6_joint_k7_composition_imshow.svg")

    fig6(lambda m: plt.pcolormesh(m, cmap="viridis", edgecolors="none"),
         panel_figsize["fig6_joint_k7_composition_pcolormesh"], origin="lower")
    save(out_dir, "fig6_joint_k7_composition_pcolormesh.svg")

    print("panels written to", out_dir)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=None)
    ap.add_argument("--scale-to-cell", action="store_true",
                    help="draw every panel at the size of its S22 grid cell")
    ap.add_argument("--target-width", type=float, default=CANVAS_WIDTH,
                    help="S22 canvas width (px) used to derive each cell size")
    args = ap.parse_args()

    if args.scale_to_cell:
        out_dir = args.out or DEFAULT_SIZED_OUT
        scales = cell_scales(args.target_width)
        print(f"sized mode: target canvas width {args.target_width:g}, "
              f"panels -> {out_dir}")
        for panel in ALL_PANELS:
            info = scales[panel]
            print(f"  {panel:42s} k_p={info['k']:.5f} "
                  f"cell={info['cell_w']:.2f}x{info['cell_h']:.2f} "
                  f"-> {PANEL_FIGSIZE[panel][0] * 72 * info['k']:.2f}"
                  f"x{PANEL_FIGSIZE[panel][1] * 72 * info['k']:.2f} pt")
        out_dir, R, S, J, T, JC = load(out_dir)
        render(out_dir, R, S, J, T, JC, scales)
        return

    out_dir = args.out or DEFAULT_OUT
    out_dir, R, S, J, T, JC = load(out_dir)
    scales = {panel: dict(k=1.0) for panel in ALL_PANELS}
    render(out_dir, R, S, J, T, JC, scales)


if __name__ == "__main__":
    main()
