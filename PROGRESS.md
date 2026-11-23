# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：六缺陷全部修复（R27–R36）**，随后的
  R37–R41 又收掉 `<defs>` mask/marker/filter 与 pattern 单位语义，R42 把转换器并入 `src/convert/`。
  余下两项 pre-existing（均已开报告）：组合图字母重号静默、doctor 告警码漂移。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` 2026-09-28 R5 起；分界原则见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 394 项**（211 lib + 59 main + 124 selftest，R42 起单 crate）全绿，
  `fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **告警 W1–W9**：W1–W7/W9 **非致命**（只走 stderr、**默认静音**，`--verbose` 才展开；退出码与产物字节都不变）；
  **W3（给已带字母的输入再加非空字母）rc=2**、**W8（缺标签字体族）rc=3**，两者都**不写产物**、不依赖 `--verbose`；
  正常路径 stderr 0 字节；`convert` 子命令同样受静音控制：
  契约与清单见 `ARCHITECTURE/warnings.md`。
- **`src/convert/`（R42 起并入主库，子命令 `svg_grid convert`）**：R `svglite`/matplotlib/`pdftocairo -svg`
  三种来源可入流水线；`<path>` 出形态决策表、`clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、
  `style` 百分比解析、`<defs>` mask/marker/filter 与 pattern 单位语义（R37–R41）。
  **2026-11-22 D1 已修**：≥3 子路径的填充 path 不再合并成 `<polygon>`，改为展平多轮廓 `<path>`（M/L/Z），
  字形孔不再填死（R27 + R28）。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（机制见
  `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选路线 `02a` = `svg_grid`，
  回退 `02b` = cowplot；两副本 `diff -r` 为空）。**HARNESS 是两件套**（2026-10-06 起）：本文件
  + `ARCHITECTURE.md`（含 `# 已定型的决定` 与 `ARCHITECTURE/` 机制文档，索引见该文件）+ `docs/`；**不设 `DECISIONS.md`**。
  **`INSTRUCTIONS.md` 是用户手写、只由用户改**，与其余文件冲突时以它为准。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**），**R1–R42 已入账**；
  `link` 组 L1–L64；`round-data/R3…R42` 各有一份可跑证据。**2026-11-22 已 reanchor**：锚点 =
  `[tools/svg_grid, monorepo]`（旧 commit 只在 monorepo 里），`check` 0 条、`ids` 不变。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）。**R3 起 `--height`/`--rel-*` 完全不再出现**；
  **R4 零告警通过**（六格 `sx=sy=1.0000`）。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已用本管线重做；**Figure 2**（矢量带 + 13 个位图
  panel）复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。
# Completed

（每条一行；细节在账本与 `ARCHITECTURE.md` §已定型的决定，不在此重复。）

- 2026-11-23 **D5b–D5d W8 收尾（R34–R36）**：① `describe_missing_font_error` 原来取"链里第一个非通用名"
  判是否在库 ⇒ 改为**链里任一名字可用（在库或通用族）即放行**，整链不可用才 rc=3 并点名整链 + 可用族 + 修法；
  ② 错误行 200 字符硬上限（`MAX_ERROR_BODY`/`assemble_within`），W3 205→188、W8 239→194/195、W5 217→191；
  ③ D5c 把病态输入的预算手算错 1（`"(available: "` 12 vs 真实 13）⇒ 整段 `available:` 丢失、回显塌成 `"…"`；
  D5d 改为**按 `format!` 拼出的真实长度反推预算**，加 `AVAILABLE_OPEN`/`truncate_body`，链回显优先于库列表。
  根 245 tests（89+59+97）/ convert 100 / doctor 57/0/1；R8–R14 + R28–R36 全绿。提交 `e362c01`→`b55669e`（R34）、
  `e4660e8`→`5482421`（R35）、`98a173e`→`d32ab03`（R36）；STOP 后 4d737b2 压 PROGRESS 回 19990 B。
- 2026-11-22 **D1–D6 六缺陷（R27–R33）**：字形孔（≥3 子路径的填充 path 改展平多轮廓 `<path>`，R27/R28）、
  末行格子尺寸（行高 = `available_w / Σ全网格列权重`，R29）、默认近方形网格（R30）、网格下界校验 +
  `--verbose` 默认静音（R31）、`plot-grid` 自动 convert（R32）、labels 的 NULL/W3/W8/margin0（R33）。
  机制见 `ARCHITECTURE/{layout,warnings,convert}.md`；数字、变异与复现见账本与 `round-data/R27…R33`。
- 2026-11-22 **账本 reanchor**：`pitfalls-manager` 新增 `reanchor` 子命令（commit `bcb19c0`，已装）；
  锚点从 monorepo 改为 `[工具仓, monorepo]`，`check` 46 条悬空 → 0，`query`/`ids`/`record` 均验证通过；
  备份在 `/home/nihao/pitfalls-manager-backups/svg_grid-reanchor-20261122-221338/`。
- 2026-11-22 **用户验收审计（未改代码）**：对照 `INSTRUCTIONS.md` 的四条新要求逐条实测，全部未满足；
  三条根因已定位到 file:line（见 `# Known Issues` 前四条）。同时核实：`doctor` 57 pass / 0 fail / 1 skip、
  `cargo test` 207 项全绿、`~/.local/bin/svg_grid` 与 `target/release` 同 sha256（装的是最新构建）。
- 2026-10-06 **R26–R17**：参数面收敛 + `plot-grid --plan`（R26）；告警瘦身、单条 ≤198 字符（R25）；
  I/O 报错点名文件（R24）；Windows 安装面（R23）；`install.sh` 装 skill + `--check`（R22）；
  原生出图查证流程（R21）；指南补 `# CORE WORKFLOW`（R20）；自检合成 `tests/selftest/`（R19）；
  `convert` 并为子命令（R18）；`doctor` 子命令（R17）。
- 2026-10-06 **R16–R13**：修 paint server 被丢弃 + W9（R16）；`install.sh`（R15）；用户参数打崩工具
  rc=2、退出码分档（R14）；字体库 `fontlib/` + W8（R13）。
- 2026-10-06 **R12–R11**：补 `round-data/R10/` 证据（R12）；`DECISIONS.md` 并入 `ARCHITECTURE.md`（R11）。
- 2026-09-28 **R10–R5**：指南规范源（R10）；HARNESS 归位（R9）；6 个 >20 KB 源文件拆分（R8）；
  `src/ink.rs`/`src/path.rs` 拆目录（R7）；`<path d>` 宽松化 + W6 微尘（R6）；主库纳入 `<path>`（R5）。
- 2026-09-27 **R4–R3**：W6 内容越界 + 死白带判据（R4）；按 7 篇反馈更新指南（R3）。
- 2026-09-20 及更早：按 10 篇反馈修阻塞点（7 修 / 3 判外部或用错）；组图路线反转为 `svg_grid` 优先；
  布局语义 cowplot 化五项、`convert/` 建 crate、`<image>`/`--png-dpi`。
# In Progress

- 无进行中轮次（D1–D6 + D5b–D5d 全部完成，R27–R36）。下一轮 = 消费 `docs/PITFALLS/blocker-convert-defs-mask-dropped.md`（R37）。

# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的缺口（2026-11-23 实测）

**D1–D6 + D5b–D5d 全部已修（R27–R36）**，`INSTRUCTIONS.md` 的六条要求均已满足。剩三项 pre-existing，均已开报告：

1. **pattern 混合单位继承仍会静默错**（R41 遗留 P2，下一轮"单位语义收尾"）：`<pattern id="base" x/y/width/height
   默认 objectBoundingBox>` + `<pattern id="p" href="#base" patternUnits="userSpaceOnUse" x="10">` ⇒ 继承来的
   tile 数值未缩放，`xor 1104 / maxdiff 255`（源 192、产物 ink 1152），rc=0 无告警。同族还有 `maskContentUnits`
   等 trim 宽松比较、`href` 前导空白/`id` 含空格的词法问题、`patternTransform` 长度量、嵌套 `<pattern>`。
2. **组合图字母重号静默**：两个各自带字母的组合图直接组合（不给 `--labels`）⇒ rc=0、stderr 0 字节，
   产物出现两个 `data-svg-grid-label="0"`（渲染成 A、B、A）。W3 只覆盖"本次给非空字母"。
   报告：`docs/PITFALLS/blocker-svg_grid-duplicate-letters-across-composites.md`。
3. **doctor 告警码漂移**：`src/cli/doctor/run.rs:394-409` 的 `WARNING_CODES` 里 W4a（`` `--labels` has ``）
   与 W5（`may be clipped by the canvas`）两条片段匹配不上现行文案 ⇒ `--json` 的 `warning_codes` 静默缺项；
   注释承诺的 `every_warning_code_still_matches_its_message` 测试**不存在**。
   报告：`docs/PITFALLS/suggestion-svg_grid-doctor-warning-codes-drift.md`。

## 其他未决项

- **`--rel-widths` 部分为 0 会静默产出 0 宽格子**（pre-existing，低优先）：如 `0,1` 时首列 cell 宽 0、面板不可见，
  与 `rel_*` 其余 fail-closed 行为不一致。

- **嵌套组图会累积"与目标网格的偏差"**（不是变形：每步 `Δ=0.00%`）。实测 A4 目标 1000×1414 的
  最小步骤链拼出 1532.48（**+8.4%**），其中 GH 行 1.445 倍行高（底行 3 格只放 2 个 ⇒ 每格 1.5 格宽）。
  中间步 margin 也会改变外层列宽比（默认 24 ⇒ A:BC = 1.951 而非 2.000；`--plot-margin 0` ⇒ 精确 2.000）。
  ⇒ **嵌套时中间步一律 `--plot-margin 0`**，且跨行块只能"近似"表达（无 span 能力）。
- **无跨格（span/merge）能力**：`src/cli/parse.rs` 的 FLAGS 只有 `nrow/ncol/rel-*/gap`，
  `grep span|merge|rowspan|colspan` 在 `src/` 无命中。用户例子里的"A 占 2×2"只能靠嵌套近似
  （A 与 BC 横向组合时同高、宽比 2:1、`Δ=0` 这一步是对的，但 ABC 的纵横比 1.4682 vs 理想 1.4144）。
- **W6 在嵌套输出上是假阳性**：越界数字全部来自**残留的 `visibility="hidden"` 内层 panel-box rect**
  （ink walker 只跳 `display:none`，`src/ink/walk.rs:82,128`）；改 `display="none"` 后 bottom 消失。
  另 `left` 是 `0.75em/字符` 上界估算（高估约 6.4×）。
- **`round-data/R17/test.py` 22.4 KB**，超"每份 ≤ 20 KB"口径：要压只能按族拆文件，暂留。

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
- **path 双实现防漂移**（2026-09-28 R5）：主库 `src/path/` 与 `src/convert/path.rs` 各一份。
  缓解：同容差、同语义、注释互指；改一侧必须同时看另一侧。
- **两副本指南**：**规范源 = `tools/svg_grid/docs/general-figure-guide/`**（改动只在这里做）；
  `tools/manuscripts_skills/general-figure-guide/` 是复制件、用 `rsync -a --delete` 覆盖过去
  （2026-09-28 R10 裁决；账本两条路径的历史已由 `link` 组 L6–L8 并起来）。
- **`research/` 两份旧 `DECISIONS.md` 未迁**（`白藜芦醇胰腺癌` 61 624 B、`MASLD_NP` 5 342 B）：
  在各自的**本地仓库**里、属别的任务 ⇒ **待用户决定**是否一并并入其 `ARCHITECTURE.md`。

# Next Steps

1. **新会话从这里开始：先读 `docs/HANDOFF.md`**（2026-11-22 交接，§0 有 R27–R42 的提交表）。
   六缺陷与 defs/pattern 语义均已收口；**下一轮 = 单位语义收尾**（上条 Known Issue 1 那族 P2/P3），
   随后消费两份待排期报告（组合图字母重号、doctor 告警码漂移），最后做交付前全量复核：
   `cargo test`（394）、`svg_grid doctor` 57/0/1、`pitfalls-manager verify` rc=0、两副本指南 `diff -r`。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 →
   删驱动文档 → 若本轮有改名/移动/拆分，**提交后 `link`**（否则悬空自检会报出来）。
   **语义分支先证后写**：改判据前先读 `usvg` 源码 + 用参考渲染器（resvg/cairosvg）实测"哪一边对"，
   再写代码与期望值（R38–R41 连栽三次的教训）。
3. **（用户定）** 上面 Known Issues 里的几项待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、
   `research/` 两份旧 `DECISIONS.md` **未迁**、是否引入 span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
