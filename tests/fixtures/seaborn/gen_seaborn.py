"""seaborn figures: heatmap, kdeplot, regplot, catplot (FacetGrid)."""

import sys, os
# the shared helpers live next to the matplotlib fixtures
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(
    os.path.abspath(__file__))), "matplotlib"))
from helpers import plt, np, finish, matplotlib
import seaborn as sns

HERE = os.path.dirname(os.path.abspath(__file__))

rng = np.random.default_rng(7)

# seaborn's set_theme rewrites rcParams, so re-apply the ones the corpus needs.
sns.set_theme(style="white", font_scale=0.75)
matplotlib.rcParams.update({
    "svg.fonttype": "none",
    "svg.hashsalt": "svg-grid-fixtures",
    "font.family": "sans-serif",
    "font.sans-serif": ["DejaVu Sans"],
})

# --- heatmap: annotated matrix with diverging colormap + colorbar --------
fig, ax = plt.subplots()
m = rng.normal(size=(5, 6))
sns.heatmap(m, ax=ax, cmap="RdBu_r", center=0, annot=True, fmt=".1f",
            annot_kws={"fontsize": 4}, linewidths=0.3, linecolor="white",
            cbar_kws={"shrink": 0.8, "pad": 0.02})
ax.set_title("heatmap")
fig.tight_layout(pad=0.3)
finish(fig, "heatmap", HERE)

# --- kdeplot: filled 2D density + contour lines -------------------------
fig, ax = plt.subplots()
x = rng.normal(size=70)
y = 0.7 * x + rng.normal(size=70)
sns.kdeplot(x=x, y=y, ax=ax, fill=True, levels=4, cmap="viridis",
            thresh=0.1, linewidths=0.3)
ax.set_title("kdeplot")
fig.tight_layout(pad=0.3)
finish(fig, "kdeplot", HERE)

# --- regplot: scatter + regression line + bootstrap CI band -------------
fig, ax = plt.subplots()
sns.regplot(x=x, y=y, ax=ax, ci=95, scatter_kws={"s": 6, "alpha": 0.6},
            line_kws={"linewidth": 0.7})
ax.set_title("regplot")
fig.tight_layout(pad=0.3)
finish(fig, "regplot", HERE)

# --- catplot: box plot small multiples ---------------------------------
import pandas as pd
df = pd.DataFrame({
    "grp": np.repeat(["p", "q", "r"], 25),
    "cond": np.tile(["one", "two"], 38)[:75],
    "val": rng.normal(size=75) + np.repeat([0, 1, 2], 25),
})
g = sns.catplot(data=df, x="cond", y="val", col="grp", kind="box",
                height=1.6, aspect=0.62, linewidth=0.4, fliersize=1)
g.figure.suptitle("catplot")
finish(g.figure, "catplot", HERE)
