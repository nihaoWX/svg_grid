# Scales (log / reverse / date), guides (legend placement + merging), themes.
source("helpers.R")
plant_seed()

# --- scales_log: log10 y axis, reversed x, numeric labels -------------------
set.seed(41)
d <- data.frame(x = 1:20, y = 10^seq(0, 3, length.out = 20) * rnorm(20, 1, 0.15))
p <- ggplot(d, aes(x, y)) +
  geom_point(size = 0.5) +
  geom_line(linewidth = 0.3) +
  scale_y_log10() +
  scale_x_reverse() +
  theme_bw(base_size = 8)
save_svg("scales_log", p)

# --- scales_date: date axis with fixed breaks ------------------------------
d <- data.frame(day = as.Date("2026-01-01") + seq(0, 84, by = 6),
                v = c(3, 4, 3.5, 5, 6, 5.5, 7, 6.5, 8, 7.5, 9, 8.5, 10, 9.5, 11))
p <- ggplot(d, aes(day, v)) +
  geom_line(linewidth = 0.4, colour = "steelblue") +
  geom_point(size = 0.6) +
  scale_x_date(date_breaks = "3 weeks", date_labels = "%b %d") +
  theme_minimal(base_size = 8) +
  theme(axis.text.x = element_text(angle = 30, hjust = 1))
save_svg("scales_date", p)

# --- guides: bottom legend, two aesthetics merged into one key --------------
d <- data.frame(x = 1:12, y = c(2, 3, 5, 4, 6, 7, 5, 8, 9, 7, 10, 11),
                z = factor(rep(c("lo", "mid", "hi"), 4)))
p <- ggplot(d, aes(x, y, colour = z, shape = z)) +
  geom_line(linewidth = 0.4) +
  geom_point(size = 0.9) +
  theme_classic(base_size = 8) +
  theme(legend.position = "bottom",
        legend.direction = "horizontal",
        legend.title = element_text(size = 7))
save_svg("guides", p)

# --- themes: theme_dark + rotated axis labels + panel border ----------------
d <- data.frame(g = factor(c("one", "two", "three")), v = c(2, 5, 3))
p <- ggplot(d, aes(g, v, fill = g)) +
  geom_col(width = 0.6, colour = "white", linewidth = 0.2) +
  theme_dark(base_size = 8) +
  theme(panel.border = element_rect(colour = "grey80", fill = NA),
        axis.text.x = element_text(angle = -30, hjust = 0),
        legend.position = "none")
save_svg("themes", p)
