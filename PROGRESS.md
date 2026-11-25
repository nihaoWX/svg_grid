# Current State

- **2026-11-22 用户新增 `INSTRUCTIONS.md` + `tests/user_tests/2026-11-20/`**（commit `6e582be`）：
  ① `plot-grid` 自动 convert；② convert 不得损坏原图；③ 单次是二维网格、除声明 NULL 外不许空一格；
  ④ `--labels`/`--plot-margin` 在拥有该散图的那层加。**审计结论：六缺陷全部修复（R27–R36）**；随后
  R37–R41 收 `<defs>` mask/marker/filter 与 pattern 单位语义，R42 把转换器并入 `src/convert/`，
  R43–R67 收单位属性、字母重号、告警码、定义元素缩放、字符引用、嵌套克隆 id、非 XML 1.0 字符、文案偏移、
  paint transform 共轭、`!important` 级联与零 alpha、滤镜图元与字号、光源、溢出写回、门禁/证据卫生、
  内容域与 `<ellipse>`、W7 默认可见、`<use>` 引用的定义、非有限值入口与覆盖面、墨迹判据、obb `clipPath`。
  **2026-11-25 用户决定收尾**（复杂域不再扩，代码留作兼容）。余下 pre-existing 见 Known Issues。
- **主库 `src/`**：纯 SVG 组图器。**支持图元**：`circle/ellipse/rect/line/polyline/polygon/path/image/text/tspan`
  + `stroke-width` + `rotate(a,x,y)`（`<path>` R5 起；分界见 `ARCHITECTURE/tag-protocol.md`）。
  布局语义 cowplot 式（`--height` 可省、`--rel-*` 是倍数、省略即按面板纵横比零变形、`--input none` 垫片、
  `rel_*` fail-closed）；`--normalize-input-max-side` 归一化（几何/`font-size`/`stroke-width`/`style` 长度/
  `textLength` 一起缩）。**测试 789 项**（491 lib + 62 main + 236 selftest，R42 起单 crate）全绿，
  `fmt`/`clippy -D warnings` 干净。
- **`doctor` 全量**：**57 pass / 0 fail / 1 skip**（`matplotlib/mathtext` 按 MANIFEST 计 SKIP）。
- **告警 W1–W9**：W1–W6/W9 **非致命**（只走 stderr、**默认静音**，`--verbose` 才展开；退出码与产物字节都不变），
  **W7（值被原文保留）例外：`plot-grid` 路径非 verbose 也打**（R63；`convert` 不缩放，非有限值由
  `--check` 直接 fail-closed，R65）；
  **W3（给已带字母的输入再加非空字母）rc=2**、**W8（缺标签字体族）rc=3**，两者都**不写产物**、不依赖 `--verbose`；
  正常路径 stderr 0 字节；`convert` 子命令同样受静音控制（契约见 `ARCHITECTURE/warnings.md`）。
- **`src/convert/`（R42 起并入主库，子命令 `svg_grid convert`）**：R `svglite`/matplotlib/`pdftocairo -svg`
  三种来源可入流水线；`<path>` 出形态决策表、`clipPath` 矩形化、非等比 `<image>` 烘框、描边显式化、
  `style` 百分比解析、`<defs>` mask/marker/filter 与 pattern 单位语义（R37–R41）。
  **D1 已修**：≥3 子路径的填充 path 改为展平多轮廓 `<path>`（M/L/Z），字形孔不再填死。
- **成品交付前检查**：`scripts/check_deliverable.py`（薄壳 + `deliverable_checks/` 包）两条检查（死白带 /
  外框 2 px 墨迹 = 0），`--selftest` 自证；判据唯一来源见 `ARCHITECTURE/check-deliverable.md`。
- **文档**：`docs/general-figure-guide/` = 本工具自带的组图 skill（首选 `02a` = `svg_grid`，回退 `02b` = cowplot；
  两副本 `diff -r` 为空）。**HARNESS 是两件套**：本文件 + `ARCHITECTURE.md`（含 `# 已定型的决定` 与
  `ARCHITECTURE/` 机制文档）+ `docs/`；**不设 `DECISIONS.md`**。**`INSTRUCTIONS.md` 是用户手写、只由用户改**。
- **账本**：由 `pitfalls-manager` 管理、按项目目录定位（**不写具体位置**），**R1–R67 已入账**；
  `link` 组 L1–L69；`round-data/R3…R67` 各有一份可跑证据。**2026-11-22 已 reanchor**：锚点 =
  `[tools/svg_grid, monorepo]`（旧 commit 只在 monorepo 里），`check` 0 条、`ids` 不变。
- **git**：本工具已独立成仓（`tools/svg_grid/.git`，remote `github.com/nihaoWX/svg_grid`）；
  根仓库 `.gitignore` 排除 `tools/` ⇒ **开发与提交一律用本目录的仓库**。
- **黑盒验证**：四轮（`tests/s22_repro/out/skilltest{,2,3,4}/`）；R3 起 `--height`/`--rel-*` 不再出现。
- **真实产物**：`research/白藜芦醇胰腺癌` 的 12 张成品图已重做；**Figure 2**（矢量带 + 13 个位图 panel）复刻成功
  ——4961×7016 @600 dpi、14 格 `sx=sy=1.00000`、`plot-grid` stderr 全 0。
# Completed

（细节在账本与 `ARCHITECTURE.md`，不在此重复。）

- 2026-11-25 **R66–R66b 墨迹判据 + `--rel-*` 拒非正数**：`ink_bbox` 的 `Ctx` 带 `{paint, visible, opaque}`，
  closed 图元要 `paints()`、`line` 要 `stroke_paints()`、`image` 只看 `visible`，`url(#id)` 复用 `Paint::inherit`
  ⇒ `fill="none"`/`stroke-width="0"`/`visibility="hidden"`/`<g opacity="0">` 的越界元素不再报 W6（d1render
  ink 全 0）；`--rel-*` 的 `<=0` 改 rc=2 并点名值与 flag。R66b 补掉评审抓到的两处**假阴性**（上色层在
  `<tspan>`、pattern 只靠 `href` 拿内容）+ `currentColor` + 亚像素格（<0.5 px 即 rc=2）。测试 724→789，
  提交 `6e34ff3`+`4626f31`+`0fef7fe`+`448f5f3`。
- 2026-11-25 **R67 非有限值覆盖面 + obb `clipPath`**：`finite::CLASSES` 表驱动补 13 类 composer 真会改写的
  属性（`tspan x/y`、gradient 坐标、pattern/mask/filter 区域、marker、`feTile`、userSpace 下 fe* 图元）⇒
  `--check` 与写入模式 rc=1、点名 `attr="1e999"`、不产文件；obb `clipPath` body 整棵 verbatim（分数不缩、
  不补 `x/y`）⇒ d1render xor 16000→0；门禁先剥注释/字面量并拦 `cfg_attr(…ignore)`。测试 747→772，
  提交 `3fb4474`+`41face5`。
- 2026-11-25 **R65 非有限数值入口 fail-closed + 门禁源码级守卫**：`convert --check` 与写入模式判据改成
  "解析失败**或** `!is_finite()`"，覆盖几何属性/`points`/`d`/`stroke-width`/`font-size`/`textLength`/
  `<stop offset>`/fe* 数值/内联 `style`/根尺寸与 viewBox ⇒ 九个非有限用例全 rc=1、点名属性与值、不写产物
  （`--force` 也不截断哨兵）。R19 加 `gate_assertions.py`（每个 `#[test]` 至少一个断言、禁 `#[ignore]`）。
  测试 711→724，提交 `245a620`+`719be40`+`71ae88f`。
- 2026-11-25 **R64 `<use>` 引用的定义跟随引用者域**：`slot_of` 放行定义里带 id 的普通元素、`collect` 把定义
  子树里的 `use` 边登记成引用 ⇒ `<defs><rect id>` 的 body 随引用者 policy 缩（u2/u3/u5/u17 xor 0；u4 双
  policy 各一份、href 重指）；`<symbol viewBox>` 与 viewBox pattern body 里的 `use` 逐字节不变；
  `<foreignObject>`/`<switch>` 改 fail-closed。测试 701→711，提交 `32803b9`+`01847da`+`5ac0555`+`571efec`。
- 2026-11-24 **R62–R63 内容域第二条轴 + `<ellipse>` + W7 默认可见**：body 已被渲染器映射的定义（pattern/marker
  有 viewBox、obb 的 `patternContentUnits`/mask/filter）其出边引用的定义**完全不缩**；user space 与 verbatim
  共用时按域克隆（`--svg-grid-verbatim`、撞名加序号），`href` 按**自己 body 的域**选靶。八组
  12160/32000/31200/800/1376/16000/18720 全 xor 0；补 `<ellipse>`（`cx/cy` 成对、`rx` 按 x、`ry` 按 y、单半径
  补写）。R63 让 `emit` 用 `classify` 逐行取码、只放行含 W7 的行（W7 默认 stderr 263 B、rc/产物字节不变）。
  测试 674→701，提交 `7876c46`+`cc66d4b`+`45dc842`+`b6f6741`+`fea3fc4`+`8d0d15b`。
- 2026-11-24 **R60–R61 光源进 user space + 溢出写回**：`z`/`pointsAtZ` 用 **RMS** `sqrt((sx²+sy²)/2)`（几何平均
  错：4×5 spot 37011/206→0）、光源位置平移+缩放（两 cell 5277/238→0）、obb 不再跳过 lighting；R61 把乘积
  非有限也**逐字节保原文 + W7**（`scaled_finite` 统一 8 处，按原子组保 rect 四边、`cx cy`、`points`、`d`、
  filter 1–2 数）。测试 643→674，提交 `72a05e7`、`9dc3c05`。
- 2026-11-24 **R57–R59 证据卫生 + 滤镜图元/字号/光源双值**：R57 把 R19 用例数改回**精确相等**、`temp_path()`
  加纳秒 `process_tag()` 修 pid 复用假红、R55 语料 220→214（`140ed0a`+`a724c08`）；R58 新增 `filters.rs`，
  图元长度按 `primitiveUnits` 门控（各向异性写**双值** `8 10`⇒0）、定义体 `font-size` 随面板缩、filter/mask
  不再凭空补 0 区域（`c4c522f`）；R59 修 `feSpotLight` 位置与单轴双值、畸形长度回 rc=0+W7（`081293e`）。
  测试 598→643。
- 2026-11-24 **R55–R56 paint server 共轭 + 字母判据收口**：R55 新增 `src/transform/transforms.rs`，按
  `T' = S·T·S⁻¹` 共轭 `patternTransform`/userSpace `gradientTransform`（等向保关键字、各向异性纯平移写
  `translate`、带旋转写 `matrix`）；R56 同属性多条 `!important` 取**文档序第一个**（跟 resvg 0.47）、
  `#rrggbbaa`/`rgba()` 零 alpha 不出墨、gradient 沿整条 href 链 + 环守卫、`<symbol>` 的 `display` 不作用于
  自身并按 `viewBox` 裁实例。测试 560→598，提交 `2bff6ce`、`04042cb`。

- 2026-11-24 **R51–R54 字母查重四轮**：`label_letters.rs`（`collect_text` 不递归进带 marker 的子元素、
  `expand_use` 按 `<use>` 实例化计并防环、`normalize_label` 只折四个 ASCII 空白并删零宽）；`label_paint.rs`
  （`Paint` 五字段随树继承、`url(#id)` 可解析时按 gradient stop / pattern 子元素判墨）；`style_prop` 剥
  `/* */`、后声明胜、`!important` 压普通。测试 496→560，提交 `bae128d`、`3bb8516`。
- 2026-11-24 **R49–R53 克隆去重 + 非 XML 1.0 字符 + 诊断偏移**：R49 新增 `hoist.rs`（被外壳外以不同 policy
  引用的嵌套定义先提升）、`split_children` 加 `narrow`；R50 新增 `is_xml_char` + `decode_reference`，文本/属性/
  CDATA 共用一份判据且 fail-closed（`&#1;` 修前 rc=0 但产物不良构 → 修后 rc=1）；R52 剥 quick-xml 合成偏移；
  R53 `classify` 收集**全部**命中码（W9/W7 不再被掩蔽）。测试 474→528。
- 2026-11-24 **R47–R48 字符引用 + 字母查重**：R47 两处 `Event::GeneralRef` 改调 `quick_xml::escape::unescape`
  （`Err` 走各自 Xml 变体 rc=1；`37&#176;C` 修前 ink 1701 → 502/diff 0）；R48 递归收 marker 子树文本、跳过
  不可见子树、`collisions` 改 O(n)、`LetterSite.cell` 变真、≥5 撞号给 `+N more`（40000 字母 1.31→0.06 s）。
  测试 435→474，提交 `8aca0d1`、`9117335`。
- 2026-11-24 **R45–R46 告警码漂移 + 定义元素缩放**：R45 短语表搬到 `src/warnings/codes.rs`（一码多短语；
  W4a 去反引号、W5 收两形态、W3/W8 移出表），`codes_tests.rs` 用真实 `describe_*` 回灌、W7 门禁复活（415→419）；
  R46 `src/transform/` 拆成 `defs/links/walk/dispatch`，`defs::split` 建不动点、多 policy **传递地复制**定义
  （id 后缀 `--svg-grid-<tag>`）、marker 有 viewBox ⇒ body verbatim、units 沿 href 继承（mask 不继承）。
  提交 `196da47`、`54fa2b6`。
- 2026-11-24 **R42–R44 转换器并入主库 + 单位属性 + 字母重号**：R42 `convert/` crate → `src/convert/` 模块
  （28 个 `.rs` `git mv`、删独立 `[[bin]]`，诊断前缀统一 `svg_grid convert:`，语料与 base 逐字节相同）；
  R43 `src/transform/units.rs` 精确 token 语义（不 trim、非法值回退默认值）+ tile 的 x/y/width/height 逐属性
  沿 href 解析（394→406）；R44 字母唯一性提升为整张图的性质（输入自带 ∪ 本次分配，同字母 ≥2 次且至少一处
  来自输入 ⇒ rc=2、不写产物、`--plan` 同 rc，406→415）。提交 `de1bfae`→`8f38435`、`8846fa8`、`4873156`。
- 2026-11-24 **R37–R41 defs 与 pattern 单位语义**：`prune_defs` 按引用保留 + 悬空引用 fail-closed（R37）、
  pattern 相位（tile 全映射 + body 只缩放不平移，R38）、obb 内容逐字保留（R39）、viewBox/严格 token（R40）、
  href 断链回退 user space + 重复 id 后者胜 + `patternUnits` 继承（R41）。
- 2026-11-23 **D5b–D5d W8 收尾（R34–R36）**：`describe_missing_font_error` 改为**链里任一名字可用（在库或
  通用族）即放行**，整链不可用才 rc=3 并点名整链 + 可用族 + 修法；错误行 200 字符硬上限
  （`MAX_ERROR_BODY`/`assemble_within`）；D5d 改为**按 `format!` 真实长度反推预算**（`AVAILABLE_OPEN`/
  `truncate_body`）。提交 `e362c01`→`b55669e`（R34）、`e4660e8`→`5482421`（R35）、`98a173e`→`d32ab03`（R36）。
- 2026-11-22 **D1–D6 六缺陷（R27–R33）**：字形孔（≥3 子路径的填充 path 改展平多轮廓 `<path>`，R27/R28）、
  末行格子尺寸（行高 = `available_w / Σ全网格列权重`，R29）、默认近方形网格（R30）、网格下界校验 +
  `--verbose` 默认静音（R31）、`plot-grid` 自动 convert（R32）、labels 的 NULL/W3/W8/margin0（R33）。
  机制见 `ARCHITECTURE/{layout,warnings,convert}.md`；数字、变异与复现见账本与 `round-data/R27…R33`。
- 2026-10-06 **R26–R13**：参数面收敛 + `--plan`、告警瘦身 ≤198 字符、I/O 报错点名文件、Windows 安装面、
  `install.sh`（含装 skill + `--check`）、原生出图查证、指南 `# CORE WORKFLOW`、`tests/selftest/`、
  `convert`/`doctor` 并为子命令、paint server 丢弃 + W9、rc 分档、字体库 `fontlib/` + W8。
- 2026-10-06 及更早 **R12–R3**：`DECISIONS.md` 并入 `ARCHITECTURE.md`、指南规范源、HARNESS 归位、
  >20 KB 拆分、`src/ink`/`src/path` 拆目录、`<path d>` 宽松化 + W6 微尘、主库纳入 `<path>`、W6 越界 +
  死白带判据、组图路线反转（`svg_grid` 优先 + `convert/` 建 crate）。
# In Progress

- **无**（R67 + R66b 已收口；2026-11-25 用户决定收尾）。恢复开发时先跑 Next Steps 第 1 条的复核，
  再从 Known Issues 挑（用户点名优先），照第 2 条的链条走（`query` → 修 → `record` → 删报告）。
# Known Issues

（只留**未决**项；机制见括号里的文件。已解决的不再保留——历史看账本与 git。）

## 对照 `INSTRUCTIONS.md` 的缺口

**D1–D6 + D5b–D5d 全部已修（R27–R36）**，六条要求均已满足。剩下列 pre-existing：

1. **滤镜图元与色彩的残余**（R58/R59/R60/R66b）：只剩各向异性 `feImage`（640/127）、
   `feDisplacementMap`（127，采样噪声）、定义体 `font-size`（5880/255，单值无法精确）与
   `stop-color="currentColor"`（无继承 `color` ⇒ d1 0 墨仍报 W6）登记不修。
2. **内容域与图元的残余**：单轴 `data-scale` 配 `rotate`/`stroke` 的 `<ellipse>` 仍错
   （`920/405261`、`881/305344`），但 `<circle>` 同病（`2709/1321221`）⇒ 单轴限制。
3. **`tspan x`/`dy` 重排**（收尾不做）：文档序 ≠ 视觉序，`<tspan x="50">A</tspan><tspan x="10">B</tspan>`
   视觉是 BA 而查重按 AB 拼接 ⇒ 既可能误判也可能漏判；要修需文本排版引擎。
4. **R19 证据脚本已 `retire --kind deprecated`**：写死旧用例数与历史测试目录，在 base 上也是红的；"旧证据
   全绿"的口径**不含 R19**。R57 把用例数改回**精确相等**、R64b 加默认变异（m4）当牙齿、R65 加源码级断言
   守卫（每个 `#[test]` 至少一个断言、禁 `#[ignore]`、点名文件+测试）；仍挡不住"同名抽空"（测试体换成
   `{}`）与"把断言挪进 helper"这类绕过。

## 其他未决项

- **两条检查对负载敏感**：`labels::visibility::markers_in_non_rendered_subtrees_do_not_collide` 在我全量
  `cargo test` 10 次里红过 1 次（`--test-threads=16` 连跑 5 次未复现），`R57/test.py` A 族亦偶发 ⇒ 仍串行跑。
- **嵌套组图累积偏差**（非变形）：A4 目标 1000×1414 拼出 1532.48（+8.4%）；中间步一律 `--plot-margin 0`。
- **无跨格（span/merge）能力**：`nrow/ncol/rel-*/gap` 之外没有 span；"A 占 2×2" 只能嵌套近似。
- **composer 不解析的数值属性**（R65 评审列 30 条，只登记）：`rect rx/ry`（圆角从不缩放）、`text/tspan dx/dy`、
  `stroke-dasharray`/`stroke-dashoffset` 的属性形式、`pattern width`、`textPath startOffset`、`pathLength`、
  多数 fe* 数值属性、`<style>` 里的 CSS 规则 ⇒ 非有限值在这些位置既不拒也不告警。
- **W6 的 `left` 是 `0.75em/字符` 上界估算**（高估约 6.4×）、无 `clip-path` 感知（`skilltest3/Figure.svg` 报
  `right by 24.000 px` 而 d1render 0 墨）、容差 1 px 不加豁免（S39-C 报 1.537 px）——**待用户裁决**。
- **合成时面板内容不再被面板画布裁剪**：超出面板的元素会渲到成品外边距（可能压相邻面板）。
  严格复刻需给 cell 补 clip——**尚未做**（见 `ARCHITECTURE/layout.md` §2）。
- **固定 `--plot-margin` 时"溢出字母带但没出画布"的字母仍被静默裁**（W5 只覆盖"越出画布"）：自行留足。
- **`convert/` 对 RDKit `MolDraw2DSVG` 的结论需重测**（旧记录早于 2026-09-20）；2D 结构式走位图。
- **`<path>` 出形态的已知边界**：判断"是否描边"只看图元自身声明；描边只来自祖先 `<g stroke=…>` 时多子轮廓
  仍会描出连接边（两方言实测 0 例，**明确不做**，见 `ARCHITECTURE/convert.md`）。
- **镜像（负）等比 + 旋转弧**会静默失真（守卫只判 `|sx| ≠ |sy|`）——**CLI 不可达**，仅登记
  （`ARCHITECTURE/tag-protocol.md` §4）。
- **跨行字号不一致**：逐行按各自格宽归一化（保证 `s = 1`）⇒ 不同行纸面字号不同（S22：5.85/3.96/9.10 pt），
  已在 `02a` 写明。
- **无"文字条"能力**（`ggdraw + draw_label`）：标题条需先做成"只有文字的面板"。**复刻图总高 5639 vs 5764
  （2.2%）未 root-cause**。
- **`--rel-heights` 微调行高会纵向拉伸该行面板**（改变格子宽高比 ⇒ 会告警）：自然纵横比布局的必然结果；
  **垫片 `--input none` 的自然纵横比按 1:1 计**。
- **`AlignMode` 未使用**。**版面可读性天花板**：一行 3 面板 + 固定宽 ⇒ 纸面字号 ~3.8 pt。
  **其他方言未覆盖**（fail-closed）。**path 双实现防漂移**（R5）：`src/path/` 与 `src/convert/path.rs`
  各一份，改一侧必须看另一侧。
- **两副本指南**：规范源 = `tools/svg_grid/docs/general-figure-guide/`（改动只在这里做）；
  `tools/manuscripts_skills/general-figure-guide/` 用 `rsync -a --delete` 覆盖（R10 裁决，账本 `link` L6–L8）。
- **`research/` 两份旧 `DECISIONS.md` 未迁**（在各自本地仓库、属别的任务）⇒ **待用户决定**。

# Next Steps

1. **新会话从这里开始：先读 `ARCHITECTURE.md` 与本文件**（历史看 `git log`，细节看账本）。六缺陷、defs/pattern 语义、
   字母查重（四轮）、告警码、定义元素缩放、字符引用、克隆 id、非 XML 1.0 字符、文案偏移、paint transform、
   滤镜图元与字号、光源与溢出、内容域与 `<ellipse>`、墨迹判据、非有限值入口均已收口；**无进行中轮次**。
   交付前全量复核：`cargo test`（789）、`svg_grid doctor` 57/0/1、`verify` rc=0、两副本指南 `diff -r`。
2. **每轮都走同一条链**：动手前 `query <要改的位置> --repo <本工具目录>` 查历史 → 修 → `record` 入账 →
   删驱动文档 → 若本轮有改名/移动/拆分，**提交后 `link`**（否则悬空自检会报出来）。
   **语义分支先证后写**：改判据前先读 `usvg` 源码 + 用参考渲染器（resvg/cairosvg）实测"哪一边对"，
   再写代码与期望值（R38–R41 连栽三次的教训）。
3. **（用户定）** Known Issues 里待裁决：W6 容差与嵌套假阳性、RDKit 方言重测、两份旧 `DECISIONS.md`、span 能力。
4. 需要时 root-cause 图级留白差异（S22 的 2.2%）；需要时补"文字条"能力。
