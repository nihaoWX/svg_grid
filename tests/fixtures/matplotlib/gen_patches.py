"""Patches: pie (Wedge arcs) and explicit Rectangle/Circle/Polygon/Ellipse/Path."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish
from matplotlib.patches import (Arc, Circle, Ellipse, PathPatch, Polygon,
                                Rectangle, Wedge)
from matplotlib.path import Path

# --- pie: wedges with exploded slice + percentage labels ----------------
fig, ax = plt.subplots()
ax.pie([35, 25, 20, 12, 8], labels=["a", "b", "c", "d", "e"], autopct="%1.0f%%",
       startangle=90, explode=(0.08, 0, 0, 0, 0), pctdistance=0.65,
       wedgeprops=dict(linewidth=0.4, edgecolor="white"),
       textprops=dict(fontsize=5))
ax.set_title("pie")
fig.tight_layout(pad=0.3)
finish(fig, "pie")

# --- patches: assorted shapes, a Bezier path and a path with a hole -----
fig, ax = plt.subplots()
ax.add_patch(Rectangle((0.05, 0.05), 0.35, 0.3, facecolor="#c7e9c0",
                       edgecolor="black", linewidth=0.4, alpha=0.9))
ax.add_patch(Circle((0.65, 0.25), 0.15, facecolor="#fdae6b",
                    edgecolor="black", linewidth=0.4))
ax.add_patch(Polygon([[0.05, 0.5], [0.3, 0.9], [0.5, 0.55]], closed=True,
                     facecolor="#9ecae1", edgecolor="black", linewidth=0.4))
ax.add_patch(Ellipse((0.75, 0.7), 0.3, 0.18, angle=25, facecolor="#bcbddc",
                     edgecolor="black", linewidth=0.4))
ax.add_patch(Wedge((0.45, 0.7), 0.2, 200, 340, facecolor="#fdd0a2",
                   edgecolor="black", linewidth=0.4))
ax.add_patch(Arc((0.5, 0.5), 0.9, 0.9, theta1=10, theta2=140, linewidth=0.4,
                 linestyle="--"))

# a quadratic/cubic Bezier path
verts = [(0.05, 0.35), (0.2, 0.5), (0.35, 0.2), (0.5, 0.35)]
codes = [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4]
ax.add_patch(PathPatch(Path(verts, codes), fill=False, linewidth=0.6,
                       edgecolor="darkred"))

# a closed path with an inner hole (two subpaths, nonzero fill rule)
outer = [(0.6, 0.4), (0.95, 0.4), (0.95, 0.55), (0.6, 0.55), (0.6, 0.4)]
inner = [(0.7, 0.45), (0.85, 0.45), (0.85, 0.5), (0.7, 0.5), (0.7, 0.45)]
hole_codes = [Path.MOVETO] + [Path.LINETO] * 3 + [Path.CLOSEPOLY]
hole_verts = outer + inner
hole = Path(hole_verts, hole_codes + hole_codes)
ax.add_patch(PathPatch(hole, facecolor="#a6cee3", edgecolor="black",
                       linewidth=0.4, alpha=0.8))
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
ax.set_xticks([])
ax.set_yticks([])
ax.set_aspect("equal")
fig.tight_layout(pad=0.3)
finish(fig, "patches")
