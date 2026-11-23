# Definition

## 任务定义

`tools/svg_grid/` 是一个**纯 SVG 面板组图器**（project-development 任务）。

- 核心能力：`svg_grid plot-grid` 把多个 SVG 面板按网格拼成一张成品 SVG/SVGZ，可选输出 PNG / AVIF。
- 它不是通用 SVG 工具，而是带**显式标签协议**的组图器：每个输入必须自带
  `<rect data-panel-box="main" .../>` 与 `data-scale` 分组，否则组图结果错误、或直接报错退出。
- 协议的设计目标：**面板几何随格子缩放，而文字大小、点径、线宽保持不变**——即 R `grid`/`cowplot`
  的 grob 语义，在 SVG 层面用标签显式表达。普通 SVG 没有这个语义：整组 `transform="scale(s)"`
  会把文字和点一起缩放。
- `svg_grid convert`（源码在 `src/convert/`）把**不带标签**的外源 SVG（R `svglite`/`ggplot2`、matplotlib、
  `pdftocairo -svg`）转成协议格式；`plot-grid` 遇到未打标签的输入会自动调它。

## 用户期望

1. **`src/` 保持纯净**：只做 SVG 组图，不引入栅格输入、去白边、混合栅格-矢量管线（2026-09-16 整体回退）。
2. **外源 SVG 的适配隔离在 `src/convert/`**：它是一个模块（R42 起；此前是独立 crate），
   与组图路径 `src/`（除 `src/convert/`）解耦，不进组图逻辑、不改组图依赖。
3. **失败必须响亮（fail-closed）**：转换器遇到无法处理的构造必须**报错退出并列出位置**，不许静默产出错位的图。
4. **先小批验证再铺开**：先跑通一张真实多面板图并与现有成品并排比对，再推广。
5. **版面靠 CLI 参数迭代**（`--width/--height/--ncol/--nrow/--plot-margin`），调完看成品再微调；
   **不做参数计算器**。`--rel-*` 只作微调（占比看格数，见 #13）。
6. 环境：Linux、命令行、可批处理、离线可构建。
7. **文档纪律**（R9；2026-10-06 随根 `AGENTS.md` 更新）：**skill 是成品、只写"现在该怎么做"**；
   **定义 / 结构 / 已定型的决定**写本文件，**机制**写 `ARCHITECTURE/`，**状态**写 `PROGRESS.md`，
   历史交给账本与 git。**每份文件 ≤ 20 KB**（见 §跨领域约定）。
8. **HARNESS 是两件套**（2026-10-06 用户裁决）：`ARCHITECTURE.md` + `ARCHITECTURE/` + `PROGRESS.md`。
   **不设 `DECISIONS.md`**——它与 `ARCHITECTURE` 本质重复，且 agent 会把两者的内容混记；
   其中仍有价值的部分已迁入本文件 `# 已定型的决定`（原来那份见 git）。
9. **产品定义以 `INSTRUCTIONS.md` 为准**（用户手写、只由用户改）：`svg_grid` 是**给 agent 用的通用组图库**
   （不是 skill），`docs/general-figure-guide/` 是依赖它的 skill。CLI 逻辑模仿 R `cowplot`。
10. **单次调用是一个二维网格，且"尽量填满"**（2026-11-22 用户明确）：一次 `plot-grid` 在 `rows × cols`
    网格里放图，**不支持跨格**；"填满"= 除声明过的 `NULL`/垫片外不许空出一格。
    - 不给 `ncol`/`nrow`：抄 cowplot 的 `cols=ceil(sqrt(n))`、`rows=ceil(n/cols)`（15→4×4，5→3×2），
      末格按 NULL 声明处理（= 经典例子 2：`ABC / DE+NULL`）。
    - 显式给了 `ncol`/`nrow` 但 `rows×cols > 输入数`：多出的格必须显式 `--input NULL` 声明（详见 `PROGRESS.md` #5）。
    - 经典例子 1（`ABC` 行 + `DE` 行，两块等宽、无 NULL）靠**嵌套最小步骤**；`NULL` 只是占位声明（画面仍空）。
11. **`convert` 必须自动**（2026-11-20 用户明确）：`plot-grid` 遇到未打标签的输入应**自己转换**，
    不要求调用方先跑 `convert`；`convert` 也不许损坏原图（字形孔等必须保真）。
12. **`--labels` / `--plot-margin` 都按调用层生效**：在**拥有该散图/该块的那一层**设置，外层改不了里层。
    字母与 NULL：裸 `--labels` 的 AUTO **跳过 NULL 槽**；显式列表**按槽位**（含 NULL）——给 NULL 槽写了字母就画，
    写 `""` 就跳过。`--plot-margin` 默认 24、每层都加、允许累积。
13. **"占比"= 该散图在最终图里占几格，不是 `--rel-*`**：占比由出图端的画布尺寸/纵横比决定（agent 按目标格子出图）；`--rel-*` 只作微调。

# 已定型的决定

> 「决定」与「定义」本来就长在一起——**不设独立的决定文件**（见用户期望 #8）。这里**只留「为什么」与指针**，
> 细则在各目的唯一来源；**逐轮过程与被推翻的旧方案不在这里**（git 与账本里）。
> 2026-10-06 前这些条目记在 `DECISIONS.md`，本表是它的迁移结果（合并/去重后）。

| 决定 | 为什么 | 细则在 |
|---|---|---|
| 主库只做 SVG 组图：**不收栅格输入、不去白边、不做混合栅格-矢量管线**；`adapters/`（MCP 风格）一并删 | 用户判定那批内容已无用处、且不希望后续工作被干扰（2026-09-16 整体回退 `98fd91e`） | 用户期望 #1；`ARCHITECTURE/tag-protocol.md` |
| 外源 SVG 的适配**隔离在 `src/convert/` 模块**；主库只多收"被画出来的图元"的**几何搬运** | 主库保持纯净；适配逻辑属外围、目标方言会变。R5 起主库纳入 `<path>`（其判据＝"最小的、完备的 SVG 表达能力"），但**改语义**的构造仍只在 `convert/` | 用户期望 #2；`ARCHITECTURE/tag-protocol.md` §3/§4 |
| **失败分层**：结构错 fail-closed（报错 + 位置）；数值/属性畸形 ⇒ 警告 + 原样保留 | 用户不信任静默错误的产物（cowplot 实测静默产出全白图）；浏览器对轻微畸形宽容，严格解析会把旧输入变成整图 `exit 1` | 用户期望 #3；`tag-protocol.md` §5 |
| **不做 `--strict`**：告警一律非致命、不改退出码、不新增把越界变硬错误的开关 | 硬错误会打断既有流水线；判据取向是"**宁可漏报，不要误报**、正常路径零输出" | `ARCHITECTURE/warnings.md` |
| 版面语义以用户既有 R 脚本 `plot_grid.R` 为**权威范本**（"注意是逻辑"）；**不做网格参数计算器** | 本工具的设计初衷就是复刻那套组图逻辑；且布局已按自然尺寸自动算，不需要调用方手算参数 | 用户期望 #5；`ARCHITECTURE/layout.md` §3/§5 |
| `--height` 可省（`auto`），且**该值即输出画布**——不做"逻辑尺寸 ≠ 输出尺寸"的分裂 | 用户要求"**输入什么就输出什么**"；双义的参数只会让 agent 困惑 | `ARCHITECTURE/layout.md` §1 |
| **面板在出图端就按目标格子尺寸绘制**；组图端只做纯坐标搬运 | 协议永不缩放 `font-size` ⇒ 组图后文字相对大小 = 设计相对大小 ÷ `s`（S22 实测 `s=4.8` 时只剩 13–21%，不可读） | `layout.md` §4 |
| 字形 / paint **由转换器按 paint 语义定型**（不再推给调用方）；paint 继承仍由渲染器负责 | 要求每个面板脚本自带规避，等于把同一段代码复制到每个调用方；而"按几何闭合性判形态"正是字形孔被填实、文字被判成描边的根因 | `ARCHITECTURE/convert.md` §3 |
| resvg **必须启用 `raster-images`**；不重新引入已回退的 compose 管线，也不加 `image`/`png` 直接依赖 | 缺该 feature 时 resvg 在该分支**直接放弃解码**（只打一条 `log::warn!`）⇒ 成品 PNG 里内嵌的 `<image>` **静默变白** | `Cargo.toml`；`ARCHITECTURE/convert.md` |
| **体积口径 + 入口薄**：每个文件 ≤ 20 KB（20000 B 从严）；`lib.rs`/`main.rs`/薄壳脚本只做声明、再导出、调用；拆分**用 `mod` 不用 `include!`** | 大到要整份载入就不可用；`include!` 会让文件脱离 rustfmt 门禁、且 `cargo fmt` **谎报干净**（实测） | 跨领域约定 #1 |
| **skill 是成品，只写"现在该怎么做"**；告警属分支 ⇒ 写进代码、真触发时由 stderr 返回 | 用户规则：变更叙述是 git 的活，写进 skill 只会让它膨胀；告警的种类会随实现变 | 用户期望 #7；`ARCHITECTURE/general-figure-guide.md` |
| **指南规范源 = `tools/svg_grid/docs/general-figure-guide/`**；`tools/manuscripts_skills/` 那份是复制件 | 用户 2026-09-28 裁决：只在这边维护，改完用 `rsync -a --delete` 覆盖过去（`diff -r` 必须为空） | `ARCHITECTURE/general-figure-guide.md` §3 |
| 账本 / round-data 的读写契约：`record` 时 `--repo` 必须显式给、改名/移动/拆分后 `link`、复发轮必留可跑证据 | 见 `tools/pitfalls-manager`（工具）；账本由它管理、按项目目录定位（**不写具体位置**） | 跨领域约定 #4 |
| **字体来源唯一 = `fontlib/`**（代码里没有 `load_system_fonts`，resvg 的 `system-fonts` feature 已去掉）；面板字母默认族 **Arial**；缺族 W8、空库 fail-closed | 同一份资产换台机器/CI 就会出现方字与换行差异；更要紧的是族名写错会**静默回退**（实测：不存在的族名与 `serif` 逐像素相同、无任何告警） | 用户期望；`ARCHITECTURE/fonts.md` |
| **改完代码必须重跑 `install.sh`**：命令以 `~/.local/bin` 的 release 副本为准，字体进数据根 `~/svg_grid/fontlib` | 以前没有装配步骤，文档里的 `svg_grid …` 在 PATH 里根本不存在；靠人记得"改了要装"必然出事 | `ARCHITECTURE/install.md` |
| **CLI 契约**：退出码分档 0/1/2/3；`-h/--help` 在取值前特判（stdout、rc=0）；帮助文本覆盖全部 flag；用户输入只走 `Result`，不许 panic | panic（`rc=101`）会被调用方读成"工具坏了"；`--help` 坏了会逼着把参数语义抄进 skill | `ARCHITECTURE/cli.md` |
| **测试语料 = 真实绘图包的产物**（ggplot2 / matplotlib / seaborn，按图型穷举），不用手写 SVG；**`doctor` 是默认回归入口** | 手写 SVG 只能覆盖"我们想到的"；真实语料抓到了 paint server 被丢弃这类只有"看墨水/看悬空引用"才能发现的缺陷 | `ARCHITECTURE/testing.md` |
| **告警里必须有"引用完整性"这一关（W9）**：`url(#x)` 的 `x` 不在定义集里 ⇒ 出声 | 结构性丢失（defs 被丢、id 被改名）不会让 rc 变非零，只有把"定义集 vs 引用集"对一遍才看得见 | `ARCHITECTURE/warnings.md`、`round-data/R16` |

# Structure

## 文件布局

```
tools/svg_grid/
├── Cargo.toml / Cargo.lock     # 主库 crate
├── INSTRUCTIONS.md             # **用户手写、只由用户改**：产品概念 / 期望用法 / 目录规划（与其余文件冲突时以它为准）
├── ARCHITECTURE.md             # 本文件：定义 + 已定型的决定 + 布局 + 跨领域约定 + ARCHITECTURE/ 索引
├── ARCHITECTURE/               # 机制细节，一个子项一份（见下表；每个文件 ≤ 20 KB）
│   ├── tag-protocol.md         # 输入契约、图元、与 convert 的分界原则、<path> 保真边界
│   ├── layout.md               # 布局语义、几何约束、设计参照（R plot_grid 对齐表）
│   ├── convert.md              # 翻译手段、fail-closed 清单、<path> 出形态决策表、幂等
│   ├── warnings.md             # 告警契约、判据取向、W1–W9 表、与 skill 的配合
│   ├── fonts.md / cli.md / install.md / testing.md   # 字体、CLI 契约、安装、语料与 doctor
│   ├── check-deliverable.md    # 交付前检查脚本：两条判据、不覆盖什么
│   └── general-figure-guide.md # skill 怎么写：写作规则、owner 划分、两副本同步纪律
├── PROGRESS.md                 # 只记当前状态（进度/问题/下一步）
├── src/                        # 主库：纯 SVG 组图器
│   ├── main.rs                 # 薄壳：只 `fn main()`，委托给 `cli`
│   ├── cli/                    # 参数解析 / 帮助文本 / 执行 / 单测
│   ├── lib.rs                  # plot_grid()：布局 → 逐输入按标签协议变换 → 拼装输出
│   ├── layout.rs               # 网格划分（nrow/ncol/rel-*/margin/gap）→ 每个 cell 的 Rect
│   ├── geom.rs                 # canvas box / panel box 解析；scale_panel_to_cell
│   ├── transform/              # 按 data-scale 逐图元重写坐标（mod/primitives/lengths/paths/units/patterns）
│   ├── path/                   # <path d> 的解析 / 按轴重写 / 展平（mod/parse/rewrite/flatten/tests）
│   ├── ink/                    # 内容墨迹盒（W6 用；mod/walk/bbox/shapes/text/uses/tests）
│   ├── warnings/               # 九条非致命告警 W1–W9 的收集与文案
│   ├── dom.rs                  # 最小 XML DOM；ns.rs 给 id/IRI 加前缀
│   ├── labels.rs               # 面板字母的构造
│   ├── input.rs / output.rs    # SVG 读写；error.rs / normalize.rs
│   ├── raster.rs               # PNG/AVIF 渲染；PNG 写 pHYs（dpi 元数据）
│   └── convert/                # 外源方言 → 协议格式（R42 起并入本库，不再单独成 crate）
│       ├── mod.rs              # 薄门面：mod + pub use（原 convert/src/lib.rs）
│       ├── dom.rs error.rs options.rs geom.rs transform.rs   # DOM、类型、画布盒、transform 分类
│       ├── scan.rs warnings.rs tag.rs self_check.rs pipeline.rs cli.rs
│       ├── affine.rs path.rs base64.rs units.rs              # 仿射烘焙 / path 解析 / 位图编解码
│       └── translate/{mod,bake,uses,image,style}.rs + translate/shapes/  # 翻译流水线
├── tests/selftest/             # **唯一**自检测试目标（R19 起；按主题分模块）
├── tests/fixtures/             # `doctor` 语料（ggplot2 / matplotlib / seaborn；MANIFEST.toml 登记）
├── tests/{mpl_dialect,rdkit_dialect}/  # 方言 fixture
├── tests/{s22_repro,fig2_mixed}/  # 端到端资产：S22 六面板复刻；Figure 2 矢量带 + 13 位图 panel
├── tests/user_tests/<日期>/    # **用户的真实使用体验**：README（带测试时的 commit id）+ 输入/成品
├── scripts/                    # 成品交付前检查（薄壳 check_deliverable.py + deliverable_checks/ 包）
├── docs/
│   ├── general-figure-guide/   # 本工具自带的组图 skill（**规范源**；manuscripts_skills 那份是复制件）
│   └── PITFALLS/               # 报告 + round-data/R4…R26（每轮可跑证据；账本由 pitfalls-manager 管理）
```

> **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；根仓库 `.gitignore`
> 排除 `tools/`，所以**开发与提交一律用本目录的仓库**。旧记录说的"另有一个空的用户仓库、要用根仓库"
> 已作废（2026-11-22 核实：本目录 `git log`/`git remote` 就是正式历史）。

## `ARCHITECTURE/` 索引

| 文件 | 写什么 | 什么时候看 |
|---|---|---|
| `ARCHITECTURE/tag-protocol.md` | 输入契约（面板盒/`data-scale` 五策略）、支持图元、**分界原则与判据**、`<path>` 保真边界、失败口径 | 要改主库能搬什么、要判断某个构造该归主库还是 `convert` |
| `ARCHITECTURE/layout.md` | 布局语义（`--height` 可省、`rel_*` 倍数、字母带、垫片）、几何约束、`s`/字号、**设计参照与对齐表** | 调版面、判断"该不该手算某个值"、核对与 R 的偏差 |
| `ARCHITECTURE/convert.md` | 翻译手段、仍 fail-closed 的构造、**`<path>` 出形态决策表**、幂等语义、方言兼容性 | 动 `src/convert/`、遇到新方言、判断某构造成不成 |
| `ARCHITECTURE/warnings.md` | 告警怎么写（三要素/聚合/非致命/英文）、判据取向与容差理由、**W1–W7 表**、**与 skill 的配合契约** | 加/改告警、写指南里的"读 stderr"那句、判断某情形该不该出声 |
| `ARCHITECTURE/fonts.md` | 字体来源唯一 = `fontlib/`、默认族 Arial、`--label-fontfamily` / `--font-dir` / `--list-fonts`、**W8**、空库 fail-closed、`fontlib/` 成员 | 动字体、加/改字体告警、换机器补字体、判断某族名为啥不生效 |
| `ARCHITECTURE/cli.md` | 子命令与参数面、**退出码分档**、`--help` 契约、**字体目录的运行时解析顺序**、CLI 单测要求 | 加/改参数、改退出码、改帮助文本、改字体目录解析 |
| `ARCHITECTURE/install.md` | `install.sh` 做什么（release 编译 + 装**一个**二进制 + 技能 + 字体到数据根）、幂等语义、**每次更新后必须重装**、`--doctor` 怎么用 | 改安装、发布/换机器、怀疑装完的二进制读了旧路径 |
| `ARCHITECTURE/testing.md` | fixture 语料库（ggplot2 / matplotlib 生成、按图型穷举）+ `MANIFEST` 字段 + **`doctor` 的判据与输出** | 加 fixture、改自检判据、判断某图型没被覆盖 |
| `ARCHITECTURE/check-deliverable.md` | 两条检查的判据与为什么、"无裁切"由谁承担、不覆盖什么、与 vision 的分工 | 改判据脚本、解释"为什么这条没抓出来" |
| `ARCHITECTURE/general-figure-guide.md` | skill 的写作规则（只写现状/不列告警/薄入口）、**各部分 owner 划分**、规范源与复制件 | 改这个 skill 的任何文件之前 |

## 跨领域约定

1. **体积口径（R8/R9 起）**：**每个文件 ≤ 20 KB（按 20000 B 从严）**——`src/`、`src/convert/`、
   `tests/`、`scripts/` 的源文件**与** `ARCHITECTURE.md`、`ARCHITECTURE/*.md`、`PROGRESS.md`、指南各文件都适用。
   **唯一豁免：`docs/plot_grid.R`**——作者早先手写的组图范式脚本，只作参考、不参与构建与断言（用户 2026-10-06 定），
   不要再"顺手压缩"它。其余 `docs/` 下的文件仍按口径。
   **拆分纪律：用 `mod`，不要用 `include!`**——rustfmt 只跟进 `mod` 的模块树，`include!` 会让文件脱离 fmt 门禁、
   且 `cargo fmt` **谎报干净**（`R8` 实测）。入口/门面保持薄（`lib.rs`/`main.rs`/`check_deliverable.py`/
   `compose_*.py` 只做声明、再导出、调用）。`round-data/R8`…`R14` 把它钉成断言。
2. **文档分工**：**skill = 成品，只写"现在该怎么做"**；**定义 / 结构 / 决定**写本文件（决定→`# 已定型的决定`，
   只留"为什么 + 指针"）；**机制**写 `ARCHITECTURE/`；**状态**写 `PROGRESS.md`（覆盖式，不记历史）；
   **历史**交给账本与 git。
3. **失败必须响亮**：工具绝不静默产出"看着对、其实错"的结果。细则见 `warnings.md`（判据取向）
   与 `tag-protocol.md` §5（fail-closed vs 宽松）。
4. **账本与历史**：**账本**由 `tools/pitfalls-manager` 管理、**只经它的脚本读写**（直接读会炸上下文，
   位置也不必知道）；`docs/PITFALLS/round-data/<轮次>/` 是每轮的**可跑证据**。
   动任何位置前先 `query <路径> --repo <本工具目录>`——**`--repo` 必须显式给**。
   改名/移动/拆分后**提交完要 `link`**。
5. **生成物不入库**：`tests/*/out/`、`*.csv`、`.docx` 等由 `.gitignore` 排除；入库的是**源**（脚本、报告、
   `!*.sh`），交付物按需重建。

## 参考索引

| 材料 | 路径 |
|---|---|
| **组图逻辑范本（本工具的设计参照）** | `research/白藜芦醇胰腺癌/steps/01_bulk筛选_单细胞空转基础分析/results/scripts/plot_grid.R` |
| 协议完整说明：`data-scale` 五种策略、图元级行为、绘图侧打标指引 | `docs/single_cell/svg_grid_agent_usage.md` |
| 设计依据与取舍：为何不用 R grob、为何选显式标签协议、被否决的方案 | `docs/single_cell/svg_plot_grid.md` |
| 初版实现计划 | `docs/superpowers/plans/2026-06-17-svg-grid.md` |
| 账本（只经脚本读） | `tools/pitfalls-manager`（按项目目录定位） |
| 每轮可跑证据 | `docs/PITFALLS/round-data/` |
