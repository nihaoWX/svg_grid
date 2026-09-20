# 回退路线：R + cowplot 组图

> **这是回退路线，不是默认路线。** 默认走 `02a_svg_grid_assembly.md`（`svg_grid`）。
> 只在下面三条判据之一成立时才走这条。

## 0. 什么时候才走这条

- `svg_grid_convert` 对面板方言 **fail-closed**（RDKit `MolDraw2DSVG`、Inkscape/Illustrator 导出等），且无法改用 `svglite`/matplotlib 重出；
- `svg_grid` / `svg_grid_convert` **不可用**（没编译、环境不支持、离线构建失败）；
- 面板**拿不到独立 `.svg`**，只能以会话内对象（ggplot/grob）组合。

**回退的代价**（决定回退前先认下）：这条路的**文件级**读图依赖 `magick`，本机实测过它**静默产出 0.00% 墨量的全白图**（仅告警、退出码 0）——用它读图组图**必须**检查输出非空白（见 §3）。

## 1. 为什么用 cowplot

`cowplot::plot_grid()` 可以：
- 任意行列拼接 ggplot / grob / 图片对象
- 统一加 **A/B/C 面板字母**（位置、字体、字号、字重可控）
- 按 `rel_heights` / `rel_widths` 精确分配面板占比
- 统一画布边距与白底

对象级组图是这条路线唯一的长处：面板是**弹性 grob**，不必先按目标格子尺寸出图，`rel_*` 直接给份额即可。

## 2. 基本范式

```r
library(cowplot)
library(ggplot2)
library(png)

# 画布与输出
plot_dir <- "Figure"
if (!dir.exists(plot_dir)) dir.create(plot_dir)

# 面板：可以是 ggplot 对象，也可以是已有 PNG 读成 ggplot 对象
read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img, x = 0, y = 0, width = 1, height = 1)
}

# 1) 直接拼 ggplot 对象
pp1 <- plot_grid(p1, p2, ncol = 2,
                 labels = c("A", "B"),
                 label_fontfamily = "serif",   # 字体：serif
                 label_size = 30,
                 label_x = -0.04, label_y = 1) # 字母位置：panel 左上角外侧

# 2) 多层嵌套组合
ppp1 <- plot_grid(pp1, pp2, p3, ncol = 1,
                  rel_heights = c(0.2, 0.2, 0.6),
                  labels = c("", "", "E"),
                  label_fontfamily = "serif", label_size = 30,
                  label_x = -0.02, label_y = 1)

# 3) 统一画布边距 + 白底
ppp1 <- ggdraw(ppp1) +
  theme(plot.margin = margin(t = 35, r = 35, b = 35, l = 35, unit = "pt"),
        plot.background = element_rect(fill = "white", color = NA))

# 4) 落盘：成品 = .svgz + .png 成对（A4 竖版 4961×7016 @600dpi）
svglite::svglite(file.path(plot_dir, "Figure 1.svg"), width = 8.27, height = 11.69)
print(ppp1); dev.off()
png(file.path(plot_dir, "Figure 1.png"), width = 4961, height = 4961 * (2^0.5), res = 600)
print(ppp1); dev.off()
# 再把 .svg gzip -9 成 Figure 1.svgz（Figure/ 里只留 .svgz 与 .png）
```

要点：
- `labels = "AUTO"` 自动按序生成 A/B/C…；需要跳过某个位置就传 `c("", "B")`。
- `label_x` / `label_y` 控制字母相对 panel 的偏移；**负的 `label_x` 把字母推到 panel 左侧外侧**。
- `rel_heights` / `rel_widths` 决定占比——把字数多的小图给更大比例。
- 输出**同时**落矢量（`svglite::svglite`）与预览（`png`），尺寸按 **A4 竖版 4961×7016 @600 dpi**（`height = width * sqrt(2)`），与 02a 主路线口径一致；`.svg` 再 gzip 成 `.svgz`。

## 3. 读外部 PNG 面板

第三方工具（如 ChimeraX 渲染的 3D 图、在线服务导出的图）产出的位图，用 `readPNG` + `draw_image` 包成 ggplot 对象即可参与组图：

```r
read_png_as_plot <- function(path) {
  img <- readPNG(path)
  ggdraw() + draw_image(img, x = 0, y = 0, width = 1, height = 1)
}
```

> 注意：这条路要求 R 环境有 **`magick`** 包（cowplot 画位图依赖它）。缺包时 `draw_image` 会静默不画，输出空白图——务必检查输出非空白。
> 若安装不了 `magick`，可改用 Python + PIL 做等价拼接（见 `references/05_checklist.md` 的替代实现）。

## 4. 字体

- 统一 `serif`（Times / DejaVu Serif 系）。
- R 侧显式指定 `label_fontfamily = "serif"`；Python 侧用 `ImageFont.truetype("<path>/DejaVuSerif.ttf", size)`。
- **不加粗**：用常规体（`DejaVuSerif.ttf`，非 `-Bold.ttf`）。

## 5. 多面板自动排版（多构象/多组）

当一个分析产出 N 个同构小图（如每个细胞类型一张），写循环函数而不是手写 N 段：

```r
plot_grid2 <- function(input_dir, group, ppp_ncol, label_size,
                       p_width, p_height, output_dir, p_name) {
  p_name_list <- list.files(input_dir, pattern = paste0("^", group, ".*.qs2$"))
  ...
  ppp <- plot_grid(plotlist = pp_list, ncol = ppp_ncol,
                   labels = "AUTO",
                   label_fontfamily = "serif", label_size = label_size,
                   label_x = -0.04, label_y = 1)
  svglite::svglite(file.path(output_dir, paste0(p_name, ".svg")),
                   width = p_width / 600, height = p_height / 600)
  print(ppp); dev.off()
  png(file.path(output_dir, paste0(p_name, ".png")),
      width = p_width, height = p_height, res = 600)
  print(ppp); dev.off()
}
```

好处：新增一个分组只改调用参数；版式自动一致。

## 6. 运行环境

- R 用 micromamba 环境（本机项目用 `r453`：`micromamba run -n r453 Rscript script.R`）。
- R 启动若访问外网（如 Bioconductor 配置）失败，加 `--vanilla` 跳过 profile：
  ```bash
  micromamba run -n r453 Rscript --vanilla assemble.R
  ```
