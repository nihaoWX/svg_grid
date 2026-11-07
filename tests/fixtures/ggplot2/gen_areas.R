# Area-ish geoms: stacked area, ribbon band, polygon rings.
source("helpers.R")
plant_seed()

# --- area_ribbon: geom_area stacked + geom_ribbon band ------------------------
d <- data.frame(x = rep(1:8, 3),
                y = c(1:8, 8:1, rep(c(2, 3, 2.5, 4, 3, 5, 4, 6), 1)),
                g = factor(rep(c("p", "q", "r"), each = 8)))
d$lo <- d$y - 1
d$hi <- d$y + 1
p <- ggplot(d, aes(x, y, fill = g)) +
  geom_area(colour = "grey20", linewidth = 0.2, alpha = 0.8) +
  geom_ribbon(aes(ymin = lo, ymax = hi, group = g), fill = "white", alpha = 0.3,
              colour = "grey30", linewidth = 0.2) +
  theme_bw(base_size = 8)
save_svg("area_ribbon", p)

# --- polygon: geom_polygon closed rings (two shapes) -------------------------
set.seed(21)
outer <- data.frame(x = cos(seq(0, 2 * pi, length = 12)) * 2 + 2,
                    y = sin(seq(0, 2 * pi, length = 12)) * 2 + 2)
inner <- data.frame(x = cos(seq(0, 2 * pi, length = 10)) * 0.7 + 2,
                    y = sin(seq(0, 2 * pi, length = 10)) * 0.7 + 2)
spots <- data.frame(x = runif(12, 0, 4), y = runif(12, 0, 4))
p <- ggplot() +
  geom_polygon(data = outer, aes(x, y), fill = "steelblue", alpha = 0.5,
               colour = "navy", linewidth = 0.4) +
  geom_polygon(data = inner, aes(x, y), fill = "white", colour = "navy",
               linewidth = 0.3) +
  geom_point(data = spots, aes(x, y), size = 0.6, colour = "firebrick") +
  coord_equal(xlim = c(0, 4), ylim = c(0, 4)) +
  theme_void(base_size = 8)
save_svg("polygon", p)
