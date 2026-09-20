#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""Regenerate `real_poppler.svg`: a genuine `pdftocairo -svg` product that
contains a single large `O` glyph.

The glyph is emitted by poppler in exactly the dialect the converter must accept:
a glyph outline lives in `<defs><g id="glyph-0-0"><path .../></g></defs>` and is
placed with `<use xlink:href="#glyph-0-0" x y/>`; the fill comes from an ancestor
`<g fill="rgb(...)">`; the `O`'s hole is two subpaths in the one `<path>`.

Usage:
    micromamba run -n python-bio python make_poppler.py [real_poppler.svg]
"""

import os
import subprocess
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_OUT = os.path.join(HERE, "real_poppler.svg")


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_OUT
    pdf = out.replace(".svg", ".pdf")

    fig = plt.figure(figsize=(4, 3))
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_axis_off()
    ax.text(0.5, 0.5, "O", fontsize=200, family="DejaVu Sans",
            ha="center", va="center")
    fig.savefig(pdf)
    plt.close(fig)

    subprocess.run(["pdftocairo", "-svg", pdf, out], check=True)
    os.remove(pdf)
    print(out)


if __name__ == "__main__":
    main()
