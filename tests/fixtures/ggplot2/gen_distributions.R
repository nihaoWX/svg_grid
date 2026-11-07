# Distribution geoms: boxplot, violin, loess smooth with ribbon, error bars.
source("helpers.R")
plant_seed()

# --- boxplot: geom_boxplot + geom_jitter overlay -----------------------------
p <- ggplot(mtcars, aes(factor(cyl), mpg, fill = factor(cyl))) +
  geom_boxplot(width = 0.55, outlier.size = 0.6, linewidth = 0.3) +
  geom_jitter(width = 0.08, size = 0.6, alpha = 0.6) +
  guides(fill = "none") +
  theme_bw(base_size = 8)
save_svg("boxplot", p)

# --- violin: geom_violin (mirrored outline) + boxplot inside ------------------
p <- ggplot(mtcars, aes(factor(cyl), mpg, fill = factor(cyl))) +
  geom_violin(trim = FALSE, linewidth = 0.3, alpha = 0.7) +
  geom_boxplot(width = 0.12, outlier.shape = NA, fill = "white", linewidth = 0.3) +
  theme_minimal(base_size = 8)
save_svg("violin", p)

# --- smooth_ribbon: loess fit with confidence ribbon -------------------------
set.seed(3)
d <- data.frame(x = sort(runif(40, 0, 10)))
d$y <- sin(d$x) + rnorm(40, 0, 0.35)
p <- ggplot(d, aes(x, y)) +
  geom_point(size = 0.8, alpha = 0.7) +
  geom_smooth(method = "loess", formula = y ~ x, se = TRUE,
              linewidth = 0.6, fill = "steelblue", alpha = 0.35) +
  theme_bw(base_size = 8)
save_svg("smooth_ribbon", p)

# --- errorbar: errorbar + pointrange + linerange + crossbar -------------------
set.seed(5)
d <- data.frame(g = factor(c("a", "b", "c")),
                m = c(2, 3.5, 3),
                lo = c(1.4, 2.8, 2.5),
                hi = c(2.6, 4.4, 3.6))
p <- ggplot(d, aes(g, m)) +
  geom_errorbar(aes(ymin = lo, ymax = hi), width = 0.22, linewidth = 0.4) +
  geom_pointrange(aes(ymin = lo, ymax = hi), size = 0.3, linewidth = 0.4,
                  colour = "darkred") +
  geom_linerange(aes(ymin = lo - 0.3, ymax = hi + 0.3),
                 colour = "steelblue", linewidth = 0.4) +
  geom_crossbar(aes(y = m, ymin = m - 0.2, ymax = m + 0.2), width = 0.5,
                fill = "grey70", linewidth = 0.3) +
  theme_classic(base_size = 8)
save_svg("errorbar", p)
