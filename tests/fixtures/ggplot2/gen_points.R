# Point-shaped geoms: shapes, size, alpha, jitter, stacked dots.
source("helpers.R")
plant_seed()

# --- points_aes: geom_point with colour + shape + size + alpha mappings -------
p <- ggplot(mtcars, aes(wt, mpg, colour = factor(cyl), shape = factor(am), size = hp)) +
  geom_point(alpha = 0.85) +
  scale_colour_brewer(palette = "Set1") +
  scale_shape_manual(values = c(16, 17)) +
  scale_size_continuous(range = c(0.5, 2)) +
  labs(x = "wt", y = "mpg") +
  theme_bw(base_size = 8)
save_svg("points_aes", p)

# --- jitter: geom_jitter over a discrete axis --------------------------------
set.seed(7)
d <- data.frame(g = rep(c("a", "b", "c"), each = 14),
                y = c(rnorm(14, 1), rnorm(14, 2), rnorm(14, 3)))
p <- ggplot(d, aes(g, y, colour = g)) +
  geom_jitter(width = 0.15, alpha = 0.7) +
  stat_summary(fun = mean, geom = "crossbar", width = 0.5, colour = "black") +
  guides(colour = "none") +
  theme_classic(base_size = 8)
save_svg("jitter", p)

# --- dotplot: geom_dotplot stacking + geom_hline ------------------------------
p <- ggplot(mtcars, aes(factor(cyl), mpg)) +
  geom_dotplot(binaxis = "y", stackdir = "center", dotsize = 0.35,
               fill = "grey40", binwidth = 1.2) +
  geom_hline(yintercept = 20, linetype = "dashed") +
  theme_minimal(base_size = 8)
save_svg("dotplot", p)
