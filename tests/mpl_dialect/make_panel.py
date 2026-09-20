#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""Render the matplotlib *dialect* panel used by V1 (geometry-equivalence proof).

The panel is deliberately loaded with everything the converter must now accept:

  * ``ax.set_xscale('log')`` + ``ax.set_yscale('log')`` — the default log tick
    formatter writes ``$\mathdefault{10^{n}}$``, i.e. one absolute
    ``<tspan x y>`` per run inside a ``<g transform="translate(...)">``
    (matplotlib mathtext);
  * a ``LogNorm`` colour bar (log colour scale) — more mathtext ticks;
  * a title with *explicit* mathtext (``$\rho$`` and ``$\mathdefault{...}$``);
  * a multi-line ``ax.text(..., "a\nb\nc")`` — matplotlib writes each line with a
    bare ``transform="translate(dx dy)"`` on the ``<text>`` itself.

``svg.fonttype='none'`` keeps the text as ``<text>`` (do not outline it), so the
whole dialect is exercised.

The axis *labels* are plain text on purpose: a rotated **mathtext** label is
emitted as ``<g transform="translate(...) rotate(-90)"><text><tspan ...>`` — a
rotating ancestor — which the converter still (correctly) rejects. Plain labels
stay ``<text x y transform="rotate(-90 x y)">`` (a leaf rotate), which is
supported. The rotated-mathtext fail-closed case is asserted separately.

Usage:
    micromamba run -n python-bio python make_panel.py [OUT.svg]
"""

import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_OUT = os.path.join(HERE, "panel.svg")


def build(path):
    matplotlib.rcParams["svg.fonttype"] = "none"   # keep <text>, do not outline

    rng = np.random.default_rng(7)
    n = 60
    x = 10.0 ** rng.uniform(1.0, 5.0, size=(n, n))
    y = 10.0 ** rng.uniform(-5.0, -1.0, size=(n, n))
    # a wide dynamic range for the LogNorm colour bar
    c = 10.0 ** rng.uniform(-6.0, 2.0, size=(n, n))

    fig, ax = plt.subplots(figsize=(6.4, 4.8))
    sc = ax.scatter(x.ravel(), y.ravel(), c=c.ravel(), s=18,
                    cmap="viridis", norm=LogNorm(vmin=1e-6, vmax=1e2))
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("x  (cm^-3)")          # plain: leaf-rotate-0 <text>
    ax.set_ylabel("y  (s^-1)")           # plain: leaf rotate(-90) <text>
    # explicit mathtext in the title: $\rho$ and a $\mathdefault{...}$ token
    ax.set_title(r"density map, $\rho$, $\mathdefault{10^{3}}$")
    # a *multi-line* plain text run: matplotlib emits one bare `translate` per line
    ax.text(0.5, 0.5, "a\nb\nc", transform=ax.transAxes, color="red")
    cb = fig.colorbar(sc, ax=ax, label="density")
    cb.ax.set_yscale("log")
    fig.tight_layout()
    fig.savefig(path, format="svg")
    plt.close(fig)
    return path


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_OUT
    build(out)
    print(out)


if __name__ == "__main__":
    main()
