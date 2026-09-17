#!/usr/bin/env python3
"""Trim the dead bottom margin off a composed row block.

Each row is composed as its own SVG so that it can carry per-panel letters.
That leaves a full plot-margin of empty space below the row's panels, which is
pure white inside the finished figure (and, for the last row, along the bottom
edge).  The row block is re-composed at cell size, so no coordinates move: only
the canvas height shrinks.

Both the `width/height` attributes AND the `viewBox` height must change
together - otherwise a renderer fits the (now taller) viewBox into the shorter
viewport and silently squashes the whole row vertically.
"""
import re
import sys

path, new_h = sys.argv[1], float(sys.argv[2])
s = open(path).read()

m = re.search(r'<svg width="([\d.]+)" height="([\d.]+)" viewBox="0 0 ([\d.]+) ([\d.]+)"', s)
assert m, "unexpected svg header"
W, old_h, vb_w, vb_h = m.group(1), m.group(2), m.group(3), m.group(4)
assert W == vb_w and old_h == vb_h, "canvas attrs and viewBox disagree before trimming"
assert new_h < float(old_h), "trim must shrink"

new = f"{new_h:.3f}"
s = s[:m.start()] + f'<svg width="{W}" height="{new}" viewBox="0 0 {W} {new}"' + s[m.end():]
s = s.replace(f'width="{W}" height="{old_h}"', f'width="{W}" height="{new}"')
open(path, "w").write(s)

n = len(re.findall(rf'height="{re.escape(new)}"', s))
print(f"{path}: height {old_h} -> {new} ({n} height attributes rewritten)")
