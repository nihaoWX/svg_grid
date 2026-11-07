#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""compose_fig2 的共享配置与基础工具（路径 / 版面几何 / 块规格 / 子进程封装）。

这里搬的是原始 `compose_fig2.py` 顶部的常量与"geometry"那一节；入口
`compose_fig2.py` 与其余辅助模块（`fig2_compose` / `fig2_verify`）都从这里取。
"""

import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))          # tools/svg_grid
CONVERT = os.path.join(ROOT, "convert/target/release/svg_grid_convert")
GRID = os.path.join(ROOT, "target/release/svg_grid")

OUT = os.path.join(HERE, "out")
PANELS = os.path.join(OUT, "panels")
TAGGED = os.path.join(OUT, "tagged")
BLOCKS_DIR = os.path.join(OUT, "blocks")

ORIGINAL = ("/home/nihao/bioagentforge/research/白藜芦醇胰腺癌/steps/08_手稿/"
            "Figure/main/Figure 2.png")

SERIF = "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"
SERIF_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"

W = 4961
TARGETS = ["CYP1B1", "GC", "KIF11", "MMP9", "SERPINA1", "HSD11B1", "ALB", "BACE1", "GSTA1"]
LIG_LETTERS = ["A", "B", "C", "D"]

LETTER_GAP = 22.0


def run(cmd, label):
    result = subprocess.run(cmd, capture_output=True, text=True)
    sys.stdout.write(f"\n$ {' '.join(cmd)}\n")
    if result.stdout.strip():
        sys.stdout.write("  [stdout] " + result.stdout.strip() + "\n")
    if result.stderr.strip():
        sys.stdout.write("  [stderr] " + result.stderr.strip() + "\n")
    else:
        sys.stdout.write("  [stderr] (empty)\n")
    if result.returncode != 0:
        raise SystemExit(f"{label}: exit code {result.returncode}")
    return result


# --------------------------------------------------------------------------- #
# geometry (mirrors make_Figure2_v6.py; make_panels.py owns the same numbers)
# --------------------------------------------------------------------------- #
M_L = M_R = 150
M_T = M_B = 80
UW = W - M_L - M_R
LETTER_H, GAP = 130, 34
C_GAP, C_CELL, C_HDR = 19, 1541, 56
H_C = 3 * (C_CELL + C_HDR) + 2 * C_GAP              # 4829
B_GAP, B_CELL = 39, 1136
H_B = 760
UH = 6856
H_A = UH - (3 * LETTER_H + 2 * GAP) - H_B - H_C     # 809

BLOCKS = [
    dict(name="A", inputs=["A_heatmap.svg"], height=M_T + LETTER_H + H_A + GAP,
         margin=(M_T + LETTER_H, M_R, GAP, M_L), ncol=1, gap=0, labels="A",
         canvas=[[UW, H_A]]),
    dict(name="B", inputs=[f"B_{L}.svg" for L in LIG_LETTERS], height=LETTER_H + H_B + GAP,
         margin=(LETTER_H, M_R, GAP, M_L), ncol=4, gap=B_GAP, labels="B,,,",
         canvas=[[B_CELL, H_B]] * 4),
    dict(name="C", inputs=[f"C_{t}.svg" for t in TARGETS], height=LETTER_H + H_C + M_B,
         margin=(LETTER_H, M_R, M_B, M_L), ncol=3, gap=C_GAP,
         labels="C" + "," * 8, canvas=[[C_CELL, C_CELL + C_HDR]] * 9),
]


# --------------------------------------------------------------------------- #
def svg_canvas(path):
    with open(path, encoding="utf-8") as handle:
        text = handle.read(4000)
    match = re.search(r'viewBox="([^"]+)"', text)
    return tuple(float(v) for v in match.group(1).split()[2:])
