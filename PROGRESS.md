# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：六缺陷全部修复（R27–R36）**；随后
  R37–R41 收掉 `<defs>` mask/marker/filter 与 pattern 单位语义，R42 把转换器并入 `src/convert/`，
  R43 收掉单位属性语义，R44 收掉组合图字母重号，R45 收掉 doctor 告警码漂移，R46 收掉定义元素的缩放单位，R47 收掉文本节点的字符引用，R48 收掉字母查重的漏判与误伤。
  余下 pre-existing 均已开报告（见 `# Known Issues`）。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` R5 起；分界见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 474 项**（256 lib + 59 main + 159 selftest，R42 起单 crate）全绿，
  `fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **告警 W1–W9**：W1–W7/W9 **非致命**（只走 stderr、**默认静音**，`--verbose` 才展开；退出码与产物字节都不变）；
  **W3（给已带字母的输入再加非空字母）rc=2**、**W8（缺标签字体族）rc=3**，两者都**不写产物**、不依赖 `--verbose`；
  正常路径 stderr 0 字节；`convert` 子命令同样受静音控制（契约见 `ARCHITECTURE/warnings.md`）。
- **`src/convert/`（R42 起并入主库，子命令 `svg_grid convert`）**：R `svglite`/matplotlib/`pdftocairo -svg`
  三种来源可入流水线；`<path>` 出形态决策表、`clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、
  `style` 百分比解析、`<defs>` mask/marker/filter 与 pattern 单位语义（R37–R41）。
  **D1 已修**：≥3 子路径的填充 path 不再合并成 `<polygon>`，改为展平多轮廓 `<path>`（M/L/Z），字形孔不再填死。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（见 `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选 `02a` = `svg_grid`，回退 `02b` = cowplot；
  两副本 `diff -r` 为空）。**HARNESS 是两件套**：本文件 + `ARCHITECTURE.md`（含 `# 已定型的决定` 与
  `ARCHITECTURE/` 机制文档）+ `docs/`；**不设 `DECISIONS.md`**。**`INSTRUCTIONS.md` 是用户手写、只由用户改**。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**），**R1–R48 已入账**；
  `link` 组 L1–L64；`round-data/R3…R48` 各有一份可跑证据。**2026-11-22 已 reanchor**：锚点 =
  `[tools/svg_grid, monorepo]`（旧 commit 只在 monorepo 里），`check` 0 条、`ids` 不变。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）。**R3 起 `--height`/`--rel-*` 完全不再出现**；
  **R4 零告警通过**（六格 `sx=sy=1.0000`）。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已用本管线重做；**Figure 2**（矢量带 + 13 个位图
  panel）复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。
# Completed

（每条一行；细节在账本与 `ARCHITECTURE.md` §已定型的决定，不在此重复。）

- 2026-11-24 **R48 字母查重的漏判与误伤**：`src/label_conflicts.rs` 的 `walk_letters`/`collect_text` 递归收集
  marker 子树文本并按文档序拼接、`normalize_label` 与显式 `--labels` 同口径（trim + 折叠空白）、`enter` 跳过
  不可见子树（`NON_RENDERED` + `display:none`/`visibility:hidden`/`opacity=0`，`visibility` 可被后代恢复、
  `display` 不可）；`collisions` 改 O(n)；`LetterSite.cell` 变真、≥5 撞号给 `+N more`、文案改成可执行。
  40000 字母 1.31 s→0.06 s。测试 446→474。提交 `9117335`。
- 2026-11-24 **R47 文本节点的字符引用**：`src/dom.rs:80-96` 与 `src/convert/dom.rs:82-93` 的
  `Event::GeneralRef` 改调 `quick_xml::escape::unescape`，`Err` 走各自 Xml 变体（rc=1，与属性路径同前缀），
  两份 `resolve_reference` 删除；控制字符/非字符与 CDATA 口径不变。`37&#176;C` 修前 ink 1701/diff 1734 →
  修后 502/diff 0。测试 435→446。提交 `8aca0d1`。
- 2026-11-24 **R46 定义元素按引用者的 data-scale 走**：`src/transform/` 拆成 `defs.rs`/`links.rs`/`walk.rs`/
  `dispatch.rs`（`mod.rs` 只留声明）；`defs::split` 按引用图建 `direct` + 内部引用边、单调不动点，多 policy
  **传递地复制**定义（id 后缀 `--svg-grid-<tag>`；单 policy 不复制、无引用者按书写位置）；marker 有 viewBox
  ⇒ body verbatim；gradient/filter units 沿 href 继承（mask 明确不继承）；`normalize.rs` 按 `*Units` 门控。
  435 测试、5/5 变异红、892 次比对 0 差异。提交 `54fa2b6`。
- 2026-11-24 **R45 doctor 告警码漂移**：短语表搬到 `src/warnings/codes.rs`（`WARNING_CODES` 26-42、`classify`
  50-68），一码多短语；W4a 去反引号、W5 收两种形态、W7 改 `could not be rewritten, kept verbatim`、W3/W8
  移出表。`codes_tests.rs` 用真实 `describe_*` 回灌；W7 门禁复活。测试 415→419。提交 `196da47`。
- 2026-11-24 **R43–R44 单位属性与字母重号**：`src/transform/units.rs` 精确 token 语义（不 trim、非法值回退
  默认值），mask/filter/gradient/marker/pattern 全切换、tile 的 x/y/width/height 逐属性沿 href 解析（R43，
  394→406，`8846fa8`）；字母唯一性提升为整张图的性质——输入自带 ∪ 本次分配，同字母 ≥2 次且至少一处来自
  输入 ⇒ rc=2、不写产物、`--plan` 同 rc（R44，406→415，`4873156`）。
- 2026-11-24 **R42 转换器并入主库**：`convert/` crate → `src/convert/` 模块（28 个 `.rs` `git mv`、
  106 处 `crate::`→`crate::convert::`、删独立 `[[bin]]`/`Cargo.toml`/`main.rs`），
  `convert/tests/cli.rs` → `tests/selftest/convert_cli/mod.rs`；诊断前缀统一 `svg_grid convert:`。
  220 fixture / 58 组合 / `--plan` / `--report` 与 base 逐字节相同。测试 394（单 crate）。提交 `de1bfae`→`8f38435`。
- 2026-11-24 **R37–R41 defs 与 pattern 单位语义**：`prune_defs` 按引用保留 + 悬空引用 fail-closed（R37）、
  pattern 相位（tile 全映射 + body 只缩放不平移，R38）、obb 内容逐字保留（R39）、
  viewBox/严格 token（R40）、href 断链回退 user space + 重复 id 后者胜 + `patternUnits` 继承（R41）。
- 2026-11-23 **D5b–D5d W8 收尾（R34–R36）**：`describe_missing_font_error` 改为**链里任一名字可用（在库或
  通用族）即放行**，整链不可用才 rc=3 并点名整链 + 可用族 + 修法；错误行 200 字符硬上限
  （`MAX_ERROR_BODY`/`assemble_within`），W3 205→188、W8 239→194/195、W5 217→191；D5c 手算预算错 1
  （`"(available: "` 12 vs 真实 13）⇒ `available:` 整段丢失，D5d 改为**按 `format!` 真实长度反推预算**
  （`AVAILABLE_OPEN`/`truncate_body`，链回显优先于库列表）。提交 `e362c01`→`b55669e`（R34）、
  `e4660e8`→`5482421`（R35）、`98a173e`→`d32ab03`（R36）。
- 2026-11-22 **D1–D6 六缺陷（R27–R33）**：字形孔（≥3 子路径的填充 path 改展平多轮廓 `<path>`，R27/R28）、
  末行格子尺寸（行高 = `available_w / Σ全网格列权重`，R29）、默认近方形网格（R30）、网格下界校验 +
  `--verbose` 默认静音（R31）、`plot-grid` 自动 convert（R32）、labels 的 NULL/W3/W8/margin0（R33）。
  机制见 `ARCHITECTURE/{layout,warnings,convert}.md`；数字、变异与复现见账本与 `round-data/R27…R33`。
- 2026-11-22 **账本 reanchor**：`pitfalls-manager` 新增 `reanchor` 子命令（commit `bcb19c0`，已装）；
  锚点从 monorepo 改为 `[工具仓, monorepo]`，`check` 46 条悬空 → 0；备份在
  `/home/nihao/pitfalls-manager-backups/svg_grid-reanchor-20261122-221338/`。
- 2026-11-22 **用户验收审计（未改代码）**：对照 `INSTRUCTIONS.md` 的四条新要求逐条实测，全部未满足；
  三条根因已定位到 file:line（见 `# Known Issues` 前四条）。同时核实：`doctor` 57 pass / 0 fail / 1 skip、
  `cargo test` 207 项全绿、`~/.local/bin/svg_grid` 与 `target/release` 同 sha256（装的是最新构建）。
- 2026-10-06 **R26–R13**：参数面收敛 + `plot-grid --plan`（R26）；告警瘦身、单条 ≤198 字符（R25）；
  I/O 报错点名文件（R24）；Windows 安装面（R23）；`install.sh` 装 skill + `--check`（R22）；
  原生出图查证流程（R21）；指南补 `# CORE WORKFLOW`（R20）；自检合成 `tests/selftest/`（R19）；
  `convert` 并为子命令（R18）；`doctor` 子命令（R17）；修 paint server 被丢弃 + W9（R16）；
  `install.sh`（R15）；用户参数打崩工具 rc=2、退出码分档（R14）；字体库 `fontlib/` + W8（R13）。
- 2026-10-06 **R12–R11**：补 `round-data/R10/` 证据（R12）；`DECISIONS.md` 并入 `ARCHITECTURE.md`（R11）。
- 2026-09-28 **R10–R5**：指南规范源（R10）；HARNESS 归位（R9）；6 个 >20 KB 源文件拆分（R8）；
  `src/ink.rs`/`src/path.rs` 拆目录（R7）；`<path d>` 宽松化 + W6 微尘（R6）；主库纳入 `<path>`（R5）。
- 2026-09-27 及更早 **R4–R3**：W6 内容越界 + 死白带判据（R4）；按 7 篇反馈更新指南（R3）；
  更早按 10 篇反馈修阻塞点（7 修 / 3 判外部或用错）、组图路线反转为 `svg_grid` 优先、
  布局语义 cowplot 化五项、`convert/` 建 crate、`<image>`/`--png-dpi`。
# In Progress

- **R49（进行中）**：R46 对抗评审的 1 阻断 + 1 P1。**B1**：嵌套定义被克隆时产生重复 id
  （`<pattern id="P">` 内含 `<defs><linearGradient id="G">`，P 被 `xy`/`y` 两个引用者引用 ⇒
  `svg-grid-cell0-G` ×2、`svg-grid-cell0-G--svg-grid-y` ×3；机制 `src/transform/defs.rs:split_children`
  递归进每份拷贝时按内层完整 policy 集合重拆）。**P1**：`src/normalize.rs::scale_definition` 在 units 门控
  **之前**无条件缩定义元素自身的 `stroke-width`/`font-size`（obb pattern 的 `0.1`→`0.050`，xor 11520）。

# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的缺口（2026-11-23 实测）

**D1–D6 + D5b–D5d 全部已修（R27–R36）**，`INSTRUCTIONS.md` 的六条要求均已满足。剩下列 pre-existing，均已开报告：

1. **滤镜图元长度不随面板缩放**（`feMorphology radius`、`feGaussianBlur stdDeviation` 等；s=2 `xor 1136`）：
   `filter` 只重写 box，图元在 `primitiveUnits="userSpaceOnUse"` 下的长度量未处理；各向异性缩放无精确表达。
2. **pattern 的其余边界**：嵌套 `<pattern>`（`xor 11808`）、`patternTransform` 里的长度量（`1561/927`）、
   pattern 外的 `clip-path` 被 pattern body 引用（`20736/6264`）、pattern 内 `<text>`（resvg 不画）。
3. **`classify` 取表序第一个命中码**（`docs/PITFALLS/blocker-svg_grid-classify-first-match-masking.md`）：
   W9 行原样打印悬空 id、W7 行原样打印属性值 ⇒ 用户内容含表序更靠前的短语时，doctor 的 W9/W7 门禁
   **fail-open**（base FAIL → head PASS `['W4a']`）。同族：`src/warnings/rules_tests.rs:236-266` 的短语副本、
   `round-data/R17/test.py:44-46` 死变量、R45 使旧行号再漂但未记 `modify`。
4. **R19 证据脚本已 `retire --kind deprecated`**：它写死 190/60/57/73 用例数、7 个历史测试目录与独立二进制
   告警前缀，在 base 上也是红的（8 FAIL）；`verify` 与"旧证据全绿"的口径**不含 R19**。

## 其他未决项

- **`--rel-widths` 部分为 0 会静默产出 0 宽格子**（pre-existing，低优先）：如 `0,1` 时首列 cell 宽 0、面板不可见，
  与 `rel_*` 其余 fail-closed 行为不一致。
- **嵌套组图会累积"与目标网格的偏差"**（不是变形：每步 `Δ=0.00%`）。实测 A4 目标 1000×1414 的
  最小步骤链拼出 1532.48（**+8.4%**），其中 GH 行 1.445 倍行高（底行 3 格只放 2 个 ⇒ 每格 1.5 格宽）。
  中间步 margin 也会改变外层列宽比（默认 24 ⇒ A:BC = 1.951 而非 2.000；`--plot-margin 0` ⇒ 精确 2.000）。
  ⇒ **嵌套时中间步一律 `--plot-margin 0`**，且跨行块只能"近似"表达（无 span 能力）。
- **无跨格（span/merge）能力**：`src/cli/parse.rs` 的 FLAGS 只有 `nrow/ncol/rel-*/gap`。用户例子里的
  "A 占 2×2"只能靠嵌套近似（A 与 BC 横向组合时同宽比 2:1、`Δ=0`，但 ABC 纵横比 1.4682 vs 理想 1.4144）。
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
  "只有文字的面板"。**复刻图总高 5639 vs 原图 5764（约 2.2%）未 root-cause**：band/margin 模型差异。
- **`--rel-heights` 微调行高会纵向拉伸该行面板**（改变格子宽高比 ⇒ 会告警）：自然纵横比布局的必然结果，
  要既改行高又不变形只能改画布宽或面板纵横比。**垫片 `--input none` 的自然纵横比按 1:1 计**。
- **`AlignMode` 仍未使用**（R 参照逻辑全程不用 `align`，本版不做）。
- **版面可读性天花板**：一行 3 面板 + 固定画布宽 ⇒ 单面板 ~2.6 in、纸面字号 ~3.8 pt（低于 5–6 pt）；
  用户原版 S22 同行亦然，只能改版式或按目标尺寸重设计。**matplotlib 之外的方言未覆盖**（仍 fail-closed）。
- **path 双实现防漂移**（R5）：主库 `src/path/` 与 `src/convert/path.rs` 各一份。
  缓解：同容差、同语义、注释互指；改一侧必须同时看另一侧。
- **两副本指南**：**规范源 = `tools/svg_grid/docs/general-figure-guide/`**（改动只在这里做）；
  `tools/manuscripts_skills/general-figure-guide/` 是复制件、用 `rsync -a --delete` 覆盖过去
  （R10 裁决；账本两条路径的历史已由 `link` 组 L6–L8 并起来）。
- **`research/` 两份旧 `DECISIONS.md` 未迁**（`白藜芦醇胰腺癌` 61 624 B、`MASLD_NP` 5 342 B）：
  在各自的**本地仓库**里、属别的任务 ⇒ **待用户决定**是否一并并入其 `ARCHITECTURE.md`。

# Next Steps

1. **新会话从这里开始：先读 `docs/HANDOFF.md`**（§0 有 R27–R46 的提交表）。六缺陷、defs/pattern 语义、
   字母重号、doctor 告警码、定义元素缩放单位、文本字符引用、字母查重均已收口；**进行中 = R49**（R46 评审的
   嵌套克隆重复 id + normalize 定义元素自身表现属性）。之后消费剩余报告（`classify` 表序遮蔽），最后做
   交付前全量复核：`cargo test`（474）、`svg_grid doctor`
   57/0/1、`pitfalls-manager verify` rc=0、两副本指南 `diff -r`。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 →
   删驱动文档 → 若本轮有改名/移动/拆分，**提交后 `link`**（否则悬空自检会报出来）。
   **语义分支先证后写**：改判据前先读 `usvg` 源码 + 用参考渲染器（resvg/cairosvg）实测"哪一边对"，
   再写代码与期望值（R38–R41 连栽三次的教训）。
3. **（用户定）** Known Issues 里待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、两份旧 `DECISIONS.md`、span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
