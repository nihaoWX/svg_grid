"""Bar-ish plots: grouped, stacked, horizontal, histogram."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(1)

# --- bars_grouped: grouped bars + hatch + legend ---------------------------
fig, ax = plt.subplots()
cats = np.arange(4)
w = 0.35
ax.bar(cats - w / 2, [3, 5, 2, 4], width=w, label="plain", color="tab:blue")
ax.bar(cats + w / 2, [4, 2, 5, 3], width=w, label="hatched", color="tab:orange",
       hatch="//", edgecolor="black", linewidth=0.3)
ax.set_xticks(cats, ["a", "b", "c", "d"])
ax.set_ylabel("count")
ax.legend(frameon=False, labelspacing=0.2)
fig.tight_layout(pad=0.3)
finish(fig, "bars_grouped")

# --- bars_stacked: stacked bars + total labels -----------------------------
fig, ax = plt.subplots()
a = np.array([2, 4, 3, 5])
b = np.array([3, 2, 2, 1])
c = np.array([1, 1, 3, 2])
ax.bar(cats, a, label="a")
ax.bar(cats, b, bottom=a, label="b")
ax.bar(cats, c, bottom=a + b, label="c")
for i in range(4):
    ax.text(cats[i], a[i] + b[i] + c[i] + 0.1, str(a[i] + b[i] + c[i]),
            ha="center", va="bottom", fontsize=5)
ax.set_xticks(cats, ["s1", "s2", "s3", "s4"])
ax.legend(frameon=False, ncol=3, columnspacing=0.6, handlelength=1.0)
fig.tight_layout(pad=0.3)
finish(fig, "bars_stacked")

# --- barh: horizontal bars with value labels + grid -----------------------
fig, ax = plt.subplots()
labels = ["alpha", "beta", "gamma", "delta", "epsilon"]
vals = np.array([5, 3, 7, 2, 6])
ax.barh(labels, vals, height=0.6, color="skyblue", edgecolor="steelblue",
        linewidth=0.4)
for i, v in enumerate(vals):
    ax.text(v + 0.15, i, str(v), va="center", fontsize=5)
ax.set_xlim(0, 8)
ax.grid(axis="x", linestyle=":", linewidth=0.3)
ax.set_axisbelow(True)
fig.tight_layout(pad=0.3)
finish(fig, "barh")

# --- hist: stacked histograms, density, log=False -------------------------
fig, ax = plt.subplots()
ax.hist([rng.normal(0, 1, 40), rng.normal(1.5, 0.8, 40)],
        bins=10, stacked=True, label=["g1", "g2"], edgecolor="white",
        linewidth=0.3)
ax.set_xlabel("value")
ax.set_ylabel("count")
ax.legend(frameon=False, labelspacing=0.2)
fig.tight_layout(pad=0.3)
finish(fig, "hist")
