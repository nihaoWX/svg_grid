# Coordinate systems: coord_flip and coord_polar.
source("helpers.R")
plant_seed()

# --- coord_flip: horizontal columns with a flipped discrete axis --------------
d <- data.frame(cat = c("alpha", "beta", "gamma", "delta"),
                val = c(4, 7, 2, 5))
p <- ggplot(d, aes(cat, val, fill = cat)) +
  geom_col(width = 0.7) +
  geom_text(aes(label = val), hjust = -0.2, size = 2.2) +
  coord_flip() +
  guides(fill = "none") +
  theme_bw(base_size = 8)
save_svg("coord_flip", p)

# --- coord_polar: pie chart (theta = y) with a hole --------------------------
d <- data.frame(grp = c("a", "b", "c"), n = c(3, 5, 2))
p <- ggplot(d, aes(x = 2, y = n, fill = grp)) +
  geom_col(width = 1, colour = "white", linewidth = 0.3) +
  coord_polar(theta = "y") +
  xlim(0.5, 2.5) +
  theme_void(base_size = 8) +
  theme(legend.position = "right")
save_svg("coord_polar", p)
