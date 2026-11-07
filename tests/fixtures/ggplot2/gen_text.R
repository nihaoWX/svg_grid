# Text: geom_text/geom_label alignment + rotation, annotate(), rotated axis text.
source("helpers.R")
plant_seed()

# --- text_angle: hjust / vjust / angle / size + geom_label boxes --------------
d <- data.frame(x = 1:4, y = c(1, 3, 2, 4),
                lab = c("alpha", "beta", "gamma", "delta"))
p <- ggplot(d, aes(x, y, label = lab)) +
  geom_text(angle = 45, hjust = 0, vjust = 0, size = 2.4) +
  geom_text(aes(y = y - 0.6), angle = 90, hjust = 1, size = 2) +
  geom_label(aes(y = y + 0.5), size = 2, label.size = 0.1) +
  coord_cartesian(xlim = c(0.5, 4.5), ylim = c(0, 5)) +
  theme_bw(base_size = 8)
save_svg("text_angle", p)

# --- annotate: annotate(text/rect/segment/point) + rotated tick labels -------
p <- ggplot(mtcars, aes(wt, mpg)) +
  annotate("rect", xmin = 2.5, xmax = 4, ymin = 25, ymax = 33,
           fill = "gold", alpha = 0.3, colour = "goldenrod", linewidth = 0.3) +
  annotate("text", x = 3.2, y = 30, label = "sweet spot", size = 2.4,
           fontface = "italic") +
  annotate("segment", x = 2, y = 15, xend = 4.5, yend = 32,
           linetype = "dashed", colour = "purple", linewidth = 0.4) +
  annotate("point", x = 4.5, y = 32, size = 1.6, colour = "purple") +
  geom_point(size = 0.6, colour = "grey40") +
  scale_x_continuous(breaks = seq(1.5, 5.5, 0.5)) +
  theme_classic(base_size = 8) +
  theme(axis.text.x = element_text(angle = 45, hjust = 1, vjust = 1))
save_svg("annotate", p)
