# Line-ish geoms: linetype arrays, step interpolation, segments/ablines,
# curvature (arc commands), quantile regression lines.
source("helpers.R")
plant_seed()

# --- lines_linetypes: geom_line + geom_path with 4 linetypes + points ---------
d <- data.frame(x = rep(1:8, 4),
                y = c(1:8, 2:9, 8:1, c(1, 3, 2, 4, 3, 5, 4, 6)),
                k = factor(rep(c("solid", "dashed", "dotted", "longdash"), each = 8)))
p <- ggplot(d, aes(x, y, colour = k, linetype = k)) +
  geom_line(linewidth = 0.5) +
  geom_point(size = 1) +
  scale_linetype_manual(values = c("solid", "dashed", "dotted", "longdash")) +
  theme_bw(base_size = 8)
save_svg("lines_linetypes", p)

# --- steps: geom_step both directions + a connecting point layer --------------
d <- data.frame(x = 1:7, y = c(2, 4, 3, 6, 5, 8, 7))
p <- ggplot(d, aes(x, y)) +
  geom_step(direction = "hv", colour = "steelblue") +
  geom_step(direction = "vh", linetype = "dashed", colour = "darkred") +
  geom_point(size = 1.2) +
  theme_minimal(base_size = 8)
save_svg("steps", p)

# --- segments: geom_segment + hline/vline/abline + arrow head -----------------
p <- ggplot(mtcars, aes(wt, mpg)) +
  geom_point(size = 0.8, colour = "grey50") +
  geom_hline(yintercept = 20, linetype = "dotted") +
  geom_vline(xintercept = 3, linetype = "dashed") +
  geom_abline(intercept = 37, slope = -5, colour = "red", linewidth = 0.4) +
  geom_segment(aes(x = 2, y = 30, xend = 4.5, yend = 15),
               arrow = arrow(length = unit(0.06, "inches")), linewidth = 0.5) +
  theme_bw(base_size = 8)
save_svg("segments", p)

# --- curve: geom_curve => arcs in the path data -------------------------------
p <- ggplot(data.frame(x = c(1, 4), y = c(1, 3), lab = c("a", "b")),
            aes(x, y)) +
  geom_curve(aes(xend = 4, yend = 1), curvature = 0.45,
             arrow = arrow(length = unit(0.06, "inches")), linewidth = 0.5) +
  geom_curve(aes(x = 1, y = 3, xend = 4, yend = 3), curvature = -0.45,
             linetype = "dashed") +
  geom_text(aes(label = lab), size = 2.5) +
  coord_cartesian(xlim = c(0, 5), ylim = c(0, 4)) +
  theme_void(base_size = 8)
save_svg("curve", p)

# --- quantile: geom_quantile (needs quantreg) ---------------------------------
p <- ggplot(mtcars, aes(wt, mpg)) +
  geom_point(size = 0.7, alpha = 0.6) +
  geom_quantile(formula = y ~ x, quantiles = c(0.1, 0.5, 0.9), linewidth = 0.4) +
  theme_classic(base_size = 8)
save_svg("quantile", p)
