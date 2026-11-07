---
name: general-figure-guide
description: >-
  Use when producing or assembling publication figures for a manuscript — first
  deciding where each panel comes from (prefer the analysis package's own
  plotting function: sc.pl.umap / Seurat::DimPlot / DESeq2::plotMA /
  ComplexHeatmap::Heatmap …, hand-drawing only as a recorded exception), then
  canvas size, panel letters, fonts, legends, colors, which panels to merge into
  one figure, and how to name/insert captions into the manuscript. Covers the
  "results-only", "fill" and "semantic" grouping principles, the A4 canvas
  convention, the paired-output rule (products `.svgz`+`.png`, panels
  `.svg`+`.png`), embedding non-vectorizable panels as 600-dpi `<image>`, the
  small non-bold serif panel letters, legends-outside-axes rule, the
  `svg_grid`-first assembly route, and the manuscript integration format
  (Figure x / Figure Sx naming, md insertion, caption format). Triggers for
  "原生出图", "native plot", "组图", "figure assembly", "panel layout",
  "figure caption", "Figure 1/S1 naming", "panel letters", "compose a figure".
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Glob
  - Grep
metadata: {"version": "1.2", "skill-author": "BioAgentForge"}
---

# 通用组图指南（manuscript figure assembly）

> 本文件是**主干**：工作流 + 每一步的判据 + 索引。规则的依据、阈值、代码范式在 `references/`。

# CORE WORKFLOW

1. **确认 panel 来源（原生出图优先）** — 先查一轮官方资料：认包 → 查本仓 `tools/` → 查官方 docs/API → 确认无才自绘；有原生函数就**必须**用它出图。
2. **按目标格子尺寸出矢量 panel** — 每个 panel 单独出一份 `.svg` + `.png`，尺寸按它将要占的格子反算。
3. **组图（svg_grid 优先）** — 文件级矢量组图；三条判据之一成立才回退 cowplot。
4. **视觉检查（vision 必须真看成品）** — 程序化检查只是下限，必须真的打开成品图看。
5. **交付核对** — 过自查清单；命名、落盘、图注、正文引用四处对齐。

## ① 确认 panel 来源（原生出图优先）

**先查一轮官方资料，再决定要不要自己画**——四步、顺序不可颠倒（判据、对照表、例外、留痕格式全在
`references/00_native_plots.md`；**那里的表只是快捷答案，表里没有就按流程查，不许以"表里没写"为由自绘**）：

- **认包** — 这条结果由哪个包 / 工具（含版本）产生？看该 step 脚本的 `library()` / `import` 与结果对象类型
  （`AnnData` / `Seurat` / `DGEList`…），**不凭"图长得像什么"猜**。
- **查本仓 `tools/`** — 已收录工具/模型的 `SKILL.md` + `references/` 就是**官方资料的第一处**，命中就按它的出图入口走：
  ```bash
  grep -ril "<包名或工具名>" tools/*/ tools/*/*/ --include=SKILL.md --include=*.md
  ```
- **查官方资料** — 本地自省 > 官方仓库/文档/tutorial/API；**不采纳博客/转载/二手**：
  ```bash
  micromamba run -n python-bio python -c "import scanpy as sc; print([n for n in dir(sc.pl) if not n.startswith('_')])"
  ```
- **确认无才自绘** — 两处都查不到，才用该领域的**通用绘图库图层**（ggplot2 / seaborn），
  **不许手搓坐标、图元、色彩映射**。

**留痕（硬规则）**：每个 panel 一行——用了原生函数写 `# native: <包>::<函数>`；绕过了写
`# native: none — <查证痕迹>`，**痕迹必须含两处（`tools/` 命中了什么 + 官方查到了什么）**，只写"没有"不算；
例外清单见 `00_native_plots.md` §3，格式与对账见 §4 与 `05_checklist.md` §A0。

**只展示结果（原则 1）**——手稿写了什么，图就展示什么。这里的"过程"专指**中途走错、被迫返工**的痕迹
（修正前/后对比、作废批次、走死的中间尝试）——它不进图、也不进正文，因为对后文论证没有帮助、只会让叙事卡壳。
注意：**质控（QC）产出不算"过程"**，它属于该分析的结果（如去卷积参考模型 QC、训练历史、空间 QC 图），
照常当结果出图。
**边界（务必分清）**：被禁的"过程量"**仅指同一条分析走错后被迫返工**留下的痕迹。**"分析间对照"是结果本身**
——如"未校正 vs 校正混杂后""全队列 vs 分层内部""主分析 vs 匹配子集重跑"——**照常出图**；
尤其当结论是**混杂 / 阴性**时，那张图就是**负结论的证据**，不画等于把证据删掉。出图前先读手稿对应节。

> 出图的判断标准：**有价值的信息，且直面数据不直观** → 出图。同时对照本领域惯例补齐常规必备图
> （见 `tools/molecular-docking/figure_guide/` 这类领域指南）。

## ② 按目标格子尺寸出矢量 panel

**每张图都必须落盘成对文件（硬规则）**：**成品图**（正图与补充图）同名 **`.svgz` + `.png`**；
**散图 panel** 同名 **`.svg` + `.png`**。

- **`.png`**：供 agent 与用户**快速预览**、供正文 markdown 嵌入（渲染器对 svg 支持不一致）。
- **`.svgz`（成品）**：gzip 压缩的 SVG——成品图的矢量原件。比未压缩 SVG 小得多，且 `svg_grid` 能直接把它当输入读
  （按扩展名 gzip 解压）。需要交一份**未压缩** `.svg` 给外部工具或投稿系统时，把组图命令里的 `--output-svgz`
  换成 `--output` 重跑即可——组图是确定性的，原件随时可重建。
- **`.svg`（散图）**：供**组图与再排版**——矢量可任意缩放而文字/线条/标记不变形，png 做不到
  （放大会糊、缩了字会小到不可读）；`svg_grid convert` 直接吃 `.svg`。
- 两者**同名同目录**：成品如 `Figure/main/Figure 3.svgz` + `Figure 3.png`；散图如 `<step>/figure_panels/xxx.svg` + `xxx.png`。

**按它将要占的格子出图**（这一步做对，组图端就不用缩放）：`figsize` 与所有 pt 尺寸一起乘
`k = 目标格 px ÷ 面板画布 px`；同一行的面板先画成**同一画布尺寸**。推导与逐面板归一化的做法见
`references/02a_svg_grid_assembly.md` 的「让面板落在它将要占据的尺度上」。

```python
import matplotlib; matplotlib.rcParams["svg.fonttype"] = "none"   # 保留 <text>，别烘成轮廓
ax = sc.pl.umap(adata, color="leiden", show=False, frameon=False)
ax.figure.savefig(f"{out}/A.svg", format="svg"); ax.figure.savefig(f"{out}/A.png", dpi=600)
```

```r
ggplot2::ggsave(file.path(out, "A.svg"), Seurat::DimPlot(obj, reduction = "umap"),
                device = svglite::svglite, width = 2180/600, height = 1500/600)   # 尺寸 = 格 px/600
```

**无法矢量化的面板**（ChimeraX 3D 渲染、RDKit `MolDraw2DCairo` 结构图、`imshow` 热图等）以**内嵌 `<image>`**
的形式进成品 svgz，**保持 600 dpi 原分辨率、不降采样**：重采样只会让图更糊，而 gzip 也压不动 base64 位图。

## ③ 组图（svg_grid 优先）

**默认走 `references/02a_svg_grid_assembly.md`（`svg_grid`）**——文件级、跨语言、保留矢量。
**全 R、同会话也一样**：面板先各自落成 `.svg`，再交给 `svg_grid` 组。

```bash
svg_grid convert --input panels/A.svg --output tagged/A.svg
svg_grid plot-grid --input tagged/A.svg --input tagged/B.svg --output row1.svg --width 4961 --ncol 2 --labels A,B
```

**只有下面三条之一成立时才回退 `references/02b_cowplot_assembly.md`**（R + cowplot）：

1. 转换器对面板 **fail-closed**，且无法改用受支持的方言（`svglite` / matplotlib / RDKit 矢量后端）重出。
   仍 fail-closed 的**构造性**清单与回退代价由 `02b` §0 持有，本指南不复述。
2. `svg_grid`（含 `convert` 子命令）**不可用**（没编译、环境不支持、离线构建失败）；
3. 面板**拿不到独立 `.svg`**，只能以会话内对象（ggplot/grob）组合。

回退路线**必须**检查输出非空白——cowplot 在本机实测过静默产出全白图。
`plot-grid` 跑完**先看 stderr**（告警一律非致命、只说"发生了什么/为什么/怎么修"）。

**组什么图（原则 2、3）**：

- **饱满原则**——同一步骤/同一逻辑块的内容**能组到一张图就组一张**，不要散落成多张小图。
  内容量大的 panel（如逐基因热图）保留足够画布占比，避免缩小后小字不可读。
- **语义原则**——**不同步骤/不同分析环节/叙事逻辑不相邻**的内容不组进同一张图。合并次序遵循论文叙事顺序。

面板字母由组图层统一加（位置/字体/字重见 `01_canvas_layout.md` §2，`--labels` 该给哪一层见 `02a` 硬规则表）：
**只为"拥有该面板作为直接输入"的那一次调用加字母，组合图本身绝不加**。

## ④ 视觉检查（vision 必须真看成品）

**程序化检查（下限）**——判据只写在 `tools/svg_grid/scripts/check_deliverable.py`，`--help` 即契约：

```bash
~/.venv/bin/python tools/svg_grid/scripts/check_deliverable.py <成品.png> --margin <本图预算> [--gap] [--label-band N]
```

```bash
~/.venv/bin/python tools/svg_grid/scripts/check_deliverable.py --selftest    # 判据自证（正/负向对照）
```

退出码即结论。脚本**不覆盖**的项（panel 包围盒互不相交、文字/图例两两无重叠、字母字号占比、字重）按图自己量，
做法见 `01_canvas_layout.md`；判据的机制说明见 `05_checklist.md` C 节。

**vision 目视（硬步骤，不能替代）**——用 vision 工具打开**成品图**肉眼判断（不看中间 panel）：
版式是否平衡、有无显眼空白（大片死白 / 某 panel 明显压过同侪）、panel 是否被挤扁或缩得不可读、
字换算到纸面尺寸后能否读。

- 上面两项**都要做**：程序化检查是下限，vision 才能发现"内容到了边缘""越栏压叠""字小但没触边"这类问题。
- **标题在坐标轴上方是可以的**；禁止的 panel 内标题仅指落在 axes 数据区**内部**、可能与数据打架的标题。
- 查到空白/失衡：通常**把 panel 放大填满**（或重排行），而不是补更多白；**补充图不必守 A4 比例**，
  可裁画布贴合内容把页面填满。
- 子代理会话的 vision 可能不可用（无视输入固定返回一张无关图）：此时在报告里**明说**，
  把视觉检查交给 vision 可用的会话（主 agent）完成，**不得默默跳过，也不得谎称已做视觉检查**。

## ⑤ 交付核对

逐条过 `references/05_checklist.md`（内容 / 版式 / 程序化 / 落盘 / 观感五项；**原生出图与例外留痕是它的头条**）。
交付前最常漏的三处一致性：

1. `Figure/main|supp/` 里的**文件名**（编号连续、与正文引用一致、png 与矢量成对不缺）；
2. 正文里的 **`Figure xA` / `Figure Sx` 引用**；
3. `supp_captions.md` 的**图注编号与 panel 说明**。

改过 panel 顺序/编号就全文 grep 一遍同步（真实教训：删掉一张补充图后 `supp_captions.md` 仍残留死链）。

### 快速规则表

| 项 | 规则 | 详见 |
|---|---|---|
| panel 来源 | **分析包的原生出图函数优先**（`sc.pl.umap` / `DimPlot` / `plotMA` / `Heatmap`…）；手绘属例外，必须 `# native: none — <原因>` 留痕 | `references/00_native_plots.md` |
| 输出格式 | **成品 `.svgz` + `.png`**（svgz = 压缩 SVG 原件）；**散图 `.svg` + `.png`**；同名同目录 | `references/04_manuscript_integration.md` |
| 画布 | **正图 A4 比例**（4961×7016 px @600dpi）；**补充图可松懈**；重排版面时宽高比保持在 **1:1 ↔ A4** 之间，不要极端 | `references/01_canvas_layout.md` |
| 面板字母 | A/B/C/D 置于**组图左上角外侧**（不画进 panel 内）；**serif 常规体、不加粗**；占画布宽 ~1.8–2.2 % | `references/01_canvas_layout.md` |
| 字号 | 整体偏小（以已发表图为参照，而非"填满画布"）；换算到纸面 ≥ 5–6 pt | `references/01_canvas_layout.md` |
| 图例 | 放**坐标轴右侧外侧**，不放图内（图内易与数据/标签重叠）；**窄格要先算右外距**，否则被右缘静默裁 | `references/01_canvas_layout.md` |
| 防重叠 | 长 y 轴标签预留边距或改共享轴，**不得触左缘、不得侵入相邻 panel**；多列图先划列，文字越栏**死白带查不到**、须做 bbox 两两检查 | `references/01_canvas_layout.md` |
| 配色 | 现代期刊风、colorblind-safe、低饱和互补；避免通篇灰冷暗沉的"老气"感 | `references/03_color.md` |
| 代码组图 | **默认 `svg_grid`**（`plot-grid`，文件级、保留矢量）；**走不通才回退 cowplot**；serif 字体 | `references/02a_svg_grid_assembly.md`（首选）、`references/02b_cowplot_assembly.md`（回退） |
| 命名/存放 | 正图 `Figure x`、补充 `Figure Sx`（**含空格**）；扁平存放 | `references/04_manuscript_integration.md` |
| 正文插入 | md 图片语法（引用 png）；图与图注放在**本节文字下方**（不是上方） | `references/04_manuscript_integration.md` |
| 图注 | `Figure x. <标题>. (A) xxx. (B) xxx.`；**补充图图注不写正文**，统一放 `Figure/supp/supp_captions.md` | `references/04_manuscript_integration.md` |
| 视觉检查 | 组好图后**必须**用 vision 工具打开**成品图**肉眼判断；**程序化检查不能替代**，两者都做 | `references/05_checklist.md` |
| 交付前 | 过 `references/05_checklist.md` 自查清单 | `references/05_checklist.md` |

### 文档索引

| 文档 | 内容 |
|---|---|
| `references/00_native_plots.md` | **panel 从哪来**：分析→原生出图函数对照表（单细胞/bulk/热图/突变/网络/统计图层）、手绘例外的判据、`# native:` 留痕、出图端最小代码 |
| `references/01_canvas_layout.md` | 画布比例与 dpi、面板字母的位置/字体/尺寸实测值、字号、间距、防重叠 |
| `references/02a_svg_grid_assembly.md` | **首选**：`svg_grid` 文件级矢量组图——转换器流水线、格子比例约束、面板按目标格子尺寸出图、svg+位图混合、跑完必须读 stderr、交付前核查 |
| `references/02b_cowplot_assembly.md` | **回退**：R + cowplot 对象级组图代码范式（仅当 02a 的三条判据成立） |
| `references/03_color.md` | 配色原则与可用色板 |
| `references/04_manuscript_integration.md` | png+svg 成对输出、命名规范、目录结构、正文插入位置、图注格式（正图 vs 补充图） |
| `references/05_checklist.md` | 交付前逐条自查 |

领域专用指南（本仓已有）：
- 分子对接与 MD：`tools/molecular-docking/figure_guide/`

### 反面清单（常见 AI 失误）

- ❌ **包明明有出图函数，却自己手写 UMAP / 热图 / 火山图**（`sc.pl.umap`/`DimPlot`/`DoHeatmap`/`plotMA` 就在手边）
  → 图型与原分析对不上（密度归一、簇配色、图层顺序都不同），且白造轮子；有原生函数就必须用，例外要 `# native: none — <原因>`
- ❌ **只出 png 不出矢量原件**（成品缺 `.svgz`、散图缺 `.svg`，或反之缺 png）→ 成对落盘，矢量是组图与投稿的基础
- ❌ 内嵌位图面板时降采样、降 dpi → 位图按 **600 dpi 原分辨率**内嵌
- ❌ **一上手就走 cowplot**（或一次转换失败就整体放弃 `svg_grid`）→ 默认 02a；回退只认 02a 的**三条判据**，且回退后必须检查输出非空白
- ❌ 图内自带脚注/图注/来源标注 → 读起来像"已出版 PDF 截图"，实际是组图阶段不该有的东西
- ❌ 面板字母过大/加粗/非 serif → 对照已发表的图缩小、去粗
- ❌ 画布比例随手定（非 A4）→ 正图必须 A4 比例
- ❌ 同一数据做两种可视化并列（如热图 + 柱状）→ 信息重复占版面，删其一
- ❌ 把过程量搬进图或正文（修正前/后对比、旧批次、走死的中间尝试）→ 违反"只展示结果"（**QC 图属结果，不在禁止之列**；**"分析间对照"——未校正 vs 校正、全队列 vs 分层、主分析 vs 匹配子集——也属结果，尤其阴性/混杂结论时照常出图**）
- ❌ 跨 step 的内容混在一张图 → 违反语义原则
- ❌ 图内文字与数据/图例重叠、y 标签越界 → 必须程序化检查边界
- ❌ 中文/日文/韩文面板不注册 `.ttc` 字体（或用了字体集里没有的符号）→ matplotlib 静默缺字；交付前把 `Glyph ... missing` 当失败
- ❌ 组图后只跑程序化检查就算过（或没看成品图却声称看过）→ 组图后**必须**真的用 vision 工具看成品图
