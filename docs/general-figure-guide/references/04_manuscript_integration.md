# 手稿集成：输出格式、命名、存放、插入与图注

## 1. 输出格式、命名与存放（硬约束）

| 项 | 规则 |
|---|---|
| 输出格式 | **成品：同名 `.svgz` + `.png`**；**散图 panel：同名 `.svg` + `.png`**（见下） |
| 正图命名 | `Figure 1` / `Figure 2` / …（**注意空格**） |
| 补充图命名 | `Figure S1` / `Figure S2` / … |
| 存放 | **扁平存放**，不建子目录：`Figure/main/`（正图）、`Figure/supp/`（补充图） |

### 1.1 落盘格式：成品 `.svgz` + `.png`，散图 `.svg` + `.png`

**成品图（正图与补充图）**落 `.svgz` + `.png`；**散图 panel** 落 `.svg` + `.png`。都要**同名同目录**。

| 格式 | 用在哪 | 用途 |
|---|---|---|
| `.png` | 两者 | 供 agent 与用户**快速预览**；供正文 markdown 嵌入（渲染器对 svg 支持不一致） |
| `.svgz` | **成品** | gzip 压缩的 SVG = 成品矢量原件：体积远小于未压缩 SVG，且 `svg_grid` 能直接读（按扩展名解压） |
| `.svg` | **散图** | 供**组图与再排版**——矢量可任意缩放，文字/线条/标记**不变形**；`svg_grid_convert` 直接吃 `.svg` |

- 组图时**按 svg 载入 panel**：默认用 **`svg_grid`**（`references/02a_svg_grid_assembly.md`）；仅当回退到 cowplot 时才用 `magick::image_read_svg()` / `rsvg` 读成 ggplot/grob（要检查输出非空白，见 `references/02b_cowplot_assembly.md`）。
- **需要未压缩 `.svg`**（交投稿系统或外部工具）时，把组图命令里的 `--output-svgz` 换成 `--output` 重跑一次即可——组图确定性、可随时重建，`Figure/` 里不必同时堆两份矢量。
- **无法矢量化的面板**（ChimeraX 3D 渲染、RDKit `MolDraw2DCairo`、`imshow` 热图）以内嵌 `<image>` 进成品，**保持 600 dpi 原分辨率、不降采样**。
- 单张 panel 也遵守成对落盘（panel 存各 step 的 `figure_panels/`）。

## 2. 目录结构

```
steps/08_手稿/
├── 01_methods.md
├── 02_results.md
├── Figure/
│   ├── main/            # 正图成品：Figure 1.png + Figure 1.svgz / Figure 2.png + Figure 2.svgz ...
│   └── supp/            # 补充图成品：Figure S1.png + Figure S1.svgz ...
│       └── supp_captions.md   # 所有补充图图注集中在此
└── Supplemantary-Materials/
```

**出图脚本与中间面板不进 `Figure/`**：
- 脚本 → 各 step 的 `figure_scripts/`
- 中间 panel（未组装的单张图）→ 各 step 的 `figure_panels/`（同样 png+svg 成对）
- `Figure/` 只放**组好的成品**

## 3. 正文插入位置

图片与图注放在**本节文字内容的下方**（不是上方）：

```markdown
## 3.2. 分子对接

（本节正文段落……引用 Figure 2A、Figure 2C 等）

![Figure 2](Figure/main/Figure%202.png)

Figure 2. 九个靶点的分子对接打分与结合模式. (A) 对接打分矩阵（9 靶点 × 4 化合物）. (B) 四个化合物结构. (C) …… . (D) …… .
```

要点：
- md 图片语法，路径相对本文件，**引用 `.png`**（预览用）；**空格转义为 `%20`**（`Figure%202.png`）。
- 同名 `.svg` 与 png 并列放在同目录，不进 md。
- 图片与图注之间空一行。
- 正文引用 panel 时用 `Figure 2A` / `Figure 2C` 形式；**panel 改名后要同步改正文引用**（真实教训：把总览从 C 移到补充图后，正文里的 `Figure 2C` 需改成 `Figure 2D`，补充图图注里的引用也要跟着改）。

## 4. 图注格式

**正图图注**（写在正文图片下方）：

```
Figure x. <这张图的标题>. (A) xxx. (B) xxx.
```

- 标题后跟句点；各 panel 用 `(A)` `(B)` 开头、句点分隔。
- 只说"这张图是什么"，不写方法学过程、不写来源脚本/路径。
- 中文手稿用中文描述即可。

**补充图图注**（**不写在正文**，统一放 `Figure/supp/supp_captions.md`）：

```markdown
# Supplementary Figure Captions

![Figure S1](../supp/Figure%20S1.png)

Figure S1. 11 核心基因筛选. (A) 共有药靶（57 个）与差异表达基因（1060 个）取交得 11 个核心基因. (B) ……

![Figure S2](../supp/Figure%20S2.png)

Figure S2. 对接打分稳健性. (A) …… . (B) ……
```

格式与正文一致：**先引用图片，再在下方写 `Figure Sx. ...`**。

## 5. 引用与正文同步检查

改图后必须检查三处一致：
1. `Figure/main|supp/` 里的**文件名**（编号连续、与正文引用的编号一致；**png 与 svg 成对不缺**）
2. 正文里的 **`Figure xA` / `Figure Sx` 引用**
3. `supp_captions.md` 里的**图注编号与 panel 说明**

> 真实教训：删除某张补充图后，`supp_captions.md` 仍残留指向已删文件的引用（死链）；panel 顺序调整后正文引用未同步（指向错误 panel）。改图后 grep 一遍编号。

## 6. 与 HARNESS 的衔接

- 图片产出后，`PROGRESS.md` 的 Current State 要反映"哪一节的图已完成"
- 组图规则若在本项目内被细化/推翻，写进项目 `GUIDE.md`（协作规则），通用部分回填本指南
