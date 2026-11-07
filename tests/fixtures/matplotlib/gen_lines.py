"""Line-ish plots: multi-line with markers, scatter with colorbar, step + stem."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(0)

# --- lines_markers: 3 lines, markers, dashes, legend ------------------------
fig, ax = plt.subplots()
x = np.linspace(0, 10, 11)
for i, (mk, ls) in enumerate([("o", "-"), ("s", "--"), ("^", ":")]):
    ax.plot(x, np.sin(x + i) + i, marker=mk, linestyle=ls, linewidth=0.8,
            markersize=2.5, label=f"series {i}")
ax.set_xlabel("x")
ax.set_ylabel("y")
ax.legend(loc="upper left", frameon=False, handlelength=1.5, labelspacing=0.2)
fig.tight_layout(pad=0.3)
finish(fig, "lines_markers")

# --- scatter_colorbar: size + colour mapping with a continuous colorbar -----
fig, ax = plt.subplots()
n = 30
sx, sy = rng.normal(size=n), rng.normal(size=n)
sc = ax.scatter(sx, sy, s=np.abs(sx) * 40 + 5, c=sy, cmap="viridis",
                alpha=0.8, linewidths=0.2, edgecolors="black")
fig.colorbar(sc, ax=ax, shrink=0.8, pad=0.02)
ax.set_xlabel("x")
ax.set_ylabel("y")
fig.tight_layout(pad=0.3)
finish(fig, "scatter_colorbar")

# --- step_stem: step() on one axis, stem() on the other --------------------
fig, axes = plt.subplots(1, 2)
t = np.arange(8)
axes[0].step(t, np.array([1, 3, 2, 5, 4, 6, 5, 7]), where="mid", linewidth=0.8)
axes[0].set_title("step")
axes[1].stem(t, np.array([2, 1, 3, 2, 4, 1, 2, 3]) * 1.0,
             linefmt="-", markerfmt="o", basefmt="-", bottom=0)
axes[1].set_title("stem")
for a in axes:
    a.tick_params(length=2)
fig.tight_layout(pad=0.3)
finish(fig, "step_stem")
