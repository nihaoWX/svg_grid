"""Distribution plots: boxplot, violinplot, errorbar, fill_between."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(2)

# --- boxplot: patch_artist boxes, fliers, legend --------------------------
fig, ax = plt.subplots()
data = [rng.normal(m, s, 25) for m, s in [(0, 1), (1, 0.6), (2, 1.2), (0.5, 0.4)]]
bp = ax.boxplot(data, patch_artist=True, widths=0.5, showfliers=True,
                medianprops=dict(color="black", linewidth=0.6),
                flierprops=dict(marker="o", markersize=1.5, alpha=0.6))
for patch, col in zip(bp["boxes"], ["#c6dbef", "#9ecae1", "#6baed6", "#4292c6"]):
    patch.set_facecolor(col)
ax.set_xticklabels(["A", "B", "C", "D"])
ax.set_ylabel("value")
fig.tight_layout(pad=0.3)
finish(fig, "boxplot")

# --- violinplot: violin bodies + extrema + inner box --------------------
fig, ax = plt.subplots()
vp = ax.violinplot([rng.normal(m, 0.8, 30) for m in (0, 1, 2)],
                   showmeans=True, showextrema=True, showmedians=True,
                   widths=0.7, points=60)
for body in vp["bodies"]:
    body.set_facecolor("mediumpurple")
    body.set_alpha(0.6)
ax.set_xticks([1, 2, 3], ["v1", "v2", "v3"])
fig.tight_layout(pad=0.3)
finish(fig, "violinplot")

# --- errorbar: asymmetric xerr/yerr with caps, marker + line ------------
fig, ax = plt.subplots()
x = np.arange(1, 7)
y = np.array([2, 3, 2.5, 4, 3.5, 5])
ax.errorbar(x, y, yerr=[np.full(6, 0.4), np.full(6, 0.9)],
            xerr=0.2, fmt="o-", markersize=2.5, linewidth=0.7,
            capsize=2, capthick=0.5, elinewidth=0.6, label="series")
ax.fill_between(x, y - 0.2, y + 0.2, alpha=0.15, linewidth=0)
ax.set_xlabel("x")
ax.legend(frameon=False, loc="upper left")
fig.tight_layout(pad=0.3)
finish(fig, "errorbar")

# --- fill_between: band with a where-mask and a step edge --------------
fig, ax = plt.subplots()
x = np.linspace(0, 10, 40)
lo, hi = np.sin(x) - 0.4, np.sin(x) + 0.4
ax.plot(x, np.sin(x), linewidth=0.7)
ax.fill_between(x, lo, hi, color="tab:green", alpha=0.25, linewidth=0)
ax.fill_between(x, 0, np.sin(x), where=np.sin(x) > 0, color="tab:red",
                alpha=0.2, step="mid", linewidth=0)
ax.axhline(0, color="grey", linestyle="--", linewidth=0.3)
ax.set_ylim(-1.3, 1.3)
fig.tight_layout(pad=0.3)
finish(fig, "fill_between")
