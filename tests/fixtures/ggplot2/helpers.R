# Shared helpers for the ggplot2 fixtures.
#
# Regenerate every fixture in this directory with:
#   micromamba run -n r453 Rscript gen_<group>.R      (run from this directory)
# Every script is deterministic: fixed seed, fixed 3 x 2 in canvas, no network.

suppressPackageStartupMessages(library(ggplot2))

# Directory of the running script (fall back to the working directory).
script_dir <- function() {
  a <- commandArgs(trailingOnly = FALSE)
  hit <- grep("^--file=", a, value = TRUE)
  if (length(hit)) dirname(normalizePath(sub("^--file=", "", hit[[1]]))) else getwd()
}

# Render `plot` to <name>.svg at exactly 3 x 2 in and report the file size.
save_svg <- function(name, plot, width = 3, height = 2) {
  f <- file.path(script_dir(), paste0(name, ".svg"))
  svglite::svglite(f, width = width, height = height)
  print(plot)
  invisible(dev.off())
  cat(sprintf("wrote %-28s %6.1f KB\n", basename(f), file.size(f) / 1024))
}

# Fixed seed, called by every script before drawing anything.
plant_seed <- function() set.seed(20260101)
