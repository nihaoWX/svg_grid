# skill：`general-figure-guide`（怎么写它）

> 这个 skill 是 `svg_grid` 的**成品**（给 agent 与人读的说明书），**不是工作记录**。
> 本文件定"它该怎么写、各部分归谁、与代码怎么配合"——**这些规矩不写进 skill 本身**。

## 1. 写作规则（2026-09-28 R9 定）

1. **只写"现在该怎么做"**。变更叙述一律不进：`已修复` / `已支持` / `现在会被精确烘进` /
   `老写法…不再需要` / `（2026-09-20 起支持）` / `已现场实测收口` 这类句子，以及"历史上曾经失败"的
   完整成因——它们的家是 `../ARCHITECTURE.md` §已定型的决定 与账本（可 `query` 取回）。
   **理由**：读它的人需要的是当前规则，不是版本史；变更叙述正是它膨胀到 26 KB 的主因
   （账本里 `where` 直指它的 finding 有 15 条、跨 4 轮）。
2. **不罗列告警**。告警属于分支，写在代码里、真触发时由 stderr 返回——细则与配合契约见
   `warnings.md` §4。指南只留一句"跑完先读 stderr"。
3. **程序化判据只说"跑哪个脚本"**，不抄判据参数（判据的单一来源 = `scripts/check_deliverable.py`，
   机制见 `check-deliverable.md`）。
4. **领域特定的枚举要标注**（"本仓实例"/"本领域实例"），通用规则在前。
5. **薄入口 + 分主题**：`SKILL.md` 只放 `# CORE WORKFLOW`（主干，5 步）+ 与主干同名的 5 个章节 +
   快速规则表/文档索引/反面清单；细则落 `references/*.md`。
6. **体积口径**：每份文件 **≤ 20 KB**（与源文件同一口径，`20000 B` 从严）。
7. **`SKILL.md` 的 frontmatter/`description` 要能被"什么时候用"触发**（"组图"、"figure assembly"、
   "panel layout"、"figure caption"…），它是 skill 的入口契约。
8. **第一条永远是"东西从哪来"，而且是查证动作而不是查表**：**认包 → 查本仓 `tools/` → 查官方资料/网络 →
   确认无才自绘**（`references/00_native_plots.md` §0）。对照表只是**快捷答案、不是上限**（R20 立规则、R21 立流程）。
   规则并列、没有主干时 agent 会自己发明顺序，而它发明的顺序是"先画图型"。

## 2. 各部分归谁（owner 划分）

| 文件 | 拥有什么 | 不该出现什么 |
|---|---|---|
| `SKILL.md` | `# CORE WORKFLOW` 五步主干（①panel 来源=原生出图 ②按格子出矢量 panel ③组图 ④vision 检查 ⑤交付核对）+ **同名同序**的五个章节、成对落盘硬规则、组图路线与回退判据**摘要**、快速规则表、文档索引、反面清单 | 任何操作细节、告警清单、变更叙述 |
| `references/00_native_plots.md` | **panel 来源**：查证四步（认包 → 查 `tools/` → 查官方 → 确认无才自绘）+ 分析→原生函数对照（95 条，75 本机实测）+ 四类手绘例外 + `# native:` 留痕（`none` 必须带两处查证痕迹） | 组图规则（归 `02a`/`02b`）、画布与配色细节（归 `01`/`03`） |
| `references/01_canvas_layout.md` | 画布比例与 dpi、面板字母（位置/字体/尺寸实测值/字重判据）、字号与纸面 pt 换算、图例与防重叠、排布范式与格尺寸速查 | 工具用法（归 `02a`） |
| `references/02a_svg_grid_assembly.md` | **首选路线**：流水线、硬规则、尺度与归一化、嵌套拼装、位图混合 | 告警目录、交付前清单（归 `05`）、回退路线细节（归 `02b`） |
| `references/02b_cowplot_assembly.md` | **回退路线**：什么时候走、回退代价、R 代码范式。**完整 fail-closed 构造清单的唯一持有者** | — |
| `references/03_color.md` | 配色原则与色板 | — |
| `references/04_manuscript_integration.md` | 命名/存放/成对落盘、正文插入位置、图注格式、同步检查 | — |
| `references/05_checklist.md` | 交付前逐条自查（内容/版式/程序化/落盘/观感）、**vision 目视必做**、程序化判据"跑哪个脚本" | 判据参数细节（归脚本与 `check-deliverable.md`） |

领域指南与本指南分层：通用规则在本 skill；领域特定（如分子对接/ MD）在
`tools/molecular-docking/figure_guide/`，由本 skill 指过去。

## 3. 规范源与复制件（2026-09-28 用户裁决）

**规范源（唯一维护处）**：`tools/svg_grid/docs/general-figure-guide/` —— **任何改动只在这里做**。
**复制件**：`tools/manuscripts_skills/general-figure-guide/` —— 部署副本，改完规范源后**直接覆盖**：

```bash
rsync -a --delete tools/svg_grid/docs/general-figure-guide/ tools/manuscripts_skills/general-figure-guide/
diff -r tools/svg_grid/docs/general-figure-guide tools/manuscripts_skills/general-figure-guide   # 必须为空
```

- 复制件**永不单独编辑**（`svg_grid/README.md` 承诺"仓库自带这套 skill"，那份是给别的 skill 用的部署件）。
- `round-data` 里 `diff -r` 是一条断言（见 `R9`）。
- **账本路径统一**：早期 5 条 finding（`R2-F3/F4/F5`、`R4-F6/F14`）记在复制件路径下，现已用 `link` 组 **L6**
  与规范源路径关联 ⇒ `query` 任一条路径都能看到全部历史；**新 finding 一律记规范源路径**。
- ⚠️ 账本早期几轮的 `reason` 里把方向写反过（当时的看法），**不追改**——现行口径以本节为准。
