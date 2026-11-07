"""Field plots: imshow (embedded raster), pcolormesh, contour(f), quiver, hexbin."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from helpers import plt, np, finish

rng = np.random.default_rng(3)

# --- imshow: raster heat map + continuous colorbar (embedded PNG) ---------
fig, ax = plt.subplots()
z = rng.random((24, 30))
im = ax.imshow(z, cmap="magma", interpolation="nearest", aspect="auto")
fig.colorbar(im, ax=ax, shrink=0.85, pad=0.02)
ax.set_xticks([])
ax.set_yticks([])
ax.set_title("imshow")
fig.tight_layout(pad=0.3)
finish(fig, "imshow")

# --- pcolormesh: quadrilateral mesh with edges + colorbar ---------------
fig, ax = plt.subplots()
gx, gy = np.meshgrid(np.linspace(0, 1, 9), np.linspace(0, 1, 7))
zz = (np.sin(6 * gx) * np.cos(5 * gy))[:-1, :-1]
pm = ax.pcolormesh(gx, gy, zz, cmap="coolwarm", edgecolors="white",
                   linewidth=0.2, shading="flat")
fig.colorbar(pm, ax=ax, shrink=0.85, pad=0.02)
ax.set_title("pcolormesh")
fig.tight_layout(pad=0.3)
finish(fig, "pcolormesh")

# --- contour: filled contours + isolines + inline labels ----------------
fig, ax = plt.subplots()
x = np.linspace(-2, 2, 40)
X, Y = np.meshgrid(x, x)
Z = np.exp(-(X ** 2 + Y ** 2)) - 0.5 * np.exp(-((X - 1) ** 2 + (Y + 1) ** 2))
cf = ax.contourf(X, Y, Z, levels=6, cmap="viridis", alpha=0.9)
cs = ax.contour(X, Y, Z, levels=4, colors="black", linewidths=0.4)
ax.clabel(cs, inline=True, fontsize=4, fmt="%.2f")
fig.colorbar(cf, ax=ax, shrink=0.85, pad=0.02)
fig.tight_layout(pad=0.3)
finish(fig, "contour")

# --- quiver: vector field arrows ---------------------------------------
fig, ax = plt.subplots()
qx, qy = np.meshgrid(np.linspace(-1, 1, 9), np.linspace(-1, 1, 7))
u, v = -qy, qx
ax.quiver(qx, qy, u, v, np.hypot(u, v), cmap="plasma", scale=12,
          width=0.008, headwidth=3)
ax.set_title("quiver")
fig.tight_layout(pad=0.3)
finish(fig, "quiver")

# --- hexbin: hexagonal binning + colorbar ------------------------------
fig, ax = plt.subplots()
hx, hy = rng.normal(size=120), rng.normal(size=120)
hb = ax.hexbin(hx, hy, gridsize=10, cmap="viridis", mincnt=1, linewidths=0.2)
fig.colorbar(hb, ax=ax, shrink=0.85, pad=0.02)
ax.set_title("hexbin")
fig.tight_layout(pad=0.3)
finish(fig, "hexbin")
