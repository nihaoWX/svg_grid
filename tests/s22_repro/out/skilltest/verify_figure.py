#!/usr/bin/env python3
"""Programmatic delivery checks for Figure.png (see references/05_checklist.md)."""
import re
import sys

import numpy as np
from PIL import Image
from scipy import ndimage

PNG = sys.argv[1] if len(sys.argv) > 1 else "Figure.png"
SVG = PNG.rsplit(".", 1)[0] + ".svg"

im = Image.open(PNG)
a = np.asarray(im.convert("L"))
H, W = a.shape
print(f"[size] {W} x {H} px   dpi={im.info.get('dpi')}   ink={100*(a<250).mean():.2f}%  "
      f"paper={W/600:.2f} x {H/600:.2f} in")

# --- dead white bands (200 px) -------------------------------------------------
bands = [float((a[i:i+200] < 245).mean()) for i in range(0, H, 200)]
zero = [i*200 for i, v in enumerate(bands) if v == 0.0]
print(f"[bands] 200px row-band ink: min={min(bands):.4f} @y={bands.index(min(bands))*200}px"
      f"  empty bands={zero}  (last band height={H%200 or 200}px)")
colbands = [float((a[:, i:i+200] < 245).mean()) for i in range(0, W, 200)]
print(f"[bands] 200px col-band ink: min={min(colbands):.4f} @x={colbands.index(min(colbands))*200}px"
      f"  empty={[i*200 for i,v in enumerate(colbands) if v==0.0]}")

ys, xs = np.where(a < 250)
print(f"[clip]  content bbox x[{xs.min()},{xs.max()}] y[{ys.min()},{ys.max()}] inside canvas: "
      f"{xs.min()>=0 and xs.max()<W and ys.min()>=0 and ys.max()<H}")

# --- panel boxes / cells (from the composed SVG) -------------------------------
svg = open(SVG).read()
boxes = [(float(m.group(1)), float(m.group(2)), float(m.group(3)), float(m.group(4)))
         for m in re.finditer(r'<rect data-panel-box="main" x="([-\d.]+)" y="([-\d.]+)" '
                              r'width="([-\d.]+)" height="([-\d.]+)"', svg)]
print(f"[boxes] {len(boxes)} panel-box rects in the product")
cells = [b for b in boxes if abs(b[2]-4961) > 1]           # drop the canvas box
# expected cell geometry (must mirror build_figure.sh)
M, Wc = 170, 4961
avail = Wc - 2 * M
cw1 = avail / 2
h1 = cw1 / 1.4
asum = 1.4 + 1.4 + 4 / 3
cw2c, cw2e, h2 = avail * 1.4 / asum, avail * (4 / 3) / asum, avail / asum
cw3, h3 = avail, avail / 2.25
cwx = [(cw1, 1.4, h1), (cw1, 1.4, h1), (cw2c, 1.4, h2),
       (cw2c, 1.4, h2), (cw2e, 4 / 3, h2), (cw3, 2.25, h3)]
src = [(504, 360), (504, 360), (504, 360), (504, 360), (460.8, 345.6), (648, 288)]
labels = "ABCDEF"
print("[cells] label  cell_x  cell_y   cell_w    cell_h   aspect   src_aspect  sx      sy     sx/sy")
for i, (x, y, w, h) in enumerate(cells):
    name = labels[i] if i < len(labels) else "?"
    # cell height = row height, row height = distance to next row's y or from geometry
    cw = cwx[i][0]
    ch = cwx[i][2]
    sx = cw / src[i][0]
    sy = ch / src[i][1]
    print(f"        {name:<4} {x:7.1f} {y:8.1f} {cw:9.1f} {ch:9.1f}  {cw/ch:6.4f}  "
          f"{src[i][0]/src[i][1]:8.4f}  {sx:6.4f} {sy:6.4f}  {sx/sy:6.5f}")

# --- per-panel distortion: ink-bbox aspect in the product vs at scale 1 ---------
print("[distortion] ink bbox inside each cell (a uniform scale keeps the aspect) + inner white")
for i, (x, y, w, h) in enumerate(cells):
    cw, ch = cwx[i][0], cwx[i][2]
    x0, y0 = int(round(x)), int(round(y))
    sub = a[y0:y0 + int(round(ch)), x0:x0 + int(round(cw))]
    m = sub < 250
    if m.sum() == 0:
        print(f"        {labels[i]}: EMPTY cell")
        continue
    yy, xx = np.where(m)
    bw, bh = xx.max() - xx.min() + 1, yy.max() - yy.min() + 1
    edge = (xx.min() == 0) or (yy.min() == 0) or (xx.max() == sub.shape[1] - 1) or (yy.max() == sub.shape[0] - 1)
    print(f"        {labels[i]}: ink bbox {bw}x{bh} aspect={bw/bh:.4f} (src {src[i][0]/src[i][1]:.4f})"
          f"  white t={yy.min()}/{(ch-yy.max()):.0f}px  touches-cell-edge={edge}")

# --- panel letters -------------------------------------------------------------
print("[labels] letter ink (component >10px in the 260x150 px corner above each cell)")
res = []
for i, (x, y, w, h) in enumerate(cells):
    x0, y0 = int(round(x)), int(round(y))
    reg = a[max(0, y0-150):y0, max(0, x0-30):x0+200]
    lab, n = ndimage.label(reg < 128)
    best = None
    for o in ndimage.find_objects(lab):
        hh, ww = o[0].stop - o[0].start, o[1].stop - o[1].start
        if ww > 20 and hh > 20 and (best is None or ww*hh > best[2]*best[3]):
            best = (ww, hh, ww, hh, float((reg[o] < 128).mean()), o)
    if best:
        ww, hh, _, _, fill, o = best
        print(f"        {labels[i]}: ink {ww}x{hh} px = {100*ww/W:.2f}% of W  "
              f"fill={fill:.3f}  gap_to_cell_top={y0-(max(0,y0-150)+o[0].stop)}px")
        res.append(ww)
print(f"        letter ink widths as % of W: {[round(100*w/W,2) for w in res]}")

# --- svg sanity -----------------------------------------------------------------
hdr = re.search(r'<svg width="([\d.]+)" height="([\d.]+)" viewBox="0 0 ([\d.]+) ([\d.]+)"', svg)
print(f"[svg]   header w={hdr.group(1)} h={hdr.group(2)} viewBox={hdr.group(3)}x{hdr.group(4)} "
      f"consistent={hdr.group(1)==hdr.group(3) and hdr.group(2)==hdr.group(4)}")
print("[svg]   <path>={} <use>={} '<g transform'={} <image>={}".format(
    len(re.findall(r"<path[ >]", svg)), len(re.findall(r"<use[ >]", svg)),
    len(re.findall(r"<g[^>]*transform=", svg)), len(re.findall(r"<image[ >]", svg))))
ids = re.findall(r'\sid="([^"]+)"', svg)
dup = {i for i in ids if ids.count(i) > 1}
print(f"[svg]   {len(ids)} ids, {len(dup)} duplicated: {sorted(dup)[:5]}")
print(f"[svg]   font-size values: {sorted(set(re.findall(r'font-size:\s*([0-9.]+)px', svg)), key=float)[:6]} ...")
print(f"[svg]   label font-family/weight: {sorted(set(re.findall(r'<text data-svg-grid-label[^>]*font-family=\"([^\"]+)\"', svg)))} "
      f"font-weight attrs: {sorted(set(re.findall(r'data-svg-grid-label[^>]*font-weight=\"([^\"]+)\"', svg)))}")
