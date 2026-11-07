# panel 的来源：原生出图优先

> 主干第 ① 步的细则。**一句话**：panel 由**产生这条分析的那个包自己的出图函数**出；本指南只做视觉优化，不重造图型。
> **本表不是上限**——§2 的对照表只是**快捷答案**：表里没有的包 / 工具 / 版本，一律走 §0 的流程查证，**禁止以「表里没写」为由自绘**。

**反面**（这条规则要治的病）：`sc.pl.umap` / `Seurat::DimPlot` 就在手边，却自己拉 `adata.obsm["X_umap"]`、自己
`ax.scatter`、自己配色 —— 密度归一、图层顺序、缺失簇配色、`uns` 里的色彩映射、标签位置全与原分析对不上。

## 0. 查证流程（**本表不是上限**，四步有序）

**任何不在 §2 里的包 / 工具都走这四步；禁止以「表里没写」为由自绘。**

1. **认包**：这个结果由哪个包 / 工具（**含版本**）产生？证据 = 出图脚本的 `library()` / `import`、结果对象的类型
   （`AnnData` / `Seurat` / `DGEList` / `GRanges` / `SingleCellExperiment` …）、上游命令行工具名。
   ```bash
   grep -rnE "library\(|^import |^from .* import" <step>/figure_scripts/ | sort -u
   ```
2. **查本仓 `tools/`**：本仓已收录的工具/模型，其 `SKILL.md` + `references/` 就是**官方资料的第一处**；
   命中就按该条目的出图入口走，并**记下命中的路径**。
   ```bash
   ls tools/
   grep -ril "scanpy" tools/*/ tools/*/*/ --include=SKILL.md --include=*.md
   # 实测（2026-11-05，154 条命中，首条）：
   # tools/foundation-models/state/references/docs/10_仓库补充_pyproject与模型配置.md
   # —— 无 tools/scanpy 专用条目，命中只是别的模型文档在提它 ⇒ 继续第 3 步
   ```
3. **查官方资料**（顺序：**本地自省 > 官方仓库/文档/tutorial/API 参考**；**不采纳博客/转载/二手文章**）：
   ```bash
   micromamba run -n python-bio python -c "import scanpy as sc; print([n for n in dir(sc.pl) if not n.startswith('_')])"
   micromamba run -n r453 Rscript --vanilla -e 'print(getNamespaceExports("Seurat"))'
   # 实测（2026-11-05，55 个符号，首四项）：['Deprecation', 'DotPlot', 'MatrixPlot', 'StackedViolin', … 'violin']
   ```
4. **确认没有才自绘**：用该领域的**通用绘图库图层**（ggplot2 / seaborn / matplotlib 的图层），
   **不手搓坐标与图元**（§2.6、§3）。

**留痕**（逐 panel 一行，紧跟出图代码前；写法与对账见 §4）：`# native: <包>::<函数>`，或

```
# native: none — 查了 tools/<xx>（无条目 / 命中 <路径> 但无出图）、官方 <版本或链接>（无对应函数）、
#                 dir(...) 见 <…>；结论：<为何仍需自绘>
```

**`none` 必须带两处查证痕迹（`tools/` + 官方资料）；只写「没有」不算。**

## 1. 判据（就是 §0 的四步，不另立一套）

顺序不可颠倒：**先在第 3 步的包里找到函数，再考虑手绘**。①包里有 → **必须用它**（参数不够先试该包的扩展
函数/参数，见 §3）；②`tools/` 条目给了出图入口 → 按它走；③两处都确认为无 → §0 第 4 步的通用图层。
**§2 是快捷答案，不是白名单**：表里没写的包/工具照样回 §0 走一遍。

## 2. 分析 → 原生出图函数对照表（快捷答案）

### 2.1 单细胞（嵌入 / 表达 / 聚类 / 轨迹 / 通讯 / 空间）

| 图型 | 原生函数 |
|---|---|
| UMAP · t-SNE · PCA 嵌入（按簇、按基因上色） | `sc.pl.umap`、`sc.pl.tsne`、`sc.pl.pca`、`Seurat::DimPlot`、`Seurat::FeaturePlot` |
| 嵌入密度（细胞密度等高线） | `sc.pl.embedding_density` |
| 簇 × 基因点图（表达比例 + 均值） | `sc.pl.dotplot`、`Seurat::DotPlot` |
| 簇 × 基因堆叠小提琴 / 矩阵图 | `sc.pl.stacked_violin`、`sc.pl.matrixplot` |
| 单基因小提琴图 / 嵌入散点 | `sc.pl.violin`、`sc.pl.scatter`、`Seurat::VlnPlot`、`Seurat::RidgePlot` |
| 差异基因热图 | `sc.pl.heatmap`、`sc.pl.rank_genes_groups_dotplot`、`Seurat::DoHeatmap`、`Seurat::DimHeatmap` |
| 高变基因 / PCA 方差诊断 | `sc.pl.highly_variable_genes`、`Seurat::ElbowPlot`、`Seurat::JackStrawPlot` |
| 拓扑（PAGA / 树状 / 簇相关矩阵） | `sc.pl.paga`、`sc.pl.dendrogram`、`sc.pl.correlation_matrix` |
| 拟时序轨迹 | `monocle3::plot_cells`、`monocle3::plot_genes_in_pseudotime`、`monocle3::plot_genes_by_group` |
| RNA velocity | `scvelo.pl.velocity_embedding`、`scvelo.pl.velocity_embedding_grid`、`scvelo.pl.velocity_embedding_stream` |
| 细胞通讯（权重/通路/差异网络） | `CellChat::netVisual_circle`、`netVisual_chord_cell`、`netVisual_aggregate`、`netVisual_heatmap`、`netVisual_bubble`、`netVisual_diffInteraction` |
| 空间转录组（切片坐标 + 邻域） | `squidpy.pl.spatial_scatter`、`squidpy.pl.spatial_segment`、`squidpy.pl.nhood_enrichment`、`squidpy.pl.centrality_scores`、`squidpy.pl.ripley`、`sc.pl.spatial`、`Seurat::SpatialDimPlot`、`Seurat::SpatialFeaturePlot` |

切片类图的坐标系必须来自包（手绘的对不上 `obsm["spatial"]` 的朝向与缩放）。

### 2.2 bulk 差异表达与富集

| 图型 | 原生函数 |
|---|---|
| MA 图（logFC vs 均值） | `DESeq2::plotMA`、`limma::plotMA` |
| 样本 PCA / MDS | `DESeq2::plotPCA`、`limma::plotMDS`、`edgeR::plotMDS.DGEList` |
| 离散度 / 均值-方差诊断 | `DESeq2::plotDispEsts`、`edgeR::plotBCV`、`edgeR::plotQLDisp`、`limma::plotSA` |
| 样本内差异（smear） | `edgeR::plotSmear` |
| GSEA 富集曲线 | `fgsea::plotEnrichment`（单条通路）、`enrichplot::gseaplot2`（多条对照） |
| ORA / GSEA 汇总（点图 · 网络 · 类别图） | `clusterProfiler::dotplot`、`clusterProfiler::cnetplot`、`clusterProfiler::emapplot` |
| 基因集打分的矩阵热图（GSVA / ssGSEA / limma-voom 的 logFC 矩阵） | `ComplexHeatmap::Heatmap`（见 §2.3） |
| 共表达模块（树 + 模块色 + TOM） | `WGCNA::plotDendroAndColors`、`WGCNA::TOMplot` |

### 2.3 复杂热图

| 图型 | 原生函数 |
|---|---|
| 带注释/劈分/多图层热图、行内嵌图 | `ComplexHeatmap::Heatmap` + `ComplexHeatmap::HeatmapAnnotation` + `ComplexHeatmap::draw` |
| 突变瀑布式矩阵 | `ComplexHeatmap::oncoPrint` |
| 快速聚类热图 | `pheatmap::pheatmap` |
| Py 侧聚类热图 / 相关矩阵 | `seaborn.clustermap`、`seaborn.heatmap` |

### 2.4 突变与 CNV

| 图型 | 原生函数 |
|---|---|
| MAF 瀑布图（样本 × 基因突变矩阵） | `maftools::oncoplot` |
| 突变密度 / kataegis 雨图 | `maftools::rainfallPlot` |
| CNV 散点 · 热图 · 模式图 | `cnvkit` 自带子命令 `cnvkit.py scatter` / `cnvkit.py heatmap` / `cnvkit.py diagram` |
| 多集合交集计数 | `ComplexHeatmap::UpSet` |

### 2.5 网络

| 图型 | 原生函数 |
|---|---|
| 通用网络图（节点/边/布局） | `igraph` 的 `plot(g, layout = …)`（泛型 `plot`，实体是 S3 方法 `plot.igraph`） |
| 分层布局 / 花式网络图 | `ggraph::ggraph` + `ggraph::geom_edge_link` + `ggraph::geom_node_point` + `ggraph::geom_node_text` |
| 细胞通讯网络 | 见 §2.1 的 `CellChat::netVisual_*` |

### 2.6 统计分布与其它二维图：用**图层**，不用基元

没有"包的内置出图函数"，但也**不是手搓**——用统计图层叠加：

- **R**：`ggplot2::geom_point`、`geom_col`、`geom_boxplot`、`geom_violin`、`geom_smooth`、`geom_tile`、`stat_summary`；
- **Python**：`seaborn.scatterplot`、`lineplot`、`boxplot`、`violinplot`、`histplot`、`kdeplot`、`ecdfplot`、
  `jointplot`、`pairplot`。

**不要**用 `matplotlib` 基元（`ax.scatter` / `ax.plot` / `ax.bar` / `ax.text` 手工摆点摆字）复刻上面的图型——
那是"手搓坐标"的入口；matplotlib 只用于**没有现成图层**的定制图（§3）。它仍是本指南允许的**矢量后端**
（`svg.fonttype="none"`）：后端与图型是两件事。

## 3. 例外：什么时候允许手绘（且必须留痕）

1. **包和 `tools/` 条目都没有该图型**——示意图、通路模型图、实验流程图、工作流框图。
2. **包的参数做不到所需叠加**——先试扩展函数/参数并**留下试过的证据**（`sc.pl.*(return_fig=True)` 拿对象再改、
   `Seurat` 返回 ggplot 可 `+ theme(...)`、`return_fig` 后的 `.fig`），确认做不到才手绘。
3. **必须逐像素复刻某张已发表图**——对齐原图图型，不属于"重造轮子"。
4. **数据源不是该包的对象**——手上只有 `data.frame` 的 logFC 表、跑包那步没留下对象：用 §2.6 的**通用图层**，
   不是手搓坐标与图元。

例外不是"自由发挥"：每条例外都要按 §4 留一行原因，且原因要能被读脚本的人复核（回到那次查询）。

## 4. 留痕（可机械检查）

**逐 panel 一行**，紧跟该 panel 的出图代码前：

```python
# native: sc.pl.umap                    （用了原生函数：写「包.函数」）
# native: none — 查了 tools/<xx>（无条目）、官方 scanpy 1.12 的 sc.pl（dir 55 个符号里无对应）、结论：无该图型
# native: none — 只有 logFC 表、无 DESeq2 对象（tools/ 与官方均无入口），改用 seaborn 图层
```

于是"有没有绕过原生出图、有没有查过"变成一行命令的事（判据与对账见 `05_checklist.md`）：

```bash
grep -rn "# native:" <step>/figure_scripts/
```

## 5. 原生出图 ≠ 最终形态

原生函数只管**图型对、数据对**；默认外观几乎一定不合期刊要求。出完照旧做视觉优化：按目标格子尺寸出图
（`02a` 的「让面板落在它将要占据的尺度上」）、画布与字号（`01_canvas_layout.md`）、配色（`03_color.md`）、
图例外置、panel 字母（`01` §2）、成对落盘 `.svg` + `.png`（`04_manuscript_integration.md`）。
**唯一不许动的是图型本身**——不许为了"好看"把它换成自己画的散点/柱状。

## 6. 出图端最小代码（落矢量，再交给 `02a` 的 `svg_grid` 组图）

**scanpy**（`svg.fonttype="none"` 才保留 `<text>`，否则字被烘成轮廓）：

```python
import matplotlib; matplotlib.rcParams["svg.fonttype"] = "none"
ax = sc.pl.umap(adata, color="leiden", show=False, frameon=False)   # 嵌入图返回 Axes
ax.figure.savefig(f"{out}/A.svg", format="svg")
ax.figure.savefig(f"{out}/A.png", dpi=600)
dp = sc.pl.dotplot(adata, genes, groupby="leiden", show=False, return_fig=True)  # 多面板对象
dp.savefig(f"{out}/B.svg")
```

**Seurat**（ggplot 对象交给 `svglite`；尺寸 = 目标格 px ÷ 600）：

```r
p <- Seurat::DimPlot(obj, reduction = "umap", label = TRUE)
ggplot2::ggsave(file.path(out, "A.svg"), p, device = svglite::svglite,
                width = 2180/600, height = 1500/600)
```

**ComplexHeatmap**（grid 图只能在设备里落盘）：

```r
svglite::svglite(file.path(out, "C.svg"), width = 2180/600, height = 1500/600)
ComplexHeatmap::draw(ht); dev.off()
```

落完就进组图（`02a`）：

```bash
svg_grid convert --input panels/A.svg --output tagged/A.svg
svg_grid plot-grid --input tagged/A.svg --input tagged/B.svg --output row1.svg --width 4961 --ncol 2 --labels A,B
```
