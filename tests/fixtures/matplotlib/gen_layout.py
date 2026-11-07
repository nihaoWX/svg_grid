"""Layout: gridspec subplots with suptitle, and twinx/twiny twin axes."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(4)

# --- subplots_gridspec: 2x2 gridspec, shared axes, suptitle, panel letters -
fig = plt.figure()
gs = fig.add_gridspec(2, 2, height_ratios=[2, 1], hspace=0.5, wspace=0.35)
ax1 = fig.add_subplot(gs[0, 0])
ax2 = fig.add_subplot(gs[0, 1])
ax3 = fig.add_subplot(gs[1, :], sharex=ax1)
x = np.linspace(0, 5, 25)
ax1.plot(x, np.sin(x), linewidth=0.7)
ax2.scatter(rng.normal(size=20), rng.normal(size=20), s=6)
ax3.plot(x, np.cos(x), linewidth=0.7)
ax3.set_xlabel("x")
ax1.set_title("A", loc="left", fontsize=7)
ax2.set_title("B", loc="left", fontsize=7)
ax3.set_title("C", loc="left", fontsize=7)
fig.suptitle("gridspec layout")
fig.tight_layout(pad=0.3, rect=(0, 0, 1, 0.94))
finish(fig, "subplots_gridspec")

# --- twinx: two y axes (+ a twiny x axis) with two legends ---------------
fig, ax = plt.subplots()
t = np.linspace(0, 10, 30)
l1, = ax.plot(t, np.sin(t), color="tab:blue", linewidth=0.8, label="sin")
ax.set_ylabel("sin", color="tab:blue")
ax2 = ax.twinx()
l2, = ax2.plot(t, np.exp(t / 4), color="tab:red", linewidth=0.8, label="exp")
ax2.set_ylabel("exp", color="tab:red")
ax3 = ax.twiny()
ax3.plot(np.linspace(0, 100, 30), np.linspace(0, 1, 30), color="tab:green",
         linestyle=":", linewidth=0.6, label="aux")
ax.legend(handles=[l1, l2], frameon=False, loc="upper left", labelspacing=0.2)
ax.set_xlabel("t")
fig.tight_layout(pad=0.3)
finish(fig, "twinx")
