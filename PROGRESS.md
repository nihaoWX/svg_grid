# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：六缺陷全部修复（R27–R36）**；随后
  R37–R41 收掉 `<defs>` mask/marker/filter 与 pattern 单位语义，R42 把转换器并入 `src/convert/`，
  R43–R60 收掉单位属性、字母重号、告警码、定义元素缩放、字符引用、嵌套克隆 id、非 XML 1.0 字符、
  诊断文案偏移、paint server transform 共轭、`!important` 级联与零 alpha、滤镜图元长度与定义体字号、光源位置因子、溢出写入、门禁与证据卫生。余下 pre-existing 见 Known Issues。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` R5 起；分界见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 643 项**（351 lib + 59 main + 233 selftest，R42 起单 crate）全绿，
  `fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **告警 W1–W9**：W1–W7/W9 **非致命**（只走 stderr、**默认静音**，`--verbose` 才展开；退出码与产物字节都不变）；
  **W3（给已带字母的输入再加非空字母）rc=2**、**W8（缺标签字体族）rc=3**，两者都**不写产物**、不依赖 `--verbose`；
  正常路径 stderr 0 字节；`convert` 子命令同样受静音控制（契约见 `ARCHITECTURE/warnings.md`）。
- **`src/convert/`（R42 起并入主库，子命令 `svg_grid convert`）**：R `svglite`/matplotlib/`pdftocairo -svg`
  三种来源可入流水线；`<path>` 出形态决策表、`clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、
  `style` 百分比解析、`<defs>` mask/marker/filter 与 pattern 单位语义（R37–R41）。
  **D1 已修**：≥3 子路径的填充 path 改为展平多轮廓 `<path>`（M/L/Z），字形孔不再填死。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查
  （死白带 / 外框 2 px 墨迹 = 0），`--selftest` 可自证；判据的**唯一来源**（见 `ARCHITECTURE/check-deliverable.md`）。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选 `02a` = `svg_grid`，回退 `02b` = cowplot；
  两副本 `diff -r` 为空）。**HARNESS 是两件套**：本文件 + `ARCHITECTURE.md`（含 `# 已定型的决定` 与
  `ARCHITECTURE/` 机制文档）+ `docs/`；**不设 `DECISIONS.md`**。**`INSTRUCTIONS.md` 是用户手写、只由用户改**。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**），**R1–R60 已入账**；
  `link` 组 L1–L69；`round-data/R3…R60` 各有一份可跑证据。**2026-11-22 已 reanchor**：锚点 =
  `[tools/svg_grid, monorepo]`（旧 commit 只在 monorepo 里），`check` 0 条、`ids` 不变。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）；R3 起 `--height`/`--rel-*` 不再出现、R4 零告警。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已重做；**Figure 2**（矢量带 + 13 个位图 panel）
  复刻成功——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、四次 `plot-grid` stderr 全 0。
# Completed

（细节在账本与 `ARCHITECTURE.md`，不在此重复。）

- 2026-11-24 **R60 光源进 user space**：`z`/`pointsAtZ` 改用 resvg 的 **RMS** `sqrt((sx²+sy²)/2)`（几何平均错，
  4×5 spot 37011/206→0）、光源 `x/y`/`pointsAtX/Y` 改为平移+缩放（两 cell 5277/238→0）、`primitiveUnits=
  objectBoundingBox` 不再跳过 lighting（100170/242→0，真 obb 分数仍门控）、图元子区域按 units 改写、
  非有限值一律保原文不写 `inf`。拆出 `src/transform/lights.rs`，测试 643→658，提交 `72a05e7`。
- 2026-11-24 **R59 滤镜光源与单轴双值**：`feSpotLight` 的 `x/y/pointsAtX/pointsAtY` 其实是长度（等向
  18003/205→0），及各向异性写**双值**；`--svg-grid` 单轴策略缺因子按 1.0 并给标量写双值（`2`→`"8 2"`）；
  五种畸形长度回 rc=0 + 原样 + W7；obb `clipPath` body 字号不再缩。提交 `081293e`，测试 633→643。
- 2026-11-24 **R57–R58 证据卫生 + 滤镜图元与定义体字号**：R57 把 R19 用例数从下限改回**精确相等**、`temp_path()`
  加纳秒 `process_tag()` 修 pid 复用假红、R55 语料 220→214（`140ed0a`+`a724c08`）；R58 新增 `filters.rs`，
  滤镜图元长度按 `primitiveUnits` 门控（各向异性写**双值** `8 10`⇒0，几何平均 `8.944`⇒69016 弃用）、定义体
  `text`/`tspan` 的 `font-size` 随面板缩、filter/mask 不再凭空补 0 区域。测试 598→633，提交 `c4c522f`。
- 2026-11-24 **R55–R56 paint server 共轭 + 字母判据收口**：R55 新增 `src/transform/transforms.rs`，按
  `T' = S·T·S⁻¹` 共轭 `patternTransform`/userSpace `gradientTransform`（等向保关键字、各向异性纯平移写
  `translate`、带旋转写 `matrix`）；R56 同属性多条 `!important` 取**文档序第一个**（跟 resvg 0.47）、
  `#rrggbbaa`/`rgba()` 零 alpha 不出墨、gradient 沿整条 href 链 + 环守卫、`<symbol>` 的 `display` 不作用于自身
  并按 `viewBox` 裁实例。测试 560→598，提交 `2bff6ce`、`04042cb`。

- 2026-11-24 **R51–R54 字母查重四轮**：`src/label_letters.rs`（`collect_text` 不递归进带 marker 的子元素、
  `expand_use` 按 `<use>` 实例化计并防环、`normalize_label` 只折四个 ASCII 空白并删零宽）；`label_paint.rs`
  （`Paint` 五字段随树继承、`url(#id)` 可解析时按 gradient stop / pattern 子元素判墨）；`style_prop` 剥
  `/* */`、后声明胜、`!important` 压普通（关键字大小写仍敏感）。测试 496→560，提交 `bae128d`、`3bb8516`。
- 2026-11-24 **R49–R53 克隆去重 + 非 XML 1.0 字符 + 诊断偏移**：R49 新增 `src/transform/hoist.rs`（被外壳外以
  不同 policy 引用的嵌套定义先提升）、`split_children` 加 `narrow`；R50 新增 `src/dom.rs::is_xml_char` +
  `decode_reference`，文本/属性/CDATA 共用一份判据且 fail-closed（`&#1;` 修前 rc=0 但产物不良构 → 修后 rc=1）；
  R52 用 `strip_synthetic_range` 剥 quick-xml 合成偏移；R53 `classify` 收集**全部**命中码（W9/W7 不再被掩蔽）。
  测试 474→528。
- 2026-11-24 **R47–R48 字符引用 + 字母查重**：R47 两处 `Event::GeneralRef` 改调 `quick_xml::escape::unescape`
  （`Err` 走各自 Xml 变体 rc=1，两份 `resolve_reference` 删除；`37&#176;C` 修前 ink 1701 → 502/diff 0）；
  R48 递归收 marker 子树文本、跳过不可见子树、`collisions` 改 O(n)、`LetterSite.cell` 变真、≥5 撞号给
  `+N more`（40000 字母 1.31 s→0.06 s）。测试 435→474。提交 `8aca0d1`、`9117335`。
- 2026-11-24 **R45–R46 告警码漂移 + 定义元素缩放**：R45 短语表搬到 `src/warnings/codes.rs`（一码多短语；
  W4a 去反引号、W5 收两形态、W3/W8 移出表），`codes_tests.rs` 用真实 `describe_*` 回灌、W7 门禁复活（415→419）；
  R46 `src/transform/` 拆成
  `defs/links/walk/dispatch`，`defs::split` 建不动点、多 policy **传递地复制**定义（id 后缀 `--svg-grid-<tag>`）、
  marker 有 viewBox ⇒ body verbatim、units 沿 href 继承（mask 不继承）。提交 `196da47`、`54fa2b6`。
- 2026-11-24 **R42–R44 转换器并入主库 + 单位属性 + 字母重号**：R42 `convert/` crate → `src/convert/` 模块
  （28 个 `.rs` `git mv`、删独立 `[[bin]]`，诊断前缀统一 `svg_grid convert:`，语料与 base 逐字节相同，394 单
  crate）；R43 `src/transform/units.rs`
  精确 token 语义（不 trim、非法值回退默认值）+ tile 的 x/y/width/height 逐属性沿 href 解析（394→406）；
  R44 字母唯一性提升为整张图的性质（输入自带 ∪ 本次分配，同字母 ≥2 次且至少一处来自输入 ⇒ rc=2、
  不写产物、`--plan` 同 rc，406→415）。提交 `de1bfae`→`8f38435`、`8846fa8`、`4873156`。
- 2026-11-24 **R37–R41 defs 与 pattern 单位语义**：`prune_defs` 按引用保留 + 悬空引用 fail-closed（R37）、
  pattern 相位（tile 全映射 + body 只缩放不平移，R38）、obb 内容逐字保留（R39）、
  viewBox/严格 token（R40）、href 断链回退 user space + 重复 id 后者胜 + `patternUnits` 继承（R41）。
- 2026-11-23 **D5b–D5d W8 收尾（R34–R36）**：`describe_missing_font_error` 改为**链里任一名字可用（在库或
  通用族）即放行**，整链不可用才 rc=3 并点名整链 + 可用族 + 修法；错误行 200 字符硬上限
  （`MAX_ERROR_BODY`/`assemble_within`），W3 205→188、W8 239→194/195、W5 217→191；D5d 改为**按 `format!`
  真实长度反推预算**（`AVAILABLE_OPEN`/`truncate_body`）。提交 `e362c01`→`b55669e`（R34）、
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
- 2026-10-06 **R26–R13**：参数面收敛 + `--plan`、告警瘦身 ≤198 字符、I/O 报错点名文件、Windows 安装面、
  `install.sh`（含装 skill + `--check`）、原生出图查证、指南 `# CORE WORKFLOW`、`tests/selftest/`、
  `convert`/`doctor` 并为子命令、paint server 丢弃 + W9、rc 分档、字体库 `fontlib/` + W8。
- 2026-10-06 **R12–R11**：补 `round-data/R10/` 证据（R12）；`DECISIONS.md` 并入 `ARCHITECTURE.md`（R11）。
- 2026-09-28 **R10–R5**：指南规范源（R10）；HARNESS 归位（R9）；6 个 >20 KB 源文件拆分（R8）；
  `src/ink.rs`/`src/path.rs` 拆目录（R7）；`<path d>` 宽松化 + W6 微尘（R6）；主库纳入 `<path>`（R5）。
- 2026-09-27 及更早 **R4–R3**：W6 越界 + 死白带判据；按 7 篇反馈更新指南；更早按 10 篇反馈修阻塞点
  （7 修 / 3 判外部或用错）、组图路线反转 `svg_grid` 优先、布局 cowplot 化五项、`convert/` 建 crate。
# In Progress

- **R61（进行中）**：收 R60 评审的 P2 —— **乘积溢出仍被写进产物**（`x="1e38"` 在 s=8 下变 `x="inf"`、
  `width="NaN"`，`stroke-width="3e38"`⇒`inf`，pin 与 base 一致 ⇒ pre-existing；`rewrite_scaled_length` 与
  `primitives` 只判输入不判乘积，`format_number(inf)` 直接给 `inf`）；以及三处文书/证据修正（"45.277 翻 11
  像素"实为 5、`R60/test.py` 的 F5-xy 空转、R58 A9 的 fixture 不可分辨）。
# Known Issues

（只留**未决**项；每条的机制解释见括号里的文件。已解决的旧条目不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的缺口（2026-11-23 实测）

**D1–D6 + D5b–D5d 全部已修（R27–R36）**，`INSTRUCTIONS.md` 的六条要求均已满足。剩下列 pre-existing，均已开报告：

1. **滤镜图元长度的残余**（R58 按 `primitiveUnits` 门控、R59 修光源与单轴）：光源因子与平移是 R60 正在修的；
   各向异性 `feImage`（640/127）、`feDisplacementMap`（127，采样噪声）、定义体 `font-size`（5880/255，单值无法
   精确）与图元子区域 `x/y/width/height`（16984/255）**登记不修**；`feConvolveMatrix`/`feDistantLight` 不动。
2. **乘积溢出仍写进产物**（R61 正在修）：输入有限但乘轴因子后溢出（`x="1e38"` 配 s=8、`stroke-width="3e38"`）
  会写出 `inf`/`NaN` 且无 W7；写回点要统一按 `style.rs::scale_token` 的"判乘积"口径改成保原文 + W7。
3. **内容域第二条轴**（R46 的 policy 轴之外）：verbatim body（pattern 有 viewBox 或 `patternContentUnits=
   objectBoundingBox`、marker 有 viewBox、obb 的 mask/clipPath/filter）的**出边仍把面板 policy 传给内层定义**
   ⇒ viewBox 父 pattern 的内层 pattern 多缩一次（`12160`）、内层 gradient `x2=16→32`（`25280`）、obb 父内
   userSpace gradient `x2=0.5→1`（`27200`）；同一 gradient 被普通 rect 与 viewBox body 共用时需按内容域克隆
   （`15520`）。`patternTransform`/`gradientTransform` 已由 R55 收口；嵌套 `<pattern>` 与 pattern 外
   `clip-path` 实测 `xor 0`。
4. **`tspan x`/`dy` 重排**（R48 评审 P2，暂不做）：文档序 ≠ 视觉序，
   `<tspan x="50">A</tspan><tspan x="10">B</tspan>` 视觉是 BA 而查重按 AB 拼接 ⇒ 既可能误判也可能漏判。
   需要视觉序模型，登记待排期。
5. **R19 证据脚本已 `retire --kind deprecated`**：它写死 190/60/57/73 用例数、7 个历史测试目录与独立二进制
   告警前缀，在 base 上也是红的（8 FAIL）；"旧证据全绿"的口径**不含 R19**。R57 已把用例数改回
   **精确相等**（658 = 366+59+233），但只挡增删、**挡不住同名抽空**（把测试体换成 `{}` 仍全绿）；要挡需按
   测试名做锚，未做。

## 其他未决项

- **`--rel-widths` 部分为 0 会静默产出 0 宽格子**（低优先）：`0,1` 时首列 cell 宽 0、面板不可见，
  与 `rel_*` 其余 fail-closed 不一致。
- **嵌套组图累积偏差**（非变形，每步 `Δ=0.00%`）：A4 目标 1000×1414 拼出 1532.48（+8.4%，GH 行 1.445 倍行高）；
  中间步 margin 改列宽比（默认 24 ⇒ A:BC=1.951）⇒ 嵌套中间步一律 `--plot-margin 0`。
- **无跨格（span/merge）能力**：`src/cli/parse.rs` 的 FLAGS 只有 `nrow/ncol/rel-*/gap`；"A 占 2×2" 只能嵌套近似。
- **W6 在嵌套输出上是假阳性**：越界数字来自残留的 `visibility="hidden"` 内层 panel-box rect（ink walker
  只跳 `display:none`，`src/ink/walk.rs:82,128`）；另 `left` 是 `0.75em/字符` 上界估算（高估约 6.4×）。
- **合成时面板内容不再被面板画布裁剪**：超出面板画布的元素会渲染到成品外边距（密集版式下可能压到相邻
  面板）。要严格复刻需在 cell 上补 clip——**尚未做**（见 `ARCHITECTURE/layout.md` §2）。
- **固定 `--plot-margin` 时"溢出字母带但没出画布"的字母仍被静默裁**（W5 只覆盖"越出画布"）：按需要自行留足。
- **W6 的容差 1 px 保留、不加豁免**（已知会为真实的小越界出声，如 S39-C 修好后仍报
  `right by 1.537 px`，成品 24 px margin 掩盖得住）——**待用户裁决**是否放宽或按侧豁免。
- **`convert/` 对 RDKit `MolDraw2DSVG` 方言的结论需重测**：记录取自 2026-09-20 前（"37 个不支持节点"），
  而 `<path>` 出形态与主库 path 支持均已改 ⇒ 重测后再采信；2D 结构式目前走位图。
- **`<path>` 出形态决策表的已知边界**：判断"是否描边"只看图元自身声明；描边只来自祖先 `<g stroke=…>`
  时，多子轮廓填充路径仍会描出连接边（两个受支持方言实测 0 例，**明确不做**，见 `ARCHITECTURE/convert.md`）。
- **镜像（负）等比 + 旋转弧**会静默失真（手性翻转需翻 `sweep`/取反 `rotation`，而守卫只判
  `|sx| ≠ |sy|`）——**当前 CLI 不可达**，仅登记；将来引入翻转必须补（见 `ARCHITECTURE/tag-protocol.md` §4）。
- **跨行字号不一致**：逐行按各自格宽归一化（保证 `s = 1`）⇒ 不同行纸面字号不同（S22 实测
  5.85 / 3.96 / 9.10 pt）。这是 `s = 1` 的代价，已在 `02a` 写明。
- **无"文字条"能力**（`ggdraw + draw_label`）：主库只搬运 `<text>` ⇒ 标题条需先做成"只有文字的面板"。
  **复刻图总高 5639 vs 原图 5764（2.2%）未 root-cause**：band/margin 模型差异。
- **`--rel-heights` 微调行高会纵向拉伸该行面板**（改变格子宽高比 ⇒ 会告警）：自然纵横比布局的必然结果，
  要既改行高又不变形只能改画布宽或面板纵横比。**垫片 `--input none` 的自然纵横比按 1:1 计**。
- **`AlignMode` 未使用**（R 全程不用 `align`）。**版面可读性天花板**：一行 3 面板 + 固定画布宽 ⇒
  纸面字号 ~3.8 pt（低于 5–6 pt，用户原版亦然）。**matplotlib 之外的方言未覆盖**（仍 fail-closed）。**path 双
  实现防漂移**（R5）：主库 `src/path/` 与 `src/convert/path.rs` 各一份，同容差、注释互指，改一侧必须看另一侧。
- **两副本指南**：规范源 = `tools/svg_grid/docs/general-figure-guide/`（改动只在这里做）；
  `tools/manuscripts_skills/general-figure-guide/` 用 `rsync -a --delete` 覆盖（R10 裁决，账本 `link` L6–L8）。
- **`research/` 两份旧 `DECISIONS.md` 未迁**（`白藜芦醇胰腺癌` 61 624 B、`MASLD_NP` 5 342 B）：
  在各自的**本地仓库**里、属别的任务 ⇒ **待用户决定**是否一并并入其 `ARCHITECTURE.md`。

# Next Steps

1. **新会话从这里开始：先读 `docs/HANDOFF.md`**（§0 有 R27–R59 的提交表）。六缺陷、defs/pattern 语义、
   字母重号与查重（四轮）、doctor 告警码、定义元素缩放、文本字符引用、嵌套克隆 id、非 XML 1.0 字符、文案
   偏移、paint server transform、滤镜图元长度与定义体字号均已收口；**进行中 = R59**（R58 评审残余），
   之后 = R60 内容域第二条轴。最后做交付前全量复核：`cargo test`（633）、`svg_grid doctor` 57/0/1、
   `pitfalls-manager verify` rc=0、两副本指南 `diff -r`。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 →
   删驱动文档 → 若本轮有改名/移动/拆分，**提交后 `link`**（否则悬空自检会报出来）。
   **语义分支先证后写**：改判据前先读 `usvg` 源码 + 用参考渲染器（resvg/cairosvg）实测"哪一边对"，
   再写代码与期望值（R38–R41 连栽三次的教训）。
3. **（用户定）** Known Issues 里待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、两份旧 `DECISIONS.md`、span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
