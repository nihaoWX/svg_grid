# Facets: facet_wrap and facet_grid (strip bands, per-panel axes).
source("helpers.R")
plant_seed()

# --- facet_wrap: 2 x 2 strip-wrapped panels ----------------------------------
d <- subset(mtcars, cyl %in% c(4, 6, 8))
d$cls <- factor(paste0(d$cyl, "cyl"))
p <- ggplot(d, aes(wt, mpg)) +
  geom_point(size = 0.7) +
  geom_smooth(method = "lm", formula = y ~ x, se = FALSE, linewidth = 0.4) +
  facet_wrap(~cls, ncol = 2, scales = "free_x") +
  theme_bw(base_size = 7)
save_svg("facet_wrap", p)

# --- facet_grid: row x column grid of panels --------------------------------
d2 <- subset(mtcars, am %in% c(0, 1) & cyl %in% c(4, 6, 8))
d2$am_lab <- factor(ifelse(d2$am == 1, "manual", "auto"))
p <- ggplot(d2, aes(wt, mpg)) +
  geom_point(size = 0.6) +
  facet_grid(am_lab ~ factor(cyl)) +
  theme_bw(base_size = 7)
save_svg("facet_grid", p)
