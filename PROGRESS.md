# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：全部未满足**（见 `# Known Issues` 前五条，均实测复现）。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` 2026-09-28 R5 起；分界原则见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 207 项**（74 lib + 59 convert-lib + 74 selftest）全绿，`fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **九条非致命告警 W1–W9**（只走 stderr，**退出码与产物字节都不变**，有测试锁定；正常路径 stderr 0 字节）：
  契约与清单见 `ARCHITECTURE/warnings.md`。
- **`convert/`**：R `svglite`/matplotlib/`pdftocairo -svg` 三种来源可入流水线；`<path>` 出形态决策表、
  `clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、`style` 百分比解析。**测试 93 项**（81 lib + 12 CLI）全绿。
  ⚠️ **字形孔在 ≥3 子路径时被填死**（见 Known Issues 第 2 条）。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（机制见
  `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选路线 `02a` = `svg_grid`，
  回退 `02b` = cowplot；两副本 `diff -r` 为空）。**HARNESS 是两件套**（2026-10-06 起）：本文件
  + `ARCHITECTURE.md`（含 `# 已定型的决定` 与 `ARCHITECTURE/` 机制文档，索引见该文件）+ `docs/`；**不设 `DECISIONS.md`**。
  **`INSTRUCTIONS.md` 是用户手写、只由用户改**，与其余文件冲突时以它为准。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**，工具已是 Rust 二进制、
  接口是 `pitfalls-manager <cmd>`），**R1–R26 已入账**；`link` 组 L1–L10；`round-data/R4…R26` 各有一份可跑证据。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）。**R3 起 `--height`/`--rel-*` 完全不再出现**；
  **R4 零告警通过**（六格 `sx=sy=1.0000`）。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已用本管线重做；**Figure 2**（矢量带 + 13 个位图
  panel）复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。

# Completed

（每条一行；细节在账本与 `ARCHITECTURE.md` §已定型的决定，不在此重复。）

- 2026-11-22 **用户验收审计（未改代码）**：对照 `INSTRUCTIONS.md` 的四条新要求逐条实测，全部未满足；
  三条根因已定位到 file:line（见 `# Known Issues` 前四条）。同时核实：`doctor` 57 pass / 0 fail / 1 skip、
  `cargo test` 207 项全绿、`~/.local/bin/svg_grid` 与 `target/release` 同 sha256（装的是最新构建）。
- 2026-10-06 **R26**：**参数面收敛 + 出图几何由工具算**——删掉 18 个未文档化拼法，`--help` 与解析器双向一一对应；
  新增 **`plot-grid --plan`**（只读、零写入）：打印总画布、每输入一行 `canvas/cell/s/Δ`、以及可照抄的归一化命令，
  几何与真跑同源（`geom::cell_scale`）。证据 R26 = 22 条。
- 2026-10-06 **R25**：**告警瘦身**——单条最长 722 → **198 字符**；公共因果抽成每次运行只打一行的 `note:`；
  触发条数/码集合/退出码不变。证据 R25 = 96 条。
- 2026-10-06 **R24**：**I/O 报错点名文件**——`SvgGridError::Io` 带 `Option<PathBuf>`，形状统一为 `I/O error at <path>: …`。
- 2026-10-06 **R23**：**Windows 安装面**——数据根解析抽成 `src/home.rs`（`HOME` → `USERPROFILE`）；新增 `docs/install-windows.md`。证据 R23 = 30 条。
- 2026-10-06 **R22**：`install.sh` 增装**随工具发布的 skill**（白名单拷贝、幂等、原子替换、`--no-skill`/`--dry-run`）
  与 **`--check`**（只读自检）。证据 R22 = 25 条；回填 **R3** 的可跑证据。
- 2026-10-06 **R21**：原生出图从"查一张表"升级为**查证流程**（认包 → 查本仓 `tools/` → 查官方资料 → 确认无才自绘）；
  `# native: none` 必须带**两处查证痕迹**。证据 R21 = 53 条。
- 2026-10-06 **R20**：组图指南补**主干**——`SKILL.md` 立 `# CORE WORKFLOW` 五步；新增 `00_native_plots.md`
  （95 条函数对照 + 四类例外）；`05_checklist.md` 加 `grep` 可查的来源判据。证据 R20 = 44 条。
- 2026-10-06 **R19**：**自检合成一个测试目标** `tests/selftest/`（190 条未丢）；`convert` 的 stderr 前缀改为随调用方。
- 2026-10-06 **R18**：**只交付一个二进制**——`convert` 成为 `svg_grid convert` 子命令；`doctor` 第一段改进程内调用；
  `install.sh` 只装 `svg_grid`。证据 R18 = 43 条。
- 2026-10-06 **R17**：`doctor` 子命令——一条命令跑整集语料（两段管线 + 判据 + `--only/--jobs/--json/--keep`）；
  全量 **57 pass / 0 fail / 1 skip**。此后"改完先跑"的默认回归入口（`ARCHITECTURE/testing.md`）。
- 2026-10-06 **R16**：语料基线测出两条**静默掉墨水**的缺陷——`<defs>` 里的 paint server 被整段丢弃 ⇒ `url(#…)` 悬空。
  修：保留 paint server、组图侧不碰其坐标域、新增 **W9**。墨水 0.0672→0.6809、0.2180→0.4245。
- 2026-10-06 **R15**：安装——`install.sh`（release 编译 → 装 `~/.local/bin` → 装字体到数据根；幂等、`--dry-run`）。
  **每次改完代码都要重跑一次**（`ARCHITECTURE/install.md`）。
- 2026-10-06 **R14**：修掉"用户参数能打崩工具"那批（`--ncol/--nrow 0`、`--avif-quality` 越界 ⇒ 用法错 rc=2）；
  退出码分档 0/1/2/3；`--help`/`-h`/`--version` 修好（stdout + rc=0，覆盖 28 flag）；字体目录改**运行时**解析；
  `transform.rs` 拆目录。
- 2026-10-06 **R13**：字体库 `fontlib/`——字体只从该目录读（`load_system_fonts` 已删、`system-fonts` feature 已去掉）、
  面板字母默认 **Arial**、`--list-fonts`/`--font-dir`、缺族 **W8**、空库 fail-closed；`src/lib.rs` 拆出 `src/labels.rs`。
- 2026-10-06 **R12**：消费阻塞点报告「R10 缺可跑证据」——自检是真阳性，按 §4.3 **就地补** `round-data/R10/`。
- 2026-10-06 **R11**：`DECISIONS.md` 并入 `ARCHITECTURE.md` §已定型的决定后删除；跨工具陈旧引用同步为两件套。
- 2026-09-28 **R10**：指南规范源裁决 = `docs/general-figure-guide/`（另一份是复制件，`rsync` 覆盖）；账本 `link` L6–L8。
- 2026-09-28 **R9**：HARNESS 归位——`ARCHITECTURE/` 拆出六份机制文档、父文档瘦身、`PROGRESS` 去历史；
  skill 按"只写现状 + 不列告警"瘦身。
- 2026-09-28 **R8**：把 6 个 > 20 KB 的源文件全部行为保持型拆开（并修掉 `include!` 造成的 fmt 盲区）。
- 2026-09-28 **R7**：`src/ink.rs`、`src/path.rs` 行为保持型拆成目录；触发账本 `link` L1/L2。
- 2026-09-28 **R6**：`<path d>` 宽松化（畸形/非有限 ⇒ W7 警告 + 原样透传）、W6 微尘提示、删掉恒真的检查 2。
- 2026-09-28 **R5**：主库纳入 `<path>`（"最小的完备表达能力"分界原则）。
- 2026-09-27 **R4**：新增 W6（内容越出自身画布）；死白带判据修正并落成唯一脚本。
- 2026-09-27 **R3**：按 7 篇出图手感反馈更新指南（未改一行代码）。
- 2026-09-20：按 10 篇真实反馈修阻塞点（7 修 / 3 判外部或用错）。
- 2026-09-17：组图路线反转为 `svg_grid` 优先、cowplot 降回退；混合（svg + 位图）端到端走通。
- 2026-09-16：布局语义 cowplot 化五项；删混合栅格-矢量 composer；`convert/` 建 crate；
  `<image>`/`raster-images`/`--png-dpi`；五条告警；批次重建 12 张成品图。

# In Progress

- 无。

# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的四条缺口（2026-11-22 实测，全部未满足）

1. **`plot-grid` 不会自动 convert**（违反用户期望 #11）。未打标签的输入直接 `rc=2`：
   `svg_grid: invalid input: input SVG must contain a <rect data-panel-box="main" .../> element`
   （抛出点 `src/geom.rs:21`，唯一调用点 `src/plan.rs:144`，`plot_grid` 与 `plan_grid` 共用）。
   现成钩子：`svg_grid_convert::convert(&str, bool)`（`convert/src/pipeline.rs:22`，纯字符串进出、无 I/O；
   `Cargo.toml` 已依赖 convert，**无新依赖**）。`doctor` 的 in-process 调用可作范式
   （`src/cli/doctor/run.rs:205` → `run_in`）。注意：已组好的组合图有多个 panel-box，convert 会报
   `Incomplete`——但这类输入本来就有 panel box，不会进自动分支。
2. **convert 损坏字形孔**（违反用户期望 #11 后半）。`convert/src/translate/shapes/path_form.rs:52`（纯填充）
   与 `:83`（填充+描边拆分）都调 `join_closed_points`（同文件 `:155-166`），把 N 条子路径点串首尾相接；
   `<polygon>` 的隐式闭合再从末点连回整串首点 ⇒ N 条 bridge 成闭环。**N=2 时两 bridge 同线段反向、面积 0
   （现有测试只测 N=2，故全绿）；N≥3 时闭环是真多边形 ⇒ 内孔 winding ±1 被填死**。
   实测：`pandamap_ALB_A.svg` 46 条 ≥3 子路径 path 全部命中，24/94 字孔翻转，16 个标签组可见变化
   （"ASP 451" bridge 环面积 192.968 单位²，隔离渲染黑墨 18,312→29,016 px = 1.58×）。
   **曲线展平不是原因**（最大偏差 0.0145 源单位 = 0.029 px @2×）。`translate/shapes/mod.rs:33-37` 的
   "bridge 与回程同线段反向"论证**只对 N=2 成立**，是错前提，须一并改。
   修法方向：`subpaths.len() >= 3` 的填充路径**不要合并成单 polygon**，直接输出展平后的 `<path>`（M/L/Z，
   resvg 完整支持、语义无损）；并补 N≥3 的回归用例（现有 `tests.rs:29`/`:139` 都绕开了）。
3. **末行格子尺寸随"该行实际有几个图"变**（违反用户期望 #10）。`src/layout.rs` 的 `row_aspect_sums`
   （`:197-208`）对末行只累计**实际存在的**面板（`index < count` 过滤，`:203`）⇒ 行高按"本行铺满整宽"算
   （`available_w / Σaspect`），而 cell 宽仍按"只占一列"算 ⇒ 5 输入 `--ncol 2` 时 E 格 **476×972.614**、
   `Δ=−50%`。根因一句话：**行高按"本行几个图"算，列宽按"全网格几列"算**，两套基准。
   cowplot 1.2.0 实测：5 图 `ncol=2` 的 5 个格子全 `1/2 × 1/3`（同格，**不拉伸**）；5 图默认
   `ceil(sqrt(5)) = 3` 列 ⇒ 3+2（左对齐，第 6 格空着）。⇒ **正确修法不是"末行跨满宽"**，而是
   **格子尺寸只由网格（cols/rows）决定**：行高 = `available_w / Σ(column weights)`（`--rel-heights`
   只做倍数），末行少几个图不影响格子大小；缺的槽位要么显式 `NULL`、要么按用户规则报错。
   变体同样复现：`--nrow 2`（5 输入）末行 `Δ−33.33%`、3 输入 `ncol 2` 末行 `Δ−50%`、显式
   `--height` 也一样（与 auto-height 无关）。
4. **`--labels` 在中间步加会重号**（违反用户期望 #12）。`src/lib.rs:367-379` 只做 W3 检测
   （`src/warnings/labels.rs:18-36`），**exit 0 照常写文件**。实测：AB 加字母后再与 C 组合并加字母
   ⇒ 4 个 `data-svg-grid-label`（A、B 各两份，C 被标成 B），渲染后字母叠印；且继承字母 `font-size`
   固定 27.027 而 cell 缩到 322 宽 ⇒ 相对字号 8.39%（应 2.7%）。
   合法的分层写法已验证可用：`--labels ,C`（空位占位）对已带字母的输入**不触发 W3**，输出 A、B（继承）+ C。
   ⇒ 建议 W3 只在"给**已带字母**的输入再赋**非空**字母"时升级为**硬错误**，空字符串占位保持安静。
5. **默认网格与"声明"规则都没实现**（违反用户期望 #10）。两处：
   - **默认网格**：用户要求抄 cowplot —— 不给 `ncol`/`nrow` 时 `cols=ceil(sqrt(n))`、`rows=ceil(n/cols)`
     （15 图 → 4×4，5 图 → 3×2），末格按 NULL 声明处理；现在 `(None, None) => (1, count)`（`src/layout.rs:69-74`）
     ⇒ 5 图默认是一行 `1×5`（`--plan` 实测 canvas 1000×242.5），与要求相反。
   - **空位必须声明**：显式 `ncol`/`nrow` 除不尽时，现在静默留空、不提示；按用户口径必须
     显式 `--input NULL` / `--input none`，否则报错。`--input NULL` 已可用
     （`src/cli/parse.rs:470-472`，不区分大小写）。待定：显式模式下报错还是自动补 NULL（见 `# Next Steps`）。

## 其他未决项

- **嵌套组图会累积"与目标网格的偏差"**（不是变形：每步 `Δ=0.00%`）。实测 A4 目标 1000×1414 的
  最小步骤链拼出 1532.48（**+8.4%**），其中 GH 行 1.445 倍行高（底行 3 格只放 2 个 ⇒ 每格 1.5 格宽）。
  中间步的 margin 也会静默改变外层列宽比（BC 用默认 margin 24 ⇒ A:BC = 1.951 而非 2.000；
  用 `--plot-margin 0` ⇒ 精确 2.000）。字母带每行再 +116.8 px（+1.16%）。
  ⇒ **嵌套时中间步一律 `--plot-margin 0`**，且跨行块只能"近似"表达（无 span 能力）。
- **无跨格（span/merge）能力**：`src/cli/parse.rs` 的 FLAGS 只有 `nrow/ncol/rel-*/gap`，
  `grep span|merge|rowspan|colspan` 在 `src/` 无命中。用户例子里的"A 占 2×2"只能靠嵌套近似
  （A 与 BC 横向组合时同高、宽比 2:1、`Δ=0` 这一步是对的，但 ABC 的纵横比 1.4682 vs 理想 1.4144）。
- **W6 在嵌套输出上是假阳性**：`right 538.560 = (300+838.560)−600`、`bottom 550.742 = (24+856.718)−329.976`
  全部来自**残留的 `visibility="hidden"` 内层 panel-box rect**，而 ink walker 只跳 `display:none`
  （`src/ink/walk.rs:82,128`）。改成 `display="none"` 后数字立刻掉到 139.465 且 bottom 消失。
  另 `left 90` 是 `0.75em/字符` 上界估算（真实约 −14，高估 6.4×）。
- **`round-data/R17/test.py` 22.4 KB**，超"每份 ≤ 20 KB"的口径（R18 之后略涨）：要压只能把该轮证据按族拆文件；
  不影响 `doctor`/`cargo test`，故暂留。

- **合成时面板内容不再被面板画布裁剪**：超出面板画布的元素会渲染到成品外边距（密集版式下可能压到相邻
  面板）。要严格复刻栅格语义需在 cell 上补 clip——**尚未做**（机制见 `ARCHITECTURE/layout.md` §2）。
- **固定 `--plot-margin` 时，"溢出字母带但没出画布"的字母仍会被静默裁**：W5 只覆盖"越出画布"。
  规避：给 margin 时按 `gap + 大写高(字号)` 自行留足字母带。
- **W6 的容差 1 px 保留、不加豁免**（已知会为真实的小越界出声，如 S39-C 修好后仍报
  `right by 1.537 px`，成品 24 px margin 掩盖得住）——**待用户裁决**是否放宽或按侧豁免。
- **`convert/` 对 RDKit `MolDraw2DSVG` 方言的结论需重测**：现有记录取自 2026-09-20 之前（当时报
  "37 个不支持节点"），而 `<path>` 出形态与主库 path 支持都已改过 ⇒ **重测后再采信**。
  2D 结构式目前以位图（`MolDraw2DCairo`）进入组图。
- **`<path>` 出形态决策表的已知边界**：判断"是否描边"只看图元自身声明；描边只来自祖先 `<g stroke=…>`
  时，多子轮廓填充路径仍会描出连接边（两个受支持方言实测 0 例，**明确不做**，见 `ARCHITECTURE/convert.md`）。
- **镜像（负）等比 + 旋转弧**会静默失真（手性翻转需翻 `sweep`/取反 `rotation`，而守卫只判
  `|sx| ≠ |sy|`）——**当前 CLI 不可达**，仅登记；将来引入翻转必须补（见 `ARCHITECTURE/tag-protocol.md` §4）。
- **跨行字号不一致**：逐行按各自格宽归一化（保证 `s = 1`）⇒ 不同行纸面字号不同（S22 实测
  5.85 / 3.96 / 9.10 pt）。这是 `s = 1` 的代价，已在 `02a` 写明。
- **无"文字条"能力**（R 的 `ggdraw + draw_label`）：主库只搬运 `<text>`、不生成文字 ⇒ 标题条需先做成
  "只有文字的面板"。
- **复刻图总高 5639 vs 原图 5764（约 2.2%）未 root-cause**：band/margin 模型差异，面板级已对齐。
- **`--rel-heights` 微调行高会纵向拉伸该行面板**（改变格子宽高比 ⇒ 会告警）：自然纵横比布局的必然结果，
  要既改行高又不变形只能改画布宽或面板纵横比。
- **垫片 `--input none` 的自然纵横比按 1:1 计**（R 的 `NULL` 靠 `rel_widths` 定份额）。
- **`AlignMode` 仍未使用**（R 参照逻辑全程不用 `align`，本版不做）。
- **版面可读性天花板**：一行 3 面板 + 固定画布宽 ⇒ 单面板 ~2.6 in、纸面字号 ~3.8 pt（低于 5–6 pt）；
  用户原版 S22 同行亦然，只能改版式或按目标尺寸重设计。
- **matplotlib 之外的方言未覆盖**（Inkscape/Illustrator 导出等，仍 fail-closed）。
- **path 双实现防漂移**（2026-09-28 R5）：主库 `src/path/` 与 `convert/src/path.rs` 各一份。
  缓解：同容差、同语义、注释互指；改一侧必须同时看另一侧。
- **两副本指南**：**规范源 = `tools/svg_grid/docs/general-figure-guide/`**（改动只在这里做）；
  `tools/manuscripts_skills/general-figure-guide/` 是复制件、用 `rsync -a --delete` 覆盖过去
  （2026-09-28 R10 裁决；账本两条路径的历史已由 `link` 组 L6–L8 并起来）。
- **`research/` 两份旧 `DECISIONS.md` 未迁**（`白藜芦醇胰腺癌` 61 624 B、`MASLD_NP` 5 342 B）：
  在各自的**本地仓库**里、属别的任务 ⇒ **待用户决定**是否一并并入其 `ARCHITECTURE.md`。

# Next Steps

1. **（待用户确认后动手）按 `INSTRUCTIONS.md` 的四条缺口排优先级**：
   ① `plot-grid` 自动 convert（`plan.rs:144` 前接 `svg_grid_convert::convert`，成本最低）；
   ② 字形孔修复（`path_form.rs` 的 N≥3 分支改出 `<path>` + 补回归用例）；
   ③ 末行 cell 跨满宽（`layout.rs` 只改末行 cell 宽，画布高不变）；
   ④ `--labels` 中间步重号（W3 升硬错误或自动剥离）。
   ②③ 是**产物正确性**问题，建议先于 ①④。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 → 删驱动文档 → 若本轮有改名/移动/拆分，
   **提交后 `link`**（否则悬空自检会报出来）。取回已删报告：`query` 会打出
   `git -C <repo> show <commit>:<path>`。
3. **（用户定）** 上面 Known Issues 里的几项待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、`research/` 两份旧
   `DECISIONS.md` **未迁**、是否引入 span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
