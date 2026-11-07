# Field geoms: tile heatmap with a continuous fill scale, 2D density contours.
source("helpers.R")
plant_seed()

# --- tile_heatmap: geom_tile + scale_fill_gradient2 + tile labels ------------
set.seed(31)
d <- expand.grid(x = letters[1:5], y = LETTERS[1:4])
d$z <- round(rnorm(nrow(d)), 2)
p <- ggplot(d, aes(x, y, fill = z)) +
  geom_tile(colour = "white", linewidth = 0.2) +
  scale_fill_gradient2(low = "#2166AC", mid = "white", high = "#B2182B",
                       midpoint = 0) +
  theme_minimal(base_size = 8) +
  theme(axis.text.x = element_text(angle = 90, hjust = 1))
save_svg("tile_heatmap", p)

# --- contour_density: geom_density_2d + geom_density_2d_filled ---------------
p <- ggplot(mtcars, aes(wt, mpg)) +
  geom_density_2d_filled(alpha = 0.7, bins = 3) +
  geom_density_2d(colour = "black", linewidth = 0.2, bins = 3) +
  geom_point(size = 0.5, colour = "white") +
  theme_bw(base_size = 8) +
  theme(legend.position = "none")
save_svg("contour_density", p)
