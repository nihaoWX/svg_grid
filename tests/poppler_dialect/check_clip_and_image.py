#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""V2 #2/#3 render confirmations.

  * clip PNG: the rectified clip box keeps the red inside the box and the white
    outside it; the degenerate trailing `M x y` subpath clips nothing.
  * image PNG: the non-uniformly-scaled bitmap really rasterised into its box
    (a red/blue 2x8 test image stretched to 138x1.15 units).
"""

import sys

import numpy as np
from PIL import Image


def main():
    clip_png, img_png = sys.argv[1], sys.argv[2]

    c = np.asarray(Image.open(clip_png).convert("RGB"))
    inside = tuple(int(v) for v in c[35, 25])
    outside = tuple(int(v) for v in c[70, 80])
    degenerate = tuple(int(v) for v in c[5, 5])
    print(f"clip inside (25,35) = {inside}  (want ~(230,77,77))")
    print(f"clip outside (80,70) = {outside}  (want white)")
    print(f"clip at degenerate `M 5 5` (5,5) = {degenerate}  (want white)")
    assert max(abs(a - b) for a, b in zip(inside, (230, 77, 77))) <= 20
    assert min(outside) >= 245 and min(degenerate) >= 245

    i = np.asarray(Image.open(img_png).convert("RGB"))
    print(f"image PNG {i.shape[1]}x{i.shape[0]}")
    strip = i[149:152, 494:632].reshape(-1, 3)
    uniq = len(np.unique(strip, axis=0))
    mean = strip.mean(axis=0)
    print(f"image box strip: unique colours={uniq} mean=[{mean[0]:.1f} {mean[1]:.1f} {mean[2]:.1f}]")
    assert uniq >= 2, "the scaled bitmap did not render into its box"
    assert mean[1] < mean[0] and mean[1] < mean[2], "strip is not the red/blue test bitmap"

    print("OK: clip + image fixtures render correctly")
    return 0


if __name__ == "__main__":
    sys.exit(main())
