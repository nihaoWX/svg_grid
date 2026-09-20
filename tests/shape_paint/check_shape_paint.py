#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""Pixel-level regression for the two `<path>` paint bugs fixed in
`tools/svg_grid/convert/src/translate/shapes.rs`.

Every fixture is composed to a 100x100 PNG at scale 1, so canvas coordinates are
pixel coordinates.

  1. `fixture_bridge_stroke.svg` — two *disjoint* closed rectangles in ONE
     `<path>` carrying both a fill and a stroke. The fill region is their
     concatenation; if that `<polygon>` kept the source `stroke`, the bridge
     segment between the rectangles (through the gap) would be stroked. Probes: a
     point on the bridge (45,45) must be background, a point inside the first
     rectangle (20,20) must be the fill red.
  2. `fixture_open_subpaths.svg` — two *open* overlapping contours in one filled
     `<path>`. Concatenating them without closing each subpath makes the bridge
     edges real edges that erase the fill near (31,29)/(29,31). The source is
     rendered independently with cairosvg and the probes must agree.

Usage:
    python check_shape_paint.py
"""

import sys

import cairosvg
from PIL import Image

BG_MIN = 245          # a channel min above this is "background (white)"
FILL_TOL = 40         # per-channel tolerance for "is the fill colour"


def rgb(path, point):
    return tuple(int(v) for v in Image.open(path).convert("RGB").getpixel(point))


def is_background(value):
    return min(value) >= BG_MIN


def close_to(value, other, tol=FILL_TOL):
    return max(abs(a - b) for a, b in zip(value, other)) <= tol


def check_defect_1():
    bridge = rgb("fixture_bridge_stroke.comp.png", (45, 45))
    fill = rgb("fixture_bridge_stroke.comp.png", (20, 20))
    print(f"defect1 bridge (45,45) RGB = {bridge}   (must be background)")
    print(f"defect1 fill   (20,20) RGB = {fill}   (must be red)")
    assert is_background(bridge), f"bridge pixel {bridge} is not background"
    assert close_to(fill, (255, 0, 0)), f"fill pixel {fill} is not red"


def check_defect_2():
    # Render the same source independently. Use a white background: cairosvg's
    # default output is transparent, and the composer paints an opaque white
    # background, so transparent source pixels must be compared as white — not as
    # the (0,0,0) that a naive RGBA->RGB conversion produces.
    cairosvg.svg2png(
        url="fixture_open_subpaths.svg",
        write_to="fixture_open_subpaths.src.png",
        output_width=100,
        output_height=100,
        background_color="white",
    )
    ok = True
    for point in [(50, 50), (31, 29), (29, 31)]:
        converted = rgb("fixture_open_subpaths.comp.png", point)
        source = rgb("fixture_open_subpaths.src.png", point)
        print(f"defect2 {point}: converted={converted} source={source}")
        if not close_to(converted, source):
            ok = False
    assert ok, "the converted fill does not match the independently rendered source"


def main():
    check_defect_1()
    check_defect_2()
    print("OK: no bridge is stroked; the open-subpath fill matches the source")
    return 0


if __name__ == "__main__":
    sys.exit(main())
