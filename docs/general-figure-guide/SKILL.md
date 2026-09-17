---
name: general-figure-guide
description: >-
  Use when assembling publication figures for a manuscript — deciding canvas
  size, panel letters, fonts, legends, colors, which panels to merge into one
  figure, and how to name/insert captions into the manuscript. Covers the
  "results-only", "fill" and "semantic" grouping principles, the A4 canvas
  convention, the png+svg paired-output rule, the small non-bold serif panel
  letters, legends-outside-axes rule, and the manuscript integration format
  (Figure x / Figure Sx naming, md insertion, caption format). Triggers for
  "组图", "figure assembly", "panel layout", "figure caption",
  "Figure 1/S1 naming", "panel letters", "compose a figure".
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Glob
  - Grep
metadata: {"version": "1.1", "skill-author": "BioAgentForge"}
---

# 通用组图指南（manuscript figure assembly）

> 本文件只放**规则与索引**。每条规则的依据、阈值、代码范式在 `references/` 下。

## 三条核心原则（先读这条，再动手）

1. **只展示结果**——手稿写了什么，图就展示什么。这里的"过程"专指**中途走错、被迫返工**的痕迹（修正前/后对比、作废批次、走死的中间尝试）——它不进图、也不进正文，因为对后文论证没有帮助、只会让叙事卡壳。注意：**质控（QC）产出不算"过程"**，它属于该分析的结果（如去卷积参考模型 QC、训练历史、空间 QC 图），照常当结果出图。出图前先读手稿对应节。
2. **饱满原则**——同一步骤/同一逻辑块的内容**能组到一张图就组一张**，不要散落成多张小图。内容量大的 panel（如逐基因热图）保留足够画布占比，避免缩小后小字不可读。
3. **语义原则**——**不同步骤/不同分析环节/叙事逻辑不相邻**的内容不组进同一张图。合并次序遵循论文叙事顺序。

> 出图的判断标准：**有价值的信息，且直面数据不直观** → 出图。同时对照本领域惯例补齐常规必备图（见 `tools/molecular-docking/figure_guide/` 这类领域指南）。

## 每张图都必须 png + svg 成对输出（硬规则）

**所有图——单张 panel、组好的成品图（正图与补充图）——都必须同时输出同名两份：`.png` 与 `.svg`。**

- **`.png`**：供 agent 与用户**快速预览**、供正文 markdown 嵌入。
- **`.svg`**：供**组图与再排版**——矢量可任意缩放而文字/线条/标记不变形，png 做不到（放大会糊、缩了字会小到不可读）。
- 两者**同名同目录**（如 `Figure 3.png` + `Figure 3.svg`），组图时按 svg 载入。

## 组图路线：先 `svg_grid`，走不通才回退 cowplot

**默认走 `references/02a_svg_grid_assembly.md`（`svg_grid`）**——文件级、跨语言、保留矢量。**全 R、同会话也一样**：面板先各自落成 `.svg`，再交给 `svg_grid` 组。

只有下面三条之一成立时才回退 `references/02b_cowplot_assembly.md`（R + cowplot）：

1. 转换器对面板方言 **fail-closed**（RDKit `MolDraw2DSVG`、Inkscape/Illustrator 导出等），且无法改用 `svglite`/matplotlib 重出；
2. `svg_grid` / `svg_grid_convert` **不可用**（没编译、环境不支持、离线构建失败）；
3. 面板**拿不到独立 `.svg`**，只能以会话内对象（ggplot/grob）组合。

回退路线**必须**检查输出非空白——cowplot 在本机实测过静默产出全白图。

## 快速规则表

| 项 | 规则 | 详见 |
|---|---|---|
| 输出格式 | **每张图同名成对输出 `.png` + `.svg`**（png 预览/嵌入，svg 组图）；扁平存放 | `references/04_manuscript_integration.md` |
| 画布 | **正图 A4 比例**（如 4961×7016 px @600dpi）；**补充图可松懈**，不必强拉 A4 | `references/01_canvas_layout.md` |
| 面板字母 | A/B/C/D 置于**组图左上角外侧**（不画进 panel 内）；**serif 常规体、不加粗**；占画布宽 ~1.8–2.2 % | `references/01_canvas_layout.md` |
| 字号 | 整体偏小（以已发表图为参照，而非"填满画布"） | `references/01_canvas_layout.md` |
| 图例 | 放**坐标轴右侧外侧**，不放图内（图内易与数据/标签重叠） | `references/01_canvas_layout.md` |
| 防重叠 | 长 y 轴标签预留边距或改共享轴，**不得侵入相邻 panel** | `references/01_canvas_layout.md` |
| 配色 | 现代期刊风、colorblind-safe、低饱和互补；避免通篇灰冷暗沉的"老气"感 | `references/03_color.md` |
| 代码组图 | **默认 `svg_grid`**（`plot-grid`，文件级、保留矢量）；**走不通才回退 cowplot**；serif 字体 | `references/02a_svg_grid_assembly.md`（首选）、`references/02b_cowplot_assembly.md`（回退） |
| 命名/存放 | 正图 `Figure x`、补充 `Figure Sx`（**含空格**）；扁平存放 | `references/04_manuscript_integration.md` |
| 正文插入 | md 图片语法（引用 png）；图与图注放在**本节文字下方**（不是上方） | `references/04_manuscript_integration.md` |
| 图注 | `Figure x. <标题>. (A) xxx. (B) xxx.`；**补充图图注不写正文**，统一放 `Figure/supp/supp_captions.md` | `references/04_manuscript_integration.md` |
| 视觉检查 | 组好图后**必须**用 vision 工具打开**成品图**肉眼判断（版式平衡、空白、panel 大小、字号可读）；**程序化检查不能替代**，两者都做 | `references/05_checklist.md` |
| 交付前 | 过 `references/05_checklist.md` 自查清单 | `references/05_checklist.md` |

## 文档索引

| 文档 | 内容 |
|---|---|
| `references/01_canvas_layout.md` | 画布比例与 dpi、面板字母的位置/字体/尺寸实测值、字号、间距、防重叠 |
| `references/02a_svg_grid_assembly.md` | **首选**：`svg_grid` 文件级矢量组图——转换器流水线、格子比例约束、面板按目标格子尺寸出图、svg+位图混合、跑完必须读 stderr、交付前核查 |
| `references/02b_cowplot_assembly.md` | **回退**：R + cowplot 对象级组图代码范式（仅当 02a 的三条判据成立） |
| `references/03_color.md` | 配色原则与可用色板 |
| `references/04_manuscript_integration.md` | png+svg 成对输出、命名规范、目录结构、正文插入位置、图注格式（正图 vs 补充图） |
| `references/05_checklist.md` | 交付前逐条自查 |

领域专用指南（本仓已有）：
- 分子对接与 MD：`tools/molecular-docking/figure_guide/`

## 反面清单（常见 AI 失误）

- ❌ **只出 png 不出 svg（或反之）** → 每张图必须同名成对输出两份，svg 是组图的唯一可用素材
- ❌ **一上手就走 cowplot**（或一次转换失败就整体放弃 `svg_grid`）→ 默认 02a；回退只认 02a 的**三条判据**，且回退后必须检查输出非空白
- ❌ 图内自带脚注/图注/来源标注 → 读起来像"已出版 PDF 截图"，实际是组图阶段不该有的东西
- ❌ 面板字母过大/加粗/非 serif → 对照已发表的图缩小、去粗
- ❌ 画布比例随手定（非 A4）→ 正图必须 A4 比例
- ❌ 同一数据做两种可视化并列（如热图 + 柱状）→ 信息重复占版面，删其一
- ❌ 把过程量搬进图或正文（修正前/后对比、旧批次、走死的中间尝试）→ 违反"只展示结果"（**QC 图属结果，不在禁止之列**）
- ❌ 跨 step 的内容混在一张图 → 违反语义原则
- ❌ 图内文字与数据/图例重叠、y 标签越界 → 必须程序化检查边界
- ❌ 组图后只跑程序化检查就算过（或没看成品图却声称看过）→ 组图后**必须**真的用 vision 工具看成品图
