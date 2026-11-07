# Current State

- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` 2026-09-28 R5 起；分界原则见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 110 项**（57 lib + 53 integration）全绿，`fmt`/`clippy -D warnings` 干净。
- **七条非致命告警 W1–W7**（只走 stderr，**退出码与产物字节都不变**，有测试锁定；正常路径 stderr 0 字节）：
  契约与清单见 `ARCHITECTURE/warnings.md`。
- **`convert/`**：R `svglite`/matplotlib/`pdftocairo -svg` 三种来源可入流水线；`<path>` 出形态决策表、
  `clipPath` 矩形化、非等比 `<image>` 烘框、字形孔保留、描边显式化、`style` 百分比解析。
  **测试 87 项**（76 lib + 11 CLI）全绿。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（机制见
  `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选路线 `02a` = `svg_grid`，
  回退 `02b` = cowplot；两副本 `diff -r` 为空）。**HARNESS 是两件套**（2026-10-06 起）：本文件
  + `ARCHITECTURE.md`（含 `# 已定型的决定` 与 `ARCHITECTURE/` 机制文档，索引见该文件）+ `docs/`；**不设 `DECISIONS.md`**。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**，工具已是 Rust 二进制、
  接口是 `pitfalls-manager <cmd>`），**R1–R19 已入账**；`link` 组 L1–L10；`round-data/R4…R19` 各有一份可跑证据
  （复发轮必须有；R10 的证据是后就地补的）。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）。**R3 起 `--height`/`--rel-*` 完全不再出现**；
  **R4 零告警通过**（六格 `sx=sy=1.0000`）。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已用本管线重做；**Figure 2**（矢量带 + 13 个位图
  panel）复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。

# Completed

（每条一行；细节在账本与 `ARCHITECTURE.md` §已定型的决定，不在此重复。）

- 2026-10-06 **R26**：**参数面收敛 + 出图几何由工具算**——删掉 18 个未文档化拼法（14 个下划线 + 4 个 kebab 变体），
  `--help` 与解析器双向一一对应（未知写法 `unknown flag`、rc=2）；新增 **`plot-grid --plan`**（只读、零写入）：
  打印总画布、每个输入一行 `canvas/cell/s/Δ`、以及可直接照抄的归一化命令，几何与真跑同源（`geom::cell_scale`）。
  指南流水线改用 `--plan`（两副本一致）。证据 R26 = 22 条（含 R4–R17 全回归，814 s）。
- 2026-10-06 **R25**：**告警瘦身**——单条从最长 722 → **198 字符**（全部码实测 ≤200），
  公共因果抽成**每次运行只打一行**的 `svg_grid: note: …`（无告警时 0 行），`N=1` 去掉 `(s)` 计数包装；
  触发条数/码集合/退出码不变。证据 R25 = 96 条（5 条 SKIP 的根因是 R11 的扫描范围，已一并改成只扫 git 跟踪文件）。
- 2026-10-06 **R24**：**I/O 报错点名文件**——`SvgGridError::Io` 带上 `Option<PathBuf>`（`at()` 构造），
  读输入（含 `.svgz` 解码）、写 `.svg`/`.svgz`/`.png`/`.avif`、读语料清单/目录全部走它，形状统一为
  `I/O error at <path>: …`（PNG 也从 `rasterization error` 归到 I/O）；退出码分档不变。证据 R24 = 21 条。
- 2026-10-06 **R23**：**Windows 安装面**——数据根解析抽成 `src/home.rs`（**`HOME` → `USERPROFILE`**，空值跳过；
  字体与语料共用同一实现），`--help`/`doctor` 的文案改说 `<home>/svg_grid` 并注明 `%USERPROFILE%`；
  新增 `docs/install-windows.md`（无脚本、不预编译：本机 `cargo build --release` → exe 进 PATH → 复制
  Arial/Times 进 `%USERPROFILE%\svg_grid\fontlib` → 复制 `tests\fixtures` → skill 进 `.commandcode\skills` →
  四条验收 + 常见问题）；README 补「安装」段（Linux/macOS 一条命令 / Windows 指路）。证据 R23 = 30 条。
- 2026-10-06 **R22**：`install.sh` 从"装二进制 + 数据"变成"装二进制 + **随工具发布的 skill** + 数据"——
  白名单拷贝 `SKILL.md`+`references/` 到 `~/.commandcode/skills/general-figure-guide/`（`--project` ⇒ 项目级），
  幂等（`diff -r` 一致则 `up to date`）、原子替换、`--no-skill`/`--dry-run`；新增 **`--check`**（只读自检：
  二进制 sha256、skill 逐字节且无多出开发侧文件、location）。证据 R22 = 25 条；回填 **R3** 的可跑证据
  （11 条语义锚点 + 两副本一致），账本 `check` 至此为 0 提示。
- 2026-10-06 **R21**：原生出图从"查一张表"升级为**查证流程**（`00_native_plots.md` §0：**认包 → 查本仓 `tools/` →
  查官方资料/网络 → 确认无才自绘**），对照表降级为"快捷答案、不是上限"；`# native: none` 的原因必须带**两处查证痕迹**。
  证据 R21 = 53 条。
- 2026-10-06 **R20**：组图指南补**主干**——`SKILL.md` 立 `# CORE WORKFLOW` 五步（①panel 来源=原生出图
  ②按格子出矢量 panel ③组图 ④vision ⑤交付核对），章节与主干同名同序；新增 `00_native_plots.md`（95 条函数对照 +
  四类例外 + `# native:` 留痕）；`05_checklist.md` 加 `grep` 可查的来源判据。证据 R20 = 44 条。
- 2026-10-06 **R19**：**自检合成一个测试目标** `tests/selftest/`（旧 `compose_tests/` + `doctor_tests/` 合并，
  用例数 190 → 190 一条未丢；内部按主题分模块）；`convert` 的 stderr 前缀改为**随调用方**
  （`svg_grid convert:` / 独立二进制 `svg_grid_convert:`）。证据 R19 = 34 条。
- 2026-10-06 **R18**：**只交付一个二进制**——`convert` 成为 `svg_grid convert` 子命令（主 crate 以库依赖用 `convert/`，
  产物与旧二进制逐字节相同）；`doctor` 第一段改进程内调用（不再找外部二进制、`exit 3` 分支删除）；
  `install.sh` 只装 `svg_grid` 并清掉旧装的 `svg_grid_convert`；skill 里的命令名一并迁移。
  证据 R18 = 43 条（含 58×4 逐字节对照 + 回归 R4–R17）。
- 2026-10-06 **R17**：`doctor` 子命令——一条命令跑整集语料（两段管线 + 判据 + `--only/--jobs/--json/--keep`）；
  全量 **57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 的 `why` 计 SKIP 并打印）。
  这是此后"改完先跑"的默认回归入口（`ARCHITECTURE/testing.md`）。
- 2026-10-06 **R16**：语料基线测出两条**静默掉墨水**的缺陷——`<defs>` 里的 paint server
  （`linearGradient`/`radialGradient`/`pattern`）被转换器整段丢弃，`url(#…)` 悬空 ⇒ 渐变柱/hatch 柱不画。
  修：转换器保留 paint server、组图侧不碰其坐标域、新增 **W9**（悬空引用出声）。
  墨水恢复 0.0672→0.6809、0.2180→0.4245。
- 2026-10-06 **R15**：安装——`install.sh`（release 编译两个 crate → 装 `~/.local/bin` → 装字体到数据根；
  幂等、`--dry-run` 零写入、英文输出、失败一句 + `Fix:`）。**每次改完代码都要重跑一次**（`ARCHITECTURE/install.md`）。
- 2026-10-06 **R14**：修掉"用户参数能打崩工具"那批——`--ncol/--nrow 0`、`--avif-quality` 越界的 panic ⇒ 用法错(2)；
  退出码分档 0/1/2/3；`--help`/`-h`/`--version` 修好（stdout + rc=0，覆盖 28 flag，+12 个 CLI 单测）；
  字体目录改**运行时**解析；`transform.rs` 拆目录；ARCHITECTURE 里账本路径残留清零。
- 2026-10-06 **R13**：字体库 `fontlib/`——字体只从该目录读（`load_system_fonts` 已删、`system-fonts` feature 已去掉）、
  面板字母默认 **Arial**、`--list-fonts`/`--font-dir`、缺族 **W8**、空库 fail-closed；skill 补字体文案（两副本一致）。
  同时把 `src/lib.rs` 按体积口径拆出 `src/labels.rs`（20 044 → 17 752 B）。
- 2026-10-06 **R12**：消费阻塞点报告「R10 缺可跑证据」——自检是真阳性（不是工具缺陷），按 §4.3 **就地补**
  `round-data/R10/`，并给 R11 一并建证据。
- 2026-10-06 **R11**：`DECISIONS.md` 并入 `ARCHITECTURE.md` §已定型的决定后删除（根 `AGENTS.md` 已把
  DECISIONS 移出 HARNESS）；跨工具陈旧引用同步为两件套。
- 2026-09-28 **R10**：指南规范源裁决 = `docs/general-figure-guide/`（另一份是复制件，`rsync` 覆盖）；
  账本两条路径的历史用 `link` L6–L8 并轨。
- 2026-09-28 **R9**：HARNESS 归位——`ARCHITECTURE/` 拆出六份机制文档、父文档瘦身、`PROGRESS` 去历史、
  `DECISIONS` 去设计文档；skill 按"只写现状 + 不列告警"瘦身；转换器 `<style>` 告警补齐"怎么处理"。
- 2026-09-28 **R8**：把 6 个 > 20 KB 的源文件全部行为保持型拆开（并修掉 `include!` 造成的 fmt 盲区）。
- 2026-09-28 **R7**：`src/ink.rs`、`src/path.rs` 行为保持型拆成目录；触发账本 `link` L1/L2。
- 2026-09-28 **R6**：`<path d>` 宽松化（畸形/非有限 ⇒ W7 警告 + 原样透传）、W6 微尘提示、
  删掉恒真的检查 2、镜像等比 + 旋转弧只登记。
- 2026-09-28 **R5**：主库纳入 `<path>`（"最小的完备表达能力"分界原则）。
- 2026-09-27 **R4**：新增 W6（内容越出自身画布）；死白带判据修正并落成唯一脚本。
- 2026-09-27 **R3**：按 7 篇出图手感反馈更新指南（未改一行代码）。
- 2026-09-20：按 10 篇真实反馈修阻塞点（7 修 / 3 判外部或用错）。
- 2026-09-17：组图路线反转为 `svg_grid` 优先、cowplot 降回退；混合（svg + 位图）端到端走通。
- 2026-09-16：布局语义 cowplot 化五项；删除混合栅格-矢量 composer；`convert/` 建 crate；
  `<image>`/`raster-images`/`--png-dpi`；五条告警落地；批次重建 12 张成品图。

# In Progress

- 无。

# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

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

1. **（用户决定）混合路径既已走通，可继续往下推**：Figure 4/5 或 S2–S4（含更多 ChimeraX 位图面板的版式）。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 → 删驱动文档 → 若本轮有改名/移动/拆分，
   **提交后 `link`**（否则悬空自检会报出来）。取回已删报告：`query` 会打出
   `git -C <repo> show <commit>:<path>`。
3. **（用户定）** 上面 Known Issues 里的几项待裁决：W6 容差、RDKit 方言重测、`research/` 两份旧
   `DECISIONS.md` **未迁**。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
