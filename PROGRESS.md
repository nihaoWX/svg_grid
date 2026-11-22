# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：六个缺陷已全部修复（R27–R34）——
  D1 字形孔、D2 末行格子尺寸、D3 默认网格、D6 网格校验 + `--verbose` 静音、D4 自动 convert、D5 labels/W3/W8
  （含 D5b 的 W8 回退链修复）。**
  余下三项 pre-existing，均已开报告：`<defs>` 内 mask/marker 被丢弃、组合图字母重号静默、doctor 告警码漂移。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` 2026-09-28 R5 起；分界原则见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 236 项**（81 lib + 59 main + 96 selftest）全绿，`fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **告警 W1–W9**：W1–W7/W9 **非致命**（只走 stderr、**默认静音**，`--verbose` 才展开；退出码与产物字节都不变）；
  **W3（给已带字母的输入再加非空字母）rc=2**、**W8（缺标签字体族）rc=3**，两者都**不写产物**、不依赖 `--verbose`；
  正常路径 stderr 0 字节；`convert` 子命令同样受静音控制：
  契约与清单见 `ARCHITECTURE/warnings.md`。
- **`convert/`**：R `svglite`/matplotlib/`pdftocairo -svg` 三种来源可入流水线；`<path>` 出形态决策表、
  `clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、`style` 百分比解析。**测试 100 项**（88 lib + 12 CLI）全绿。
  **2026-11-22 D1 已修**：≥3 子路径的填充 path 不再合并成 `<polygon>`，改为展平多轮廓 `<path>`（M/L/Z），
  字形孔不再填死（R27 + R28）。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（机制见
  `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选路线 `02a` = `svg_grid`，
  回退 `02b` = cowplot；两副本 `diff -r` 为空）。**HARNESS 是两件套**（2026-10-06 起）：本文件
  + `ARCHITECTURE.md`（含 `# 已定型的决定` 与 `ARCHITECTURE/` 机制文档，索引见该文件）+ `docs/`；**不设 `DECISIONS.md`**。
  **`INSTRUCTIONS.md` 是用户手写、只由用户改**，与其余文件冲突时以它为准。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**），**R1–R33 已入账**；
  `link` 组 L1–L56；`round-data/R3…R33` 各有一份可跑证据。**2026-11-22 已 reanchor**：锚点 =
  `[tools/svg_grid, monorepo]`（两个都要，旧 commit 只在 monorepo 里），`check` 0 条、`ids` 不变。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）。**R3 起 `--height`/`--rel-*` 完全不再出现**；
  **R4 零告警通过**（六格 `sx=sy=1.0000`）。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已用本管线重做；**Figure 2**（矢量带 + 13 个位图
  panel）复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。

# Completed

（每条一行；细节在账本与 `ARCHITECTURE.md` §已定型的决定，不在此重复。）

- 2026-11-23 **D5b W8 回退链（R34）**：`describe_missing_font_error` 原来取"链里第一个非通用名"判是否在库，
  把 CSS 回退链当单族名 ⇒ 链首不在库就 rc=3。改为**链里任一名字可用（在库或通用族）即放行**，
  只有整条链都不可用才 rc=3 并点名整链 + 可用族 + 修法；`font-family` 原样交给渲染器。
  新增 `MAX_ERROR_BODY`/`assemble_within`，W3 205→188、W8 239→194/195、W5 217→191（全部 ≤200 字符）。
  根 236 tests（81+59+96）/ convert 100 / doctor 57/0/1；R8–R14 + R28–R34 全绿、R34 证据 38 PASS。
  提交 `e362c01`→`b55669e`（删报告），证据 `docs/PITFALLS/round-data/R34/`。
- 2026-11-22 **D5 labels / W3 / W8 / margin 0（R33）**：裸 `--labels` 的 AUTO **跳过 NULL 槽**（5 槽 `A,B,–,C,D`）；
  显式列表**按槽位**消费（给 NULL 槽写字母就画、`""` 跳过）。W3 升硬错误：给已带 `data-svg-grid-label`
  的输入再赋非空字母 ⇒ rc=2、不写产物、文案点名 input#/字母，`--plan` 同 rc；`--labels ,C` 仍放行。
  W8 升硬错误：缺标签字体族在 SVG/SVGZ/PNG/AVIF **一律 rc=3**、不写产物、文案列可用族与修法，`--plan` 不受影响。
  `--plot-margin 0` 时字母 baseline 夹到画布内（`y ≥ 0.75em`），W5 改为"已内移"说明。顺带把 `src/lib.rs`
  20182→18914 B、`tests/selftest/warnings/mod.rs` 20781→14156 B（拆 `src/label_conflicts.rs`、
  `tests/selftest/warnings/ink.rs`），R8 体积门禁复绿。根 233 tests / convert 100 / doctor 57/0/1，
  R8–R14 + R28–R32 全绿、R33 证据 19 PASS。提交 `e9425d2`→`398a21e`（删报告），证据 `docs/PITFALLS/round-data/R33/`。
- 2026-11-22 **D4 自动 convert（R32）**：`plot-grid`/`--plan` 对缺 `data-panel-box="main"` 的输入进程内
  `svg_grid_convert::convert`（纯内存、每条最多一次），未打标签 svglite/matplotlib 输入从 rc=2 变为 rc=0，
  产物与"先手动 convert 再 plot-grid"逐字节相同；已带 panel-box 的输入输出与父提交逐字节相同
  （SHA-256 `ca2f6503…`）；不支持的 `<symbol>` rc=2 且文案点名"auto-conversion failed"、不写输出；
  原图 bytes/SHA-256/mtime 前后不变；自动转换诊断默认静音、`--verbose` 可见。
  旧证据 R4–R9/R13 补 `--verbose`、R14 改测零宽 canvas 并新增"未打标签 ⇒ rc=0"断言。根 232 tests /
  convert 100 / doctor 57/0/1；提交 `162c0cd`→`79cf9e5`→`a2d5fb5`（删报告），证据 `docs/PITFALLS/round-data/R32/`。
- 2026-11-22 **D6 网格校验 + `--verbose`（R31）**：显式 `ncol/nrow` 时新增下界校验
  `(rows-1)*cols < count`（`src/layout.rs:93-123`）——5 图 `--nrow 4`（4×2）、5/6/3 图 3×3 均 rc=2，
  文案点名空行、给 `--nrow` 建议值与需补的 `--input NULL` 数；合法短末行（5 图 `--ncol 2`）、默认网格、
  显式 NULL 补齐 4×2 仍 rc=0；`--plan` 同 rc/同文案且不写文件。`--verbose` 从零新增（`plot-grid` 与 `convert`
  都接受）：默认 stderr 只留报错，W1–W9 与聚合 `note:` 静音，`--plan` 的 stdout `note:` 不受影响；
  doctor 内部显式开诊断保持 57/0/1。根 223 tests（81+59+83）/ convert 100 / R31 证据 50 PASS。
  提交 `3311eec`→`676d82d`（删报告），证据 `docs/PITFALLS/round-data/R31/`。
- 2026-11-22 **D3 默认网格（R30）**：不给 `--ncol/--nrow` 时 `cols=ceil(sqrt(count))`、`rows=ceil(count/cols)`
  （count 含显式 NULL：1→1×1、2→1×2、3/4→2×2、5/6→2×3、7–9→3×3、10–12→3×4、13–16→4×4、17→4×5），
  末行不满合法、末槽不生成 cell。5 图默认 `1000×533.333`、格全 `333.333×266.667`；15 图默认 `1000×800`、格 `250×200`。
  默认输出与显式近方形网格逐字节相同；1/2 图默认与旧 `--ncol 1`/`--ncol 2` 逐字节相同。
  新增 4 条测试；根 217 tests / convert 100 / doctor 57/0/1。提交 `e09c33b`→`dc9a900`（删报告），证据 `docs/PITFALLS/round-data/R30/`。
- 2026-11-22 **D2 末行格子尺寸（R29）**：`src/layout.rs` 行高信号改为 `available_w / Σ(全网格列权重)`
  （列权重 = `--rel-widths` 或该列第一张非 NULL 面板的纵横比），删除按"本行实际图数"求和的 `row_aspect_sums`；
  短末行与满行同尺寸（5 图 `--ncol 2` ⇒ `1000×1200`、全 `500×400`），满行形状几何逐字节不变
  （4/2×2、6/2×3、9/3×3、16/4×4、15/5×3 与父提交 `cmp` 相同）。15 图 `--ncol 4`（4+4+4+3）为有意行为变更。
  新增 6 条测试（含 `--rel-widths 1,2` vs `2,4` 等价与变异可捕获）；根 213 tests / convert 100 / doctor 57/0/1。
  提交 `66fba3c`（D1 注释）→`cf76029`→`f8be8ef`→`8b08524`（删报告），证据 `docs/PITFALLS/round-data/R29/`。
- 2026-11-22 **D1 字形孔（R27 + R28）**：convert 对 ≥3 子路径的填充路径不再合并成单 `<polygon>`
  （隐式闭合把 bridge 连成闭环、内孔被填死），改为展平多轮廓 `<path>`；N≤2 保留原 polygon。
  R28 收紧 self-check 白名单（只认与输出序列化一致的展平 path；mask/marker 内 `<path>` 一律判残渣）、
  拆分填充半清干净 stroke 残留、`--report` 计入 `path`。
  实测：`text_4` 标签组墨量 17,726 / 坏 28,637 / 新 17,731 px；220 fixture 仅 2 个等价 byte 变化（渲染 XOR=0）。
  验证：根 207 tests、convert 100 tests、`doctor` 57/0/1。提交 `1e05ce1`→`3a9747a`（删报告）、
  `1da48f0`→`64e4551`（删报告），证据 `docs/PITFALLS/round-data/R28/`。
- 2026-11-22 **账本 reanchor**：`pitfalls-manager` 新增 `reanchor` 子命令（commit `bcb19c0`，已装）；
  锚点从 monorepo 改为 `[工具仓, monorepo]`，`check` 46 条悬空 → 0，`query`/`ids`/`record` 均验证通过；
  备份在 `/home/nihao/pitfalls-manager-backups/svg_grid-reanchor-20261122-221338/`。
- 2026-11-22 **用户验收审计（未改代码）**：对照 `INSTRUCTIONS.md` 的四条新要求逐条实测，全部未满足；
  三条根因已定位到 file:line（见 `# Known Issues` 前四条）。同时核实：`doctor` 57 pass / 0 fail / 1 skip、
  `cargo test` 207 项全绿、`~/.local/bin/svg_grid` 与 `target/release` 同 sha256（装的是最新构建）。
- 2026-10-06 **R26**：参数面收敛（删 18 个未文档化拼法）；新增 `plot-grid --plan`（只读、几何与真跑同源）。
- 2026-10-06 **R25**：告警瘦身（单条最长 198 字符）；公共因果抽成一行 `note:`。
- 2026-10-06 **R24**：I/O 报错点名文件（`I/O error at <path>: …`）。
- 2026-10-06 **R23**：Windows 安装面（`src/home.rs` + `docs/install-windows.md`）。
- 2026-10-06 **R22**：`install.sh` 增装随工具发布的 skill + `--check` 只读自检。
- 2026-10-06 **R21**：原生出图升级为查证流程（认包 → 查本仓 tools → 查官方 → 无才自绘）。
- 2026-10-06 **R20**：指南补主干 `# CORE WORKFLOW` 五步 + `00_native_plots.md`。
- 2026-10-06 **R19**：自检合成一个测试目标 `tests/selftest/`（190 条未丢）。
- 2026-10-06 **R18**：只交付一个二进制——`convert` 成为 `svg_grid convert` 子命令。
- 2026-10-06 **R17**：`doctor` 子命令（两段管线 + 判据 + `--only/--jobs/--json/--keep`）。
- 2026-10-06 **R16**：修 paint server 被丢弃（新增 W9）。
- 2026-10-06 **R15**：`install.sh`（release 编译 → `~/.local/bin` → 字体到数据根）。
- 2026-10-06 **R14**：修"用户参数能打崩工具"（rc=2）、退出码分档、`--help` 契约、字体目录运行时解析。
- 2026-10-06 **R13**：字体库 `fontlib/`（缺族 W8、空库 fail-closed、字母默认 Arial）。
- 2026-10-06 **R12**：补 `round-data/R10/` 可跑证据。
- 2026-10-06 **R11**：`DECISIONS.md` 并入 `ARCHITECTURE.md` 后删除。
- 2026-09-28 **R10**：指南规范源 = `docs/general-figure-guide/`（另一份是复制件）。
- 2026-09-28 **R9**：HARNESS 归位（`ARCHITECTURE/` 拆六份机制文档、`PROGRESS` 去历史）。
- 2026-09-28 **R8**：6 个 > 20 KB 源文件行为保持型拆开。
- 2026-09-28 **R7**：`src/ink.rs`、`src/path.rs` 拆成目录。
- 2026-09-28 **R6**：`<path d>` 宽松化（W7）、W6 微尘提示。
- 2026-09-28 **R5**：主库纳入 `<path>`（最小完备表达能力）。
- 2026-09-27 **R4**：新增 W6（内容越出自身画布）；死白带判据。
- 2026-09-27 **R3**：按 7 篇出图手感反馈更新指南。
- 2026-09-20：按 10 篇真实反馈修阻塞点（7 修 / 3 判外部或用错）。
- 2026-09-17：组图路线反转为 `svg_grid` 优先、cowplot 降回退。
- 2026-09-16：布局语义 cowplot 化五项；`convert/` 建 crate；`<image>`/`--png-dpi`。

# In Progress

- 无进行中轮次（D1–D6 + D5b 全部完成，R27–R34）。下一轮 = 消费 `docs/PITFALLS/blocker-convert-defs-mask-dropped.md`（R35）。

# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的缺口（2026-11-23 实测）

**D1–D6 + D5b 全部已修（R27–R34）**，`INSTRUCTIONS.md` 的六条要求均已满足。剩三项 pre-existing，均已开报告：

1. **`<defs>` 内 mask/marker 被丢弃**（待消费，下一轮 R35）：`convert/src/translate/uses.rs:176` 的 `prune_defs`
   的 `REFERENCE_TARGETS` 不含 `mask`/`marker` ⇒ `<defs>` 内的 mask/marker 被整块丢弃、引用悬空、静默丢遮罩/箭头
   （自测：半遮罩源图黑墨 6,396 px → 产物 12,800 px；marker 变体让 cairosvg 渲染时抛 AttributeError）。
   报告：`docs/PITFALLS/blocker-convert-defs-mask-dropped.md`（含 2026-11-23 追加的 maskUnits/markerUnits/self_check 三层实测）。
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

1. **新会话从这里开始：先读 `docs/HANDOFF.md`**（2026-11-22 交接）——它把规则、六个缺陷
   （D1–D6，带 `file:line` 与复现）、验证入口、账本锚点的坑和修复顺序都写全了。
   **D1–D6 + D5b 已全部完成**（R27–R34）；下一步消费 `docs/PITFALLS/blocker-convert-defs-mask-dropped.md`（R35），
   再做一次交付前全量复核（测试 / doctor / R8–R14 / 两副本 diff / 账本 check）。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 → 删驱动文档 → 若本轮有改名/移动/拆分，
   **提交后 `link`**（否则悬空自检会报出来）。取回已删报告：`query` 会打出
   `git -C <repo> show <commit>:<path>`。
3. **（用户定）** 上面 Known Issues 里的几项待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、`research/` 两份旧
   `DECISIONS.md` **未迁**、是否引入 span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
