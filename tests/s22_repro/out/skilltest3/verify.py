#!/usr/bin/env python
"""Programmatic checks for the assembled Figure S (general-figure-guide 02a checklist).
Run:  micromamba run -n python-bio python verify.py
"""
import re, os
import numpy as np
from PIL import Image

BASE = os.path.dirname(os.path.abspath(__file__))
MARGIN = 24.0
W = 4961.0

def dims(p):
    h = open(p).read(4000)
    m = re.search(r'<svg[^>]*\bwidth="([0-9.]+)"\s+height="([0-9.]+)"', h)
    return float(m.group(1)), float(m.group(2))

def boxes(p):
    return [tuple(float(g) for g in m.groups()) for m in re.finditer(
        r'<rect data-panel-box="main" x="([0-9.eE+-]+)" y="([0-9.eE+-]+)" '
        r'width="([0-9.eE+-]+)" height="([0-9.eE+-]+)"', open(p).read())]

avail = W - 2*MARGIN
print("canvas:", dims(f"{BASE}/Figure.svg"))

# per-row sx/sy  (panel box sits at the cell origin; spacing == cell width)
for r, n in ((1, 2), (2, 3), (3, 1)):
    rw, rh = dims(f"{BASE}/rows/row{r}.svg")
    inner = sorted(boxes(f"{BASE}/rows/row{r}.svg")[1:], key=lambda b: b[0])
    cellh = rh - 1.05*134.081 - MARGIN
    for i, (x, y, w, h) in enumerate(inner):
        cw = (inner[i+1][0]-x) if i < len(inner)-1 else (MARGIN+avail-x)
        print(f"  row{r} cell{i}: sx={cw/w:.5f} sy={cellh/h:.5f}  (sx==sy: {abs(cw/w-cellh/h)<1e-3})")

s_o = avail/W
print(f"  outer s = {s_o:.5f}")

# residual foreign constructs must be gone
txt = open(f"{BASE}/Figure.svg").read()
for pat in ("<path", "<use", "<g transform", "<symbol"):
    print(f"  residual '{pat}': {txt.count(pat)}")

# letters
for m in re.finditer(r'<text data-svg-grid-label="\d"[^>]*x="([0-9.]+)" y="([0-9.]+)" '
                     r'font-family="([^"]*)" font-size="([0-9.]+)"[^>]*>([A-F])</text>', txt):
    x, y, ff, fs, g = m.groups()
    print(f"  letter {g}: family={ff} size={float(fs)/W*100:.2f}%W ({float(fs)/600*72:.2f}pt) "
          f"font-weight attr present: {'font-weight' in m.group(0)}")

# raster: dpi, ink, dead bands
im = Image.open(f"{BASE}/Figure.png")
a = np.asarray(im.convert("L")); ink = a < 250
print(f"  PNG {im.size} dpi={im.info.get('dpi')} ink={ink.mean()*100:.2f}%")
def worst(mask):
    best = 0; s = None
    for i, v in enumerate(mask):
        if v and s is None: s = i
        elif not v and s is not None: best = max(best, i-s); s = None
    return max(best, (len(mask)-s) if s is not None else 0)
print(f"  longest all-white row band = {worst((ink.mean(1)==0))} px, "
      f"col band = {worst((ink.mean(0)==0))} px  (must be < 200)")
