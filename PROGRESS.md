# Current State

- **主库** `tools/svg_grid`：纯 SVG 组图器。相对 `98fd91e` 的净增量：`<image>` 坐标适配、resvg
  `raster-images`、`--png-dpi`、认 `style` 声明的长度、字母默认值（裸传 `--labels`=AUTO / 位置在面板
  左上外侧 / `--label-size auto`）、**布局五项**（`--height` 可省、纵横比进布局、字母带自动预留、
  `--input none` 垫片、`rel_*` fail-closed）、以及**五条非致命告警**（见下）。
  **50 项测试**（17 lib + 33 integration）全绿，fmt/clippy 干净。
- **非致命告警（fail-loud）**：W1 输入被缩放 ≠1（>0.5%，可抓外层漏给 `--plot-margin 0` 的 0.97%）、
  W2 格子被拉伸（>1%）、W3 给组合图重复加字母、W4a/W4b 字母个数不符/重复、W5 字母会被裁。
  只走 stderr，**退出码与产物字节都不变**（有测试锁定）。`compose_s22.py` 全流程 stderr 为 0 字节。
- **convert**：R(svglite) 与 matplotlib 双方言；描边图元显式化 + `style` 百分比解析。65 项测试全绿。
- **文档**：`general-figure-guide/references/02a_svg_grid_assembly.md` 已是可照做的手册（含"跑完必须读
  stderr"一节）；**route 已反转为 svg_grid 优先**——`02a` = 首选路线（全 R 同会话也走），
  `02b_cowplot_assembly.md` = 回退路线（判据仅三条）；`ARCHITECTURE.md` 含"设计参照"与"非致命告警"两节；
  `docs/PITFALLS/` 两篇。副本 `docs/general-figure-guide/` 与规范源逐字节一致。
- **黑盒验证**：四轮。R1 暴露 12 处隐含知识；R2 通过（自己找到逐行 normalize 的路线）；
  **R3 更丝滑——`--height`/`--rel-*` 完全不再出现**；**R4 干净通过——`plot-grid` 零告警**，
  它按 02a 提示自己推出"逐面板各自归一化到格宽"，**六个格子 `sx=sy=1.0000`**（含 E），
  逐行显式给字母、外层 `--plot-margin 0`、并读 stderr。
  产物分别见 `tests/s22_repro/out/skilltest{,2,3,4}/`。

- **混合（svg + 位图）已端到端走通**：正图 **Figure 2**（1 个矢量带 + 13 个位图 `<image>` 包装）用本管线
  复刻成功——4961×7016 @600dpi（正好 A4 竖版，高由工具推出）、**14 格 `sx=sy=1.00000`**、
  **四次 `plot-grid` 调用 stderr 全为 0**。与现有 `Figure 2.png` 实质一致（差异仅来自
  pcolormesh vs imshow、resvg vs Agg 的字形栅格化）。测试资产 `tools/svg_grid/tests/fig2_mixed/`。

# Completed

- 2026-09-17 **skill 组图路线反转为 svg_grid 优先**（用户批准）：`general-figure-guide` 的 `02a` 定为
  **首选路线**（全 R 同会话也走），`02_cowplot_assembly.md` → **`02b_cowplot_assembly.md` 回退路线**
  （判据仅三条：convert 方言 fail-closed / 工具不可用 / 拿不到独立 `.svg`）；SKILL.md 的规则表、
  新增「组图路线」节、文档索引、反面清单同步；`04_manuscript_integration.md` §1.1 改为 svg_grid 默认。
  规范源与副本 `docs/general-figure-guide/` 逐字节一致（`diff -r` 为空）
- 2026-09-17 **混合组图（svg + 位图）端到端走通**（`582bf06` 造面板 / `64802aa` 组图 / `d15efb5` README）；
  同期修正 `convert/README.md` 里过期的"拒绝 `<image>`"说法（实测转换器早已接受，**纯 `<image>` 包装**
  也能过，由我独立复核），并在 02a 新增"位图 panel（svg + 位图混合组图）"一节
- 2026-09-17 指南修正（用户批准）：`general-figure-guide/references/01_canvas_layout.md` 的 panel 内标题口径
  与 `05_checklist.md` 对齐；字重判据补"只对同一字母成立"；`§3` 的"放大源图字号"补量化边界（不得超过 `s`）。
  同期把这批改动记入本 HARNESS（本文件的 Known Issues 与 `DECISIONS.md`）。
- 2026-09-16/17 字母规则入 DECISIONS（只加在"拥有面板为直接输入"的那一层；组合图不加）
- 2026-09-16/17 五条非致命告警（`6c00f92` 收集器 / `2e702a4` 缩放与拉伸 / `234dc83` 字母相关）
- 2026-09-17 决策改写：默认零变形 + 不一致改为告警（推翻原"不加告警"）（`5fd297d`）
- 2026-09-16 布局语义 cowplot 化五项（`bfee815`/`93989cd`/`6d8295c`/`4ae0487`/`919ec42`）
  + `compose_s22.py` 改为省略 `rel_*`（`b83dda0`）
- 2026-09-16 设计参照入 HARNESS（`5fc72d3`）；02a 多轮按实测补齐（`3e27fb9`/`7655449`/`5fd297d`）
- 2026-09-16 主库认 `style` + convert 补 style + 字母默认值（`f619da8` / `dde9d46` / `c2c6f9a`）
- 此前各条：matplotlib 方言翻译、`<image>` 适配、resvg `raster-images`、`--png-dpi`、模块拆分、
  回退混合 composer 等（`0350c56`→`2b2ced8`→`e7ed509`→`c7017ac`→`823cdb8`→`ed26e09`→`0eda564`
  →`d77477e`/`8322a29`/`3cadade`/`48030cf`→`19519df`→`61d6483`→`8a6a436`→`8455380`）

# In Progress

- 无。

# Known Issues

- **合成时面板内容不再被面板画布裁剪**：组图器把面板内容**内联**进 `<g data-svg-grid-cell=…>`，没有嵌套
  `<svg>` 视口 ⇒ 超出面板画布的元素（实测：matplotlib panel 的 colorbar 标签在 panel 本地 x≈4749 >
  画布宽 4661）在原栅格成品里被裁、在本管线里会**渲染到外边距**。本例无害，但密集版式下可能压到相邻面板；
  若要严格复刻栅格语义，需在 cell 上补 clip（尚未做）。
- **固定 `--plot-margin` 时"溢出字母带但没出画布"的字母会被静默裁**：现有 W5 只覆盖"越出画布"。
  规避：给 margin 时按 `gap + 大写高(字号)` 自行留足字母带。
- **`convert` 对 RDKit `MolDraw2DSVG` 方言仍 fail-closed**（纯 `<path>`；实测报"37 个不支持节点"）⇒
  2D 结构式目前只能以位图进入组图；若要矢量化需单独评估该方言缺哪个构造。
- **指南内部口径已对齐**（本次修正，用户批准）：`01_canvas_layout.md` 原说"panel 内标题不要"，与
  `05_checklist.md`"标题在坐标轴上方是可以的"矛盾 → 已统一为"**源面板自带、位于坐标轴上方**的标题保留；
  禁止落在数据区内部、图外脚注/来源说明、以及组图阶段新增的任何 panel 内文字"。同时 `01` 的字重判据补上
  限定（`0.27/0.37` 只是 `A` 的实测值，判字重必须**拿同一个字母自己对比**——常规体 `B/D/E` 填充率
  0.36–0.43，按 0.37 会误判成粗体），`01 §3` 的"放大源图字号"补上量化边界（**不得超过 `s`**）。
- **跨行字号不一致**：逐行按各自格宽归一化（保证 `s=1`）⇒ 不同行的纸面字号不同
  （S22 实测 5.85 / 3.96 / 9.10 pt）。这是 `s=1` 的代价，已在 02a 写明。
- （`tools/svg_grid/` 下的嵌套空 git 仓库是**用户有意建的**（准备单独开源该工具），已在
  `ARCHITECTURE.md` 的结构一节记为事实；本工具的开发与提交一律用**根仓库**。）
- **`--rel-heights` 微调行高会纵向拉伸该行面板**（改变格子宽高比 ⇒ 面板变形，工具会告警）：
  这是自然纵横比布局的必然结果，已在 02a 写明；要既改行高又不变形只能改画布宽或面板纵横比。
- **垫片 `--input none` 的自然纵横比按 1:1 计**（R 的 `NULL` 靠 `rel_widths` 定份额）。
- **无"文字条"能力**（R 用 `ggdraw + draw_label`）：主库不生成文字，标题条需先做成"只有文字的面板"。
- **复刻图总高 5639 vs 原图 5764（约 2.2%）未 root-cause**：band/margin 模型差异，面板级已对齐。
- **`AlignMode` 仍未使用**（按 R 参照逻辑本版不做）。
- **字体相对大小依赖 `s ≈ 1`**：协议不缩放 `font-size`；落尺度靠 `--normalize-input-max-side`
  （现已能缩 `style` 里的字号与线宽）或出图端按目标尺寸画。见 PITFALLS。
- **版面可读性天花板**：一行 3 面板 + 固定画布宽 → 单面板 ~2.6 in、纸面字号 ~3.8 pt（低于 5–6 pt）；
  用户原版 S22 同行亦然，只能改版式或按目标尺寸重设计。
- matplotlib 之外的方言（Inkscape/Illustrator 导出等）未覆盖。

# Next Steps

1. **（用户决定）混合路径既已走通，可继续往下推**：Figure 4/5 或 S2–S4（含更多 ChimeraX 位图面板的版式）。
2. 按需处理上面三条新发现（cell 级 clip、字母带告警、RDKit 方言矢量化）。
3. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力（R 的 `ggdraw + draw_label`）。
