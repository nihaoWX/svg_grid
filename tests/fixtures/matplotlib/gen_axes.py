"""Axis scales: log axes (auto mathtext) and a date axis."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish
import datetime as dt

rng = np.random.default_rng(6)

# --- log_axes: log-log plot with minor ticks (mathtext tick labels) -----
fig, axes = plt.subplots(1, 2)
x = np.logspace(0, 3, 25)
axes[0].loglog(x, x ** 1.8 * (1 + 0.1 * rng.normal(size=25)), linewidth=0.7)
axes[0].set_title("loglog")
axes[1].semilogy(x, np.exp(-x / 200) * 1000 + 0.1, linewidth=0.7)
axes[1].set_title("semilogy")
axes[1].grid(True, which="minor", linewidth=0.2, alpha=0.4)
fig.tight_layout(pad=0.3)
finish(fig, "log_axes")

# --- date_axis: date ticks with a rotated formatter ---------------------
fig, ax = plt.subplots()
start = dt.date(2026, 1, 1)
days = [start + dt.timedelta(days=7 * i) for i in range(8)]
ax.plot(days, [3, 5, 4, 6, 5, 7, 6, 8], marker="o", markersize=2,
        linewidth=0.7)
ax.set_xlabel("week")
ax.set_ylabel("value")
for lab in ax.get_xticklabels():
    lab.set_rotation(30)
    lab.set_ha("right")
fig.autofmt_xdate(rotation=30, ha="right")
fig.tight_layout(pad=0.3)
finish(fig, "date_axis")
