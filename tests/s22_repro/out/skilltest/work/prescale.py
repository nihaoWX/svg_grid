#!/usr/bin/env python3
"""Pre-scale a tagged panel SVG so the composer can yield readable type.

Why this exists
---------------
`svg_grid plot-grid` never rescales `font-size` (protocol rule), and for
matplotlib panels the font sizes live in a CSS `style="..."` attribute, which
the composer does not rewrite at all (the converter warns about this).  The
documented remedies are (a) re-plot the panel at the target cell size or
(b) `--normalize-input-max-side`; neither can touch style-based font sizes.
So we bake the scaling into the panel before composing:

  * `font-size`            x k_font   (typography; k_font may exceed the pure
                                       geometric scale s to keep paper pt >= 5)
  * `stroke-dasharray` /   x s_geom   (these follow the geometry, not the type)
    `stroke-dashoffset`
  * explicit `stroke-width="1"` is injected where a polyline/polygon has none,
    so the composer's xy-strategy can rescale it (default 1 is otherwise left
    untouched because it is not written out).

Geometry (coordinates) is deliberately left alone: the composer does that.
"""
import re
import sys


def scale_len(prefix_pat, text, factor):
    def rep(m):
        nums = re.findall(r"-?\d*\.?\d+", m.group(0))
        out = m.group(0)
        for n in nums:
            out = out.replace(n, f"{float(n) * factor:.4f}", 1)
        return out
    return re.sub(prefix_pat, rep, text)


def main():
    src, dst, k_font, s_geom = sys.argv[1], sys.argv[2], float(sys.argv[3]), float(sys.argv[4])
    s = open(src).read()

    # 1. font-size (in style attributes; there are no font-size attributes here)
    s = re.sub(r"font-size:\s*(-?\d*\.?\d+)px",
               lambda m: f"font-size: {float(m.group(1)) * k_font:.4f}px", s)

    # 2. dash pattern lengths follow the geometry
    s = scale_len(r"stroke-dasharray:\s*[-\d.,\s]+?(?=;|\")", s, s_geom)
    s = scale_len(r"stroke-dashoffset:\s*[-\d.,\s]+?(?=;|\")", s, s_geom)

    # 3. make the implicit stroke-width="1" explicit so it gets rescaled
    s = re.sub(r"<(polyline|polygon)\b(?![^>]*stroke-width)([^>]*?)(/?)>",
               lambda m: f"<{m.group(1)} stroke-width=\"1\"{m.group(2)}{m.group(3)}>", s)

    open(dst, "w").write(s)
    print(f"{src} -> {dst}  k_font={k_font} s_geom={s_geom}")


if __name__ == "__main__":
    main()
