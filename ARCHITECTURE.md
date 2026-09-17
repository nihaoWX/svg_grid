# Definition

## 任务定义

`tools/svg_grid/` 是一个**纯 SVG 面板组图器**（project-development 任务）。

- 核心能力：`svg_grid plot-grid` 把多个 SVG 面板按网格拼成一张成品 SVG/SVGZ，可选输出 PNG / AVIF。
- 它不是通用 SVG 工具，而是带**显式标签协议**的组图器：每个输入必须自带
  `<rect data-panel-box="main" .../>` 与 `data-scale` 分组，否则组图结果错误、或直接报错退出。
- 协议的设计目标：**面板几何随格子缩放，而文字大小、点径、线宽保持不变**——即 R `grid`/`cowplot`
  的 grob 语义，在 SVG 层面用标签显式表达。普通 SVG 没有这个语义：整组 `transform="scale(s)"`
  会把文字和点一起缩放。
- `convert/` 把**不带标签**的外源 SVG（目前限 R `svglite`/`ggplot2`）转成协议格式。

## 用户期望

1. **`src/` 保持纯净**：主库只做 SVG 组图。不引入栅格（PNG/JPEG）输入、去白边、白掩膜、混合栅格-矢量管线——
   这类内容已于 2026-09-16 整体回退。
2. **外源 SVG 的适配隔离在 `convert/`**：与主库 `src/` 解耦（不改主库代码与依赖）。
3. **失败必须响亮（fail-closed）**：转换器遇到组图器**无法处理的构造**必须**报错退出并列出位置**，
   不能静默产出一张位置错乱的图。
4. **先小批验证再铺开**：先跑通一张真实多面板图并与现有成品并排比对，再推广。
5. **版面靠 CLI 参数迭代**（`--width/--height/--ncol/--nrow/--rel-*/--plot-margin`），
   参考 cowplot 的 `rel_*` 心智：定"谁跟谁一组 + 各自占比"即可；调完看一遍成品图再微调。
   不为此再做参数计算器。
6. 环境：Linux、命令行、可批处理、离线可构建。

# Structure

## 文件布局

```
tools/svg_grid/
├── Cargo.toml / Cargo.lock     # 主库 crate
├── src/                        # 主库：纯 SVG 组图器
│   ├── main.rs                 # CLI：plot-grid 子命令与全部 flag（含 --png-dpi，默认 600）
│   ├── lib.rs                  # plot_grid()：布局 → 逐输入按标签协议变换 → 拼装输出
│   ├── layout.rs               # 网格划分（nrow/ncol/rel-widths/rel-heights/margin/gap）→ 每个 cell 的 Rect
│   ├── geom.rs                 # canvas box / panel box 解析；scale_panel_to_cell
│   ├── transform.rs            # 按 data-scale 策略逐图元重写坐标（协议的执行者）
│   ├── dom.rs                  # 最小 XML DOM（parse/serialize）
│   ├── input.rs / output.rs    # SVG 读写；error.rs / normalize.rs
│   └── raster.rs               # PNG/AVIF 渲染；PNG 写 pHYs（dpi 元数据）
├── tests/compose_tests.rs      # 主库集成测试（20 项）
├── tests/s22_repro/            # 端到端测试资产：S22 的 6 个 matplotlib panel → 复刻（脚本入库，out/ 忽略）
└── convert/                    # 外源 SVG → 协议格式 的转换器（独立 crate，与 src/ 隔离）
    ├── src/lib.rs / main.rs    # 薄入口：mod + pub use / fn main
    ├── src/dom.rs error.rs options.rs geom.rs transform.rs   # DOM、类型、画布盒、transform 分类
    ├── src/scan.rs warnings.rs tag.rs self_check.rs pipeline.rs cli.rs
    ├── src/affine.rs path.rs base64.rs                       # 方言翻译：仿射烘焙 / path 解析 / 位图编解码
    ├── src/translate/{mod,bake,uses,image,shapes,style}.rs   # 翻译流水线（<g transform>/<use>/<path>/<image>/style）
    ├── tests/cli.rs            # 进程级测试（真实驱动二进制）
    └── README.md               # 用法、两步流水线、缩放后果与配准建议
```

> ⚠️ **`tools/svg_grid/` 下另有一个用户建的空 git 仓库**（`.git`，准备把这个工具单独开源用）。本工具的**开发与提交一律用根仓库**；在该目录内直接跑 `git` 会落到那个空仓（看不到任何跟踪文件，容易被误判成"工作区全丢了"）。子 agent 曾因此踩坑，故记于此。

## 标签协议（组图器接受的输入格式）

组图器逐图元重写坐标，**不套外层 transform**。两条必需项：

| 项 | 形式 | 缺失后果 |
|---|---|---|
| 面板盒 | `<rect data-panel-box="main" x y width height/>`（树内任意位置） | **直接报错退出** |
| 缩放策略 | `<g data-scale="xy\|x\|y\|position\|none">`（逐层继承，子级覆盖） | 默认 `none`：只平移不缩放，面板不适配格子 |

策略语义：`xy` 坐标双向重映射、尺寸按 `sqrt(sx*sy)` 缩放；`position` 坐标重映射、**尺寸一律不变**；
`none` 整块平移、内部坐标不重算。**`font-size` 在任何策略下都不缩放。**

**主库受支持的图元**：`circle` / `text` / `use` / `line` / `rect` / `image` / `polyline` / `polygon`，
外加 `stroke-width` 与 `rotate(angle, x, y)`。（`<image>` 与 `<rect>` 同款处理 `x/y/width/height`；
渲染内嵌栅格需要 resvg 的 `raster-images` feature，已启用。）
**主库不处理**（原样留在原坐标 → 版面错乱）：`<path>`、任何非 `rotate` 的 `<g transform>`。
（另：几何写在 CSS `style` 里的值不会被重写，例如 svglite 的 `stroke-width`。）

**`convert/` 的职责是把外源方言翻译成上述图元**，因此它接受的构造比"裸主库"宽：
`<g transform>` 会被烘焙进坐标、`<use>`/`<defs>` 会按引用展开后删除、`<path d>` 会转成
`<polyline>`/`<polygon>`、文字 `translate(x y) rotate(a)` 会归一化、`<image>` 的轴对齐翻转会烘焙进位图
并补 `preserveAspectRatio="none"`。仍然 **fail-closed 拒绝**：无法烘焙的 `<image>` 变换、
嵌套 `<svg>`、`<symbol>`、带绝对坐标的 `<tspan>`、逗号分隔的 `viewBox`、非 `svg` 根、非正画布尺寸。
唯一允许出现在 `<defs>` 里的图元是 `<clipPath>` 下的 `<rect>`——它必须与几何同策略，裁剪框才跟着缩放。

## 输入来源兼容性（2026-09-16 实测）

| 来源 | 图元构成 | 结论 |
|---|---|---|
| R `svglite`/`ggplot2` | 绝对坐标、`<path>` = 0；circle/rect/line/polyline/polygon/text + `<clipPath>` | **已可适配**：`convert/` 后可直接组图（真实 panel 端到端跑通） |
| Python `matplotlib` | `<path>` 292、`<use>` 436、带 transform 的 `<g>` 42、`<image>` 6 | **已可适配**：由 `convert/` 的翻译层吸收（烘焙/展开/path 转换/位图翻转）；S22 端到端验证 |
| 栅格（PNG/JPEG、ChimeraX 渲染） | — | 主库现在支持 `<image>` 的坐标搬运，可随面板进入成品（矢量图内含一小块位图） |

## 已知几何约束

- `scale_panel_to_cell` 用 `sx = cell.width / canvas.width`、`sy = cell.height / canvas.height`
  **独立缩放两轴**，即把画布盒拉伸铺满格子。因此**格子宽高比必须与源画布宽高比一致**，否则面板变形。
  调用方需通过 `--width/--height/--ncol/--nrow/--rel-widths/--rel-heights` 配准
  （`available_w = W − margin.left − margin.right − gap·(ncol−1)`，`cell_w = available_w·relw_i/Σrelw`，
  行同理；默认 `margin=24`、`gap=0`）。
- **画布尺寸 = 输出栅格的像素数**（SVG 是矢量，画布尺寸不影响清晰度，只影响坐标与 PNG 像素数）。
- **PNG 的 dpi 是文件元数据（pHYs 块），与 SVG 无关**：`--png-dpi`（默认 600）只写元数据，
  不改变像素数、不缩放坐标。
- `AlignMode` 目前在 `plot_grid()` 内未被使用（`main.rs` 恒传 `AlignMode::Panels`）。

## 非致命告警（fail-loud；只走 stderr，退出码不变、产物字节不变）

`plot-grid` 会在下列"看起来成功但结果悄悄错"的情形向 stderr 告警；**跑完必须读 stderr**（尤其 agent）。
每条含"发生了什么 + 为什么重要 + 怎么办"，末尾附一条计数汇总。

| # | 触发条件 | 要点 |
|---|---|---|
| W1 | 某输入被布局缩放 `\|sx−1\|` 或 `\|sy−1\|` **> 0.5%** | 协议不缩放 `font-size` ⇒ 该面板内文字相对大小随之改变。0.5% 是刻意的：外层拼装漏给 `--plot-margin 0` 时偏差只有 `(W−48)/W ≈ 0.97%`，用 1% 会漏报 |
| W2 | 某格子的宽高比与面板画布宽高比偏差 **> 1%** | 该格被非等比拉伸（面板变形、文字与几何脱钩）。修法：省略 `--rel-*` 让布局按面板纵横比分配 |
| W3 | 某输入已含 `data-svg-grid-label` 而本次又传了 `--labels` | 给组合图重复加字母 → 重号/串位。字母只加在"拥有该面板为直接输入"的那一层 |
| W4a | `--labels` 的条目数与输入数不符 | 多余的被忽略、缺少的不标；跳过某格用空条目 `--labels A,,C` |
| W4b | `--labels` 里字母重复 | 同名面板无法区分 |
| W5 | 字母墨迹会越出画布 | 调大 `--plot-margin`，或省略它让字母带自动预留 |

## 设计参照：R 端 `plot_grid` 的组图逻辑

本工具的设计初衷是**复刻**用户既有 R 脚本的**组图逻辑**（范本：`research/白藜芦醇胰腺癌/steps/01_bulk筛选_单细胞空转基础分析/results/scripts/plot_grid.R`，677 行；编排全部来自 `cowplot::plot_grid`，脚本内无 `source()`）。判断"哪里该改"以该逻辑为准，而不是以 `svg_grid` 现状为准。

**逻辑特点（"用起来丝滑"的具体机制）**

1. **组图期完全不谈尺寸**：全脚本没有任何 `plot_grid(..., width=, height=)`；`width/height` 只出现在最后落盘的 `png(width=, height=, res=)`（如 `png(..., width = 4000, height = 4000*(2^0.5), res = 300)`）。尺寸只影响输出分辨率，不影响组图对象本身。
2. **`rel_*` 只算比值**：上游 `x_deltas <- rel_widths/sum(rel_widths)`、`y_deltas <- rel_heights/sum(rel_heights)`。脚本里 `c(1,1)`、`c(1,0.8)`、`c(0.1,1)`（和≠1）随处可见 ⇒ `c(1,1,2)` ≡ `c(3.2,3.2,6.4)`；单值还会 `rep(..., length.out = rows/cols)` 自动铺开。
3. **面板在格内弹性重排**：格内文字/轴线/标题保持 pt 绝对量、panel 主体吃剩余空间（R grid 的 null 单位）——所以**调用方不需要知道任何面板的纵横比**；同排共用 `y_delta` 天然等高、同列共用 `x_delta` 天然等宽。
4. **嵌套天然成立**：一次 `plot_grid()` 的产物就是普通面板，可直接塞进上一层（脚本叠到 3 层）；每层 `rel_*` 只在本层归一化，层间无需任何配套数字。
5. **字母**：`"AUTO"` 自动编号（`LETTERS[1:num_plots]`）；位置写在"格内相对坐标"（`x + label_x*x_delta`）；某格不要字母写 `""`。
6. **留白不靠 margin 参数**：外层 `ggdraw() + theme(plot.margin = margin(t,r,b,l,unit="pt"))` 统一预留；要"两侧留白"就用 `NULL` 占位格（`plot_grid(NULL, pp, NULL, rel_widths = c(0.08, 1-0.16, 0.08))`）。
7. **跨面板对齐不设参数**：脚本全程不用 `align`；需要对齐就先组子图再整体放入。

**调用方不需要手算的东西**：行列数（给一个另一个自动）、每格占比的分母、每格位置与尺寸、面板纵横比与边距、画布宽高（只在保存时给一次）、字母编号与位置、"空位"。

**与 R 逻辑的对齐情况**（2026-09-16 起；逐条决策见 `DECISIONS.md`）

| 要点 | R 端 | `svg_grid` 现状 |
|---|---|---|
| 画布宽高 | 后置/自动：只在保存时给 | **已对齐**：省略 `--height` → 画布高由各行的自然高（分配宽 ÷ 该行 Σ纵横比）+ margin + gap 推出，**该值就是输出画布**（`viewBox` 与 PNG 同一尺寸） |
| 面板纵横比 | 不需要喂 | **已对齐**：省略 `--rel-widths` → 列宽按各面板 canvas 纵横比分配 ⇒ 每格宽高比 = 其画布宽高比 ⇒ **零变形** |
| `rel_*` 语义 | 乘在自然尺寸上的倍数 | **已对齐**：同上（`1,1,2` ≡ `3.2,3.2,6.4`）；显式给出时作为倍数生效 |
| 字母空间 | 外层 `ggdraw + plot.margin` 一次预留，字号与 margin 同单位 | **已对齐（自动预留）**：用 `--labels` 且未显式给 `--plot-margin` → 自动留字母带；显式给了则尊重，并在字母会被裁时 stderr 告警 |
| 空位/垫片 | `NULL` 占位格 + `rel_widths` 分份额 | **已有**：`--input none` 占一个空格；垫片的自然纵横比按 **1:1** 计，可用 `--rel-widths` 调 |
| `rel_*` 长度不符 | `rep()` 自动补齐 | **已改为 fail-closed**：长度/取值非法直接报错（比 R 更严，符合项目原则） |
| 文字条（标题/图注条） | `ggdraw() + draw_label()` + `rel_heights` 给份额 | **未做**：主库只搬运 `<text>`、不生成文字 → 需先做成"只有文字的面板" |
| 跨面板对齐 | 不设参数，靠嵌套绕开 | 一致：`AlignMode` 仍未使用（本版不做） |

**布局语义（现状摘要）**：`--width` 定尺度；省略 `--height` 即自动推出；省略 `--rel-*` 即自然尺寸；显式给 `--height`/`--rel-*` 时保持旧的"填满画布"语义（旧调用点不受影响，**但把 `--rel-widths` 填成纵横比的老写法现在语义变了**——见 `DECISIONS.md`）。

## 参考索引

| 材料 | 路径 |
|---|---|
| **组图逻辑范本（本工具的设计参照）** | `research/白藜芦醇胰腺癌/steps/01_bulk筛选_单细胞空转基础分析/results/scripts/plot_grid.R` |
| 协议完整说明：`data-scale` 五种策略、图元级行为、绘图侧打标指引 | `docs/single_cell/svg_grid_agent_usage.md` |
| 设计依据与取舍：为何不用 R grob、为何选显式标签协议、被否决的方案 | `docs/single_cell/svg_plot_grid.md` |
| 初版实现计划 | `docs/superpowers/plans/2026-06-17-svg-grid.md` |
