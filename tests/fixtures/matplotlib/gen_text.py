"""Text: mathtext labels (tspan runs) and annotate with an arrow + bbox."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(5)

# --- mathtext: math in title/labels/ticks/annotations --------------------
fig, ax = plt.subplots()
x = np.linspace(0.5, 4, 25)
ax.plot(x, np.log(x), linewidth=0.7, label=r"$\ln(x)$")
ax.set_xlabel(r"$x$ [$\mathrm{a.u.}$]")
ax.set_ylabel(r"$\alpha\,\beta^2 + \sum_{i=1}^{N} x_i$")
ax.set_title(r"$\mathbf{F}(x)=\int_0^1 f\,dx$")
ax.text(1.2, 0.9, r"$\frac{1}{2}\sqrt{\pi}$", fontsize=7)
ax.legend(frameon=False)
fig.tight_layout(pad=0.3)
finish(fig, "mathtext")

# --- annotate_arrow: arrow annotations with boxes and rotations ---------
fig, ax = plt.subplots()
ax.plot(np.linspace(0, 4, 20), np.sin(np.linspace(0, 4, 20)) * 2,
        linewidth=0.7, color="tab:purple")
ax.annotate("peak", xy=(1.57, 2.0), xytext=(2.4, 2.6), fontsize=6,
            arrowprops=dict(arrowstyle="->", linewidth=0.5,
                            connectionstyle="arc3,rad=-0.25"))
ax.annotate("trough", xy=(4.0, -1.5), xytext=(1.6, -2.2), fontsize=6,
            bbox=dict(boxstyle="round,pad=0.25", fc="lightyellow",
                      ec="grey", linewidth=0.4),
            arrowprops=dict(arrowstyle="-|>", linewidth=0.5))
ax.text(0.2, 2.4, "rotated", rotation=30, fontsize=6, ha="left", va="top")
ax.set_xlim(0, 4.6)
ax.set_ylim(-2.8, 3.2)
fig.tight_layout(pad=0.3)
finish(fig, "annotate_arrow")
