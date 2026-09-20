#!/usr/bin/env python
# -*- coding: utf-8 -*-
r"""V1: prove `svg_grid_convert` is a **geometry-preserving** rewrite of the
matplotlib dialect — purely mechanically, without looking at a rendering.

For every text run (the `<text>` anchor *and* every `<tspan>` anchor, in document
order) we compute its absolute canvas position twice:

  * from the **original** panel SVG, honouring SVG text semantics: the `<text>`'s
    own `transform` and every ancestor `<g transform>` are composed, and a
    `<tspan>` `x`/`y` is an *absolute* position in the text's user space that
    overrides the running text position **per axis** (a missing axis continues the
    current text position, not `0`) — so the position is
    ``M_ancestor · M_text · (x, y)``;
  * from the **converted** SVG, where all ancestor transforms are gone and each
    `<text>`/`<tspan>` now carries explicit absolute `x`/`y` (only a leaf
    `rotate(a cx cy)` may remain, whose fixed point is its own centre).

The two lists must have the *same length, same element kinds and pairwise
positions within 1e-3 canvas units* — that is the machine-checkable statement of
"the conversion does not move any text".

Usage:
    python check_geometry.py ORIGINAL.svg CONVERTED.svg
"""

import sys
import xml.etree.ElementTree as ET

# transforms that establish a new coordinate system we must not descend into for
# *rendered* text (their content is either unreferenced or non-textual).
SKIP = {"defs", "clipPath", "symbol", "style", "mask", "pattern",
        "linearGradient", "radialGradient", "metadata"}


def local(tag):
    return tag.split("}")[-1] if "}" in tag else tag


# --- a minimal 2x3 affine matrix ------------------------------------------- #
class Mat:
    __slots__ = ("a", "b", "c", "d", "e", "f")

    def __init__(self, a=1.0, b=0.0, c=0.0, d=1.0, e=0.0, f=0.0):
        self.a, self.b, self.c, self.d, self.e, self.f = a, b, c, d, e, f

    def mul(self, o):
        return Mat(
            self.a * o.a + self.c * o.b,
            self.b * o.a + self.d * o.b,
            self.a * o.c + self.c * o.d,
            self.b * o.c + self.d * o.d,
            self.a * o.e + self.c * o.f + self.e,
            self.b * o.e + self.d * o.f + self.f,
        )

    def apply(self, x, y):
        return (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)


def parse_transform(value):
    import math
    import re

    m = Mat()
    for name, args in re.findall(r"([a-zA-Z]+)\s*\(([^)]*)\)", value):
        nums = [float(t) for t in re.split(r"[,\s]+", args.strip()) if t]
        n = name.lower()
        if n == "translate":
            tx = nums[0]
            ty = nums[1] if len(nums) > 1 else 0.0
            m = m.mul(Mat(1, 0, 0, 1, tx, ty))
        elif n == "scale":
            sx = nums[0]
            sy = nums[1] if len(nums) > 1 else nums[0]
            m = m.mul(Mat(sx, 0, 0, sy, 0, 0))
        elif n == "rotate":
            a = math.radians(nums[0])
            ca, sa = math.cos(a), math.sin(a)
            r = Mat(ca, sa, -sa, ca, 0, 0)
            if len(nums) == 3:
                cx, cy = nums[1], nums[2]
                r = Mat(1, 0, 0, 1, cx, cy).mul(r).mul(Mat(1, 0, 0, 1, -cx, -cy))
            m = m.mul(r)
        elif n == "matrix":
            m = m.mul(Mat(*nums[:6]))
        elif n == "skewx":
            m = m.mul(Mat(1, 0, math.tan(math.radians(nums[0])), 1, 0, 0))
        elif n == "skewy":
            m = m.mul(Mat(1, math.tan(math.radians(nums[0])), 0, 1, 0, 0))
    return m


def num(value, default=0.0):
    return float(value) if value is not None else default


def runs(root):
    """[(kind, x, y)] for every text/tspan run, in document order."""
    out = []

    def visit(el, acc):
        name = local(el.tag)
        if name in SKIP:
            return
        mat = parse_transform(el.get("transform", ""))
        world = acc.mul(mat)
        if name == "text":
            x, y = num(el.get("x")), num(el.get("y"))
            out.append(("text", *world.apply(x, y)))
            # The running text position a <tspan> continues on the axis it does
            # not re-specify: `x`/`y` are absolute and override their own axis
            # only, so the other axis stays at the current text position.
            current = (x, y)
            for child in el:
                current = walk_children(child, world, current)
        else:
            for child in el:
                visit(child, world)

    def walk_children(el, world, current):
        cur_x, cur_y = current
        if local(el.tag) == "tspan":
            x = el.get("x")
            y = el.get("y")
            if x is not None or y is not None:
                # A missing axis inherits the current text position (`num`'s
                # default), NOT 0 — `0` would misplace every single-axis <tspan>.
                point = (num(x, cur_x), num(y, cur_y))
                out.append(("tspan", *world.apply(*point)))
                cur_x, cur_y = point
        for child in el:
            cur_x, cur_y = walk_children(child, world, (cur_x, cur_y))
        return (cur_x, cur_y)

    visit(root, Mat())
    return out


def main():
    original, converted = sys.argv[1], sys.argv[2]
    a = runs(ET.parse(original).getroot())
    b = runs(ET.parse(converted).getroot())

    print(f"original : {len(a)} runs "
          f"({sum(1 for k, _, _ in a if k == 'text')} text + "
          f"{sum(1 for k, _, _ in a if k == 'tspan')} tspan)")
    print(f"converted: {len(b)} runs "
          f"({sum(1 for k, _, _ in b if k == 'text')} text + "
          f"{sum(1 for k, _, _ in b if k == 'tspan')} tspan)")

    assert len(a) == len(b), f"run count differs: {len(a)} vs {len(b)}"

    worst = 0.0
    worst_i = -1
    mismatches = []
    for i, (ka, xa, ya) in enumerate(a):
        kb, xb, yb = b[i]
        if ka != kb:
            mismatches.append((i, ka, kb, "kind"))
            continue
        d = max(abs(xa - xb), abs(ya - yb))
        if d > worst:
            worst, worst_i = d, i
        if d >= 1e-3:
            mismatches.append((i, (xa, ya), (xb, yb), f"delta={d:.6g}"))

    print(f"max |Δ| over all {len(a)} runs = {worst:.6g} canvas units "
          f"(run #{worst_i}: {a[worst_i][0]})")
    if mismatches:
        print(f"FAIL: {len(mismatches)} mismatch(es):")
        for m in mismatches[:20]:
            print("   ", m)
        return 1

    print(f"OK: all {len(a)} runs align within 1e-3")
    return 0


if __name__ == "__main__":
    sys.exit(main())
