"""Shared helpers for the matplotlib fixtures.

Regenerate a group with (run from this directory):

    ~/.venv/bin/python gen_<group>.py

Every script is deterministic: fixed seed, fixed 3 x 2 in canvas, no network, and
``svg.fonttype = 'none'`` so glyphs stay ``<text>`` instead of becoming paths.
"""

import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402,F401  (re-exported for the fixture scripts)

HERE = os.path.dirname(os.path.abspath(__file__))

matplotlib.rcParams.update({
    "svg.fonttype": "none",          # keep text as <text>, not glyph outlines
    "svg.hashsalt": "svg-grid-fixtures",  # stable element ids (idempotent-ish)
    "figure.figsize": (3.0, 2.0),
    "figure.dpi": 100,
    # one explicit family keeps each <text> style attribute short (the default
    # fallback list alone is ~300 bytes per label, which blows the 60 KB budget)
    "font.family": "sans-serif",
    "font.sans-serif": ["DejaVu Sans"],
    "font.size": 7,
    "axes.titlesize": 7,
    "axes.labelsize": 7,
    "xtick.labelsize": 6,
    "ytick.labelsize": 6,
    "legend.fontsize": 5.5,
    "figure.titlesize": 8,
    "axes.grid": False,
    "path.simplify": True,
    "path.simplify_threshold": 0.5,
})


def finish(fig, name, directory=None):
    """Save ``fig`` as <name>.svg at exactly 3 x 2 in and report the size."""
    fig.set_size_inches(3, 2, forward=True)
    path = os.path.join(directory or HERE, name + ".svg")
    fig.savefig(path, format="svg", metadata={"Date": None})
    plt.close(fig)
    print(f"wrote {name + '.svg':28s} {os.path.getsize(path) / 1024:6.1f} KB")
