# Bar-ish geoms: stacked/dodged columns, histogram + density + rug.
source("helpers.R")
plant_seed()

d <- data.frame(cat = rep(c("a", "b", "c", "d"), each = 3),
                grp = rep(c("u", "v", "w"), 4),
                val = c(3, 2, 1, 2, 4, 2, 5, 1, 2, 2, 3, 4))

# --- bars_stacked: geom_col with position = "stack" and a legend --------------
p <- ggplot(d, aes(cat, val, fill = grp)) +
  geom_col(position = "stack", width = 0.7) +
  scale_fill_brewer(palette = "Pastel1") +
  theme_bw(base_size = 8)
save_svg("bars_stacked", p)

# --- bars_dodged: position = "dodge" + geom_text count labels -----------------
p <- ggplot(d, aes(cat, val, fill = grp)) +
  geom_col(position = position_dodge(width = 0.8), width = 0.7) +
  theme_minimal(base_size = 8) +
  theme(legend.position = "right")
save_svg("bars_dodged", p)

# --- hist_density: histogram + density curve + rug ---------------------------
set.seed(11)
x <- c(rnorm(60, 0, 1), rnorm(30, 2, 0.6))
p <- ggplot(data.frame(x = x), aes(x)) +
  geom_histogram(aes(y = after_stat(density)), bins = 14,
                 fill = "grey80", colour = "white", linewidth = 0.2) +
  geom_density(linewidth = 0.6, colour = "darkgreen") +
  geom_rug(alpha = 0.5, length = unit(0.02, "npc")) +
  theme_classic(base_size = 8)
save_svg("hist_density", p)

# --- gradient_fill: gradient fills (linearGradient / radialGradient) ---------
p <- ggplot(d, aes(cat, val)) +
  geom_col(fill = grid::linearGradient(c("#deebf7", "#08519c"), stops = c(0, 1)),
           width = 0.6) +
  geom_point(aes(y = val + 0.5), size = 1.5,
             fill = grid::radialGradient(c("#fee0d2", "#67000d"))) +
  theme_bw(base_size = 8) +
  theme(panel.background = element_rect(
          fill = grid::linearGradient(c("white", "grey90"), x1 = 0, y1 = 0,
                                      x2 = 1, y2 = 1)))
save_svg("gradient_fill", p)
