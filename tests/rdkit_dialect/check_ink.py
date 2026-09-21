#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""Assert a composed PNG actually has ink, instead of trusting a blank render.

The failure this guards against is a fallback route that emits a fully white image
while still exiting 0. We measure the fraction of non-white pixels with PIL and
require it above a floor.

  python check_ink.py composed.png
"""

import sys

import numpy as np
from PIL import Image

INK_FLOOR_PERCENT = 1.0


def main():
    png = sys.argv[1]
    image = Image.open(png).convert("RGB")
    pixels = np.asarray(image)
    non_white = np.any(pixels < 250, axis=2)
    ink = 100.0 * non_white.sum() / non_white.size
    colors = len(np.unique(pixels.reshape(-1, 3), axis=0))
    width, height = image.size
    print(f"{png}: {width}x{height}  ink = {ink:.2f}%  colors = {colors}")
    assert ink > INK_FLOOR_PERCENT, f"blank/near-blank render (ink {ink:.2f}%)"
    print(f"OK: ink {ink:.2f}% > {INK_FLOOR_PERCENT:.1f}%")


if __name__ == "__main__":
    main()
