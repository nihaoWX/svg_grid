# 交接文档 — `svg_grid`（2026-11-22）

> 本文件是 2026-11-22 的审计交接：缺陷已复现并定位到 `file:line`。规则来源是用户手写的
> `INSTRUCTIONS.md`（最高优先级）。**修复进度见下面的 §0**，修复后的语义以 `PROGRESS.md` 与
> `ARCHITECTURE/` 为准；§2 起的缺陷描述保留作复现步骤。

## 0. 修复进度（2026-11-23 更新）

| 缺陷 | 状态 | 提交 / 账本 |
|---|---|---|
| D1 convert 填死字形孔 | ✅ 已修，对抗评审 PASS | `1e05ce1`→`3a9747a`，`1da48f0`→`64e4551`；R27/R28 |
| D2 末行格子尺寸随行拉伸 | ✅ 已修，评审 PASS | `66fba3c`→`cf76029`→`f8be8ef`→`8b08524`；R29 |
| D3 默认网格是单行 | ✅ 已修，评审 PASS | `e09c33b`→`dc9a900`；R30 |
| D6 `rows×cols` 上界 + `--verbose` | ✅ 已修；D6b 关闭性复核通过（R4–R14 绿） | `3311eec`→`676d82d`；R31；连带面 `79cf9e5`/`dc0bda6` |
| D4 `plot-grid` 自动 convert | ✅ 已修 | `162c0cd`→`79cf9e5`→`a2d5fb5`；R32 |
| D5 `--labels` NULL/W3/W8 | ✅ 已修 | `e9425d2`→`398a21e`；R33 |
| D5b W8 把回退链当单族名 | ✅ 已修，评审 PASS | `e362c01`→`b55669e`；R34 |
| D5c 病态输入的 W8 行超 200 字符 | ✅ 已修 | `e4660e8`→`5482421`；R35 |
| D5d 预算手算少 1（丢 `available:`、回显塌 `"…"`） | ✅ 已修，对抗评审 PASS | `98a173e`→`d32ab03`；R36 |
| R37 `<defs>` 内 mask/marker/filter 被丢弃 | ✅ 已修，对抗评审 PASS | `18102d8`→`756dfb2`；R37 |
| R38 `<defs>` 嵌套定义被误丢 + pattern 相位（tile 与 body 双平移） | ✅ 已修（含 R39/R40/R41 收尾），对抗评审 PASS | `95698ca`→`75d59fb`；R38/R39/R40/R41 |
| R39/R40/R41 pattern 单位语义（obb 双重缩放、viewBox、href 继承、重复 id） | ✅ 已修，R41 评审 PASS | `2a6a1ed`→`96dfeab`、`475894a`→`715ce48`、`0d3d758`→`3a8416e` |
| R42 `convert/` crate 并入 `src/convert/` 模块（用户要求） | ✅ 已搬迁，对抗评审 PASS | `de1bfae`→`8f38435`；R42 |
| R43 单位属性语义（非法值/继承、marker/mask 内容域、IRI） | ✅ 已修，对抗评审 PASS（1 项 P1 遗留） | `8846fa8`→`2287213`；R43 |
| R44 组合图字母重号静默（rc=0、A,B,A） | ✅ 已修，rc=2 fail-closed | `4873156`→`42184d5`；R44 |
| R45 doctor 告警码漂移（11 条里 5 条死码，W7 门禁 fail-open） | ✅ 已修（短语表搬到 `src/warnings/codes.rs`，W7 门禁复活） | `196da47`；R45 |
| R46 `<defs>` 子树不继承引用者 `data-scale` + marker/viewBox + units 继承 + normalize 盲缩放 | ✅ 已修（按引用图传递地复制定义；mask 明确不继承） | `54fa2b6`；R46 |
| R47 数字实体 / 未定义命名实体的文本路径 | ✅ 已修（`quick_xml::escape::unescape`，未定义实体 rc=1） | `8aca0d1`；R47 |
| 字母重号检测漏判（tspan/g 包裹、实体、空白）与反向误伤（不可见字母） | ✅ 已修（递归收集 + 规范化 + 跳过不可见子树 + O(n)）；对抗评审 FAIL 的 1 阻断（嵌套 marker 重复计数）待修 | `8aca0d1`→`9117335`；R48 |
| R49 嵌套定义被克隆时产生重复 id + normalize 盲缩定义元素自身表现属性 | ✅ 已修（`hoist.rs` 提升 + `narrow` 收窄 + `content_is_user_space` 门控） | `4e9f8c6`；R49 |
| R50 产物不良构：非 XML 1.0 字符被解码成裸控制字符（`&#1;`/`&#xFFFE;`，两条路径） | ✅ 已修（`is_xml_char` 一份判据两条路径 fail-closed + 产物良构不变量测试） | `1fbc3c3`；R50 |
| R51 字母查重剩余缺口（嵌套 marker 重复计数、`<defs>`+`<use>`、NBSP、关键字大小写、`fill="none"`） | ✅ 已修（收集器抽成 `label_letters.rs`；关键字大小写按渲染器保持敏感，只剥 `!important`） | `bae128d`；R51 |
| R52 R50 评审的账本/文案收尾（R47 行号被误改、R50 meta 语料数、假 range `at 1..5`、`lib.rs` 体积） | ✅ 已修（`strip_synthetic_range` 唯一实现；append-only 更正 R47 真实行号；`normalize_label` 搬进 `label_letters.rs`） | `6d32738`；R52 |
| R53 `classify` 取表序第一个命中码（W9/W7 门禁可被用户内容打成 PASS）+ 短语副本 + `R17` 死变量 | ✅ 已修（`classify` 收集全部命中码；短语单一来源；补记 R45 行号漂移） | `a0025cf`；R53 |
| R54 字母判据的误伤面（`url(#missing)`、`fill-opacity=0`、祖先 opacity/stroke-width 继承、style 级联、`<symbol>` 经 `<use>`、嵌套文本切口） | ✅ 已修（新增 `label_paint.rs` 继承五字段 + url 可解析性判墨；`style_prop` 剥注释/后声明胜/`!important` 优先） | `3bb8516`；R54 |
| R55 `patternTransform` / `gradientTransform` 里的长度量不跟随面板缩放 | ✅ 已修（`transforms.rs` 按 `T' = S·T·S⁻¹` 共轭；各向异性非正交格点残余已登记） | `2bff6ce`；R55 |
| R56 字母判据的 `!important` 级联、零 alpha 颜色、`<symbol>` 实例化 | ✅ 已修（同属性多条 `!important` 取文档序第一个；`#rrggbbaa`/`rgba()` alpha=0 不出墨；`label_clip.rs` 按 `viewBox` 裁实例） | `04042cb`；R56 |
| R57 R56 入账 + R19 门禁补回真牙 + `temp_path()` 并发唯一性 | ✅ 已修（R19 用例数改回精确相等，`gate_demo.py` 证明只有精确快照绿；`process_tag()` 加纳秒修 pid 复用） | `a724c08`；R57 |
| R58 滤镜图元长度（门控 `primitiveUnits`）+ 定义体 `font-size` + filter 区域被补 0 | ✅ 已修（新增 `filters.rs`；各向异性写双值 `8 10`，几何平均实测更差；`rewrite_region` 不再凭空写 0） | `c4c522f`；R58 |
| R59 R58 评审残余：`feSpotLight` 长度被当角度跳过、单轴策略下双值标量没缩、非法长度变 rc=2、obb clipPath 字号 | ✅ 已修（等向 18003/205→0；单轴写双值 `8 2`/`2 10`；畸形值保原文 + W7） | `081293e`；R59 |
| R60 R59 评审 P1：光源 `z`/`pointsAtZ` 该用 RMS 而非几何平均、光源 `x/y` 只乘不平移、obb 下光源改写被跳过 | ✅ 已修（spot 4×5 37011/206→0、point 95008/2→0、obb 100170/242→0、两 cell 5277/238→0） | `72a05e7`；R60 |
| R61 R60 评审 P2：输入有限但乘积溢出仍写 `inf`/`NaN`（`x="1e38"` 配 s=8 ⇒ `x="inf" width="NaN"`） | ✅ 已修（`scaled_finite` 判乘积；溢出一律逐字节保原文 + W7；原子组保 rect 四边/cx cy/points/d/filter 数） | `9dc3c05`；R61 |
| R62 R61 评审：内容域第二条轴（verbatim body 出边不传 policy、多域按域克隆）+ `<ellipse>` 从未被缩放 + 账本补 modify | ✅ 已修（`Slot::{UserSpace,Verbatim}`、按域克隆 `--svg-grid-verbatim`、href 按自身 body 域选靶；`rewrite_ellipse`；评审 7876c46/cc66d4b FAIL → 终版 PASS 有条件） | `b6f6741`；R62 |
| R63 R62 评审：W7 默认不可见（值被保原文时 stderr 0 字节、rc=0 ⇒ 静默错图）+ 测试硬化（椭圆 `ry` 变异存活、三层 gradient-href 断言被削） | ✅ 已修（`emit` 按 `classify` 逐行取码，只放行含 W7 的行；其余码与 `note:` 仍静音、footer 文案不变；214+58 成功运行仍 0 字节。评审 PASS 有条件：仅 `plot-grid` 路径；`convert --check` 对非有限值仍判 compatible ⇒ Known Issues） | `fea3fc4`；R63 |
| R64 `<use>` 引用的 `<defs>` 普通元素不被缩放（`<defs><rect width="10">` 经 `<use>` 仍 10 ⇒ xor 300/255）+ `<foreignObject>`/`<switch>` 被 `convert` 静默放行 | ✅ 已修（`slot_of` 放行定义里带 id 的普通元素、`collect` 登记定义子树里的 `use` 边 ⇒ u2/u3/u5/u17 xor 0、ink 相等，u4 双 policy 各一份且 href 重指；`symbol viewBox` 与 viewBox pattern body 里的 `use` 仍逐字节不变；`foreignObject`/`switch` 改 fail-closed。评审 PASS 有条件：P1-1 测试强度由 R64b 收口） | `32803b9`+`01847da`+`5ac0555`+`571efec`；R64/R64b |
| R65 `convert --check` 对非有限数值仍判 compatible（`1e999`/`NaN` 能解析成 `inf`/`NaN` ⇒ `is_err()` 判不出）+ R19 门禁挡不住“同名抽空”（把测试体换成 `{}` 仍全绿） | ✅ 已修（判据改“解析失败**或** `!is_finite()`”，覆盖几何属性/`points`/`d`/`stroke-width`/`font-size`/`textLength`/`<stop offset>`/fe* 数值/内联 `style`/根尺寸与 viewBox ⇒ 九个非有限用例全 rc=1、点名属性与值、不写产物，合法值仍 rc=0 且产物逐字节不变；R19 加 `gate_assertions.py`：每个 `#[test]` 至少一个断言、禁 `#[ignore]`、点名文件+测试） | `245a620`+`719be40`+`71ae88f`；R65 |
| R66 W6/W5 对“画不出墨”的内容误报（`fill="none"`/`stroke-width="0"`/`visibility="hidden"`/`<g opacity="0">` 的越界元素仍算进 ink 盒）+ `--rel-widths 0` 静默产出 0 宽格子 | ✅ 已修（`ink_bbox` 的 `Ctx` 带 `{paint, visible, opaque}`，closed 图元要 `paints()`、`line` 要 `stroke_paints()`、`image` 只看 `visible`，`url(#id)` 复用 `Paint::inherit`；`--rel-*` 的 `<=0` 改 rc=2 并点名值与 flag） | `6e34ff3`+`4626f31`；R66 |
| R67 `convert --check` 的非有限值判据漏 13 类 composer 真会改写的属性 + obb `clipPath` body 分数被当 user 单位缩放 + `gate_assertions.py` 不剥字符串/注释 | ⏳ 进行中 | R67 |

## 1. 先读什么（按顺序）

1. `INSTRUCTIONS.md` —— 用户手写、只由用户改。**产品定义与所有语义以它为准**。
2. `PROGRESS.md` → `# Known Issues` —— 每条带 `file:line` 与实测数字。
3. `ARCHITECTURE.md` → 用户期望 **#9–#13**（本次新加的四条）。
4. `ARCHITECTURE/convert.md` → §3 末尾的 **⚠️**（bridge 净绕数为 0 只对 N=2 成立）。
5. `ARCHITECTURE/layout.md` → §2 末尾两条 ⚠️（末行格子尺寸、`rows×cols` 校验）。
6. `ARCHITECTURE/cli.md` → `--verbose` 契约。

## 2. 用户最终定下的规则（实现依据）

| 主题 | 规则 | 出处 |
|---|---|---|
| 单次调用 | 一个**二维网格**放图；**不支持跨格（span）** | `INSTRUCTIONS.md` §网格组图 |
| 填满 | 除**声明过的 `NULL`** 外不许空出一格；显式 `ncol/nrow` 时空位必须写 `--input NULL` | 同上「重在声明」 |
| 默认网格 | 不给 `ncol/nrow`：`cols=ceil(sqrt(n))`、`rows=ceil(n/cols)`，**末格自动当 NULL**。15→4×4，5→2×3 | 同上 |
| `rows×cols` | 必须装得下；且**不许有整行全空**（显式 `NULL` 不算空）。例：5/6 个输入给 3×3=9 格 ⇒ 报错 | 同上 `--ncol/nrow` |
| 格子尺寸 | **只由网格决定**：行高按该行占满 `cols` 列算，**不随该行实际几个图变** | cowplot 1.2.0 实测 + 用户"尽量填满" |
| `--labels` | 作用于该次调用的直接输入；按调用层可调；裸 `--labels` 的 AUTO **跳过 NULL 槽**；显式列表**按槽位**（含 NULL），`""` 跳过 | 同上 `--labels` |
| `--plot-margin` | **默认 24**；每层都加、**允许累积**；只在设置它的那层可调 | 同上 |
| `--verbose` | 默认只输出"必要信息"= **报错**（`ncol/nrow` 不合法、重复给 labels、缺字体）；其余 W1–W9 静音 | 同上 `--verbose` |
| 占比 | = 该散图在最终图里**占几格**，**不是** `--rel-*` | 同上 流程第 2 步 |
| `convert` | **必须自动**，且**不许损坏原图**（字形孔必须保真） | 同上 |

两个 5 图经典例子（用户后续都按这两个指代）：

- **例子 1**：`ABC` 行（3 图）+ `DE` 行（2 图）上下叠，**共 5 格无 NULL** ⇒ 只能靠**嵌套最小步骤**做出来。
- **例子 2**：`ABC` 行 + `DE+NULL` 行，**共 6 格有 NULL** ⇒ 单次 2×3 即可，也正是默认行为。

## 3. 已确认的缺陷（按建议修复顺序）

### D1 `convert` 把 N≥3 子路径的字形孔填死（产物错，最优先）

- 位置：`src/convert/translate/shapes/path_form.rs:52`（纯填充分支）、`:83`（填充+描边拆分分支），
  两者都调 `join_closed_points`（同文件 `:155-166`）。
- 根因：N 条子路径首尾相接后，`<polygon>` 的**隐式闭合**又从末点连回整串首点 ⇒ N 条 bridge 成**闭环**。
  **N=2** 时两条 bridge 是同线段反向、面积 0；**N≥3** 时闭环是真多边形，圈内 winding ±1 ⇒ 内孔被填死。
  现有测试 `tests.rs:29`/`:139` 恰好只覆盖 N=2，所以全绿。
- 实测（`research/白藜芦醇胰腺癌/.../pandamap_all/pandamap_ALB_A.svg`）：
  - 14 个 `<text>` **逐字节保留**（用户以为"文字变 polygon"是误判：残基标签在原图里本来就是字形 `<path>`）；
  - 46 条 ≥3 子路径的 path **全部命中**，24/94 个字孔翻转；"ASP 451" 的 bridge 环面积 192.968 单位²；
  - 隔离渲染黑墨 18,312 → 29,016 px（**1.58×**）。曲线展平**不是**原因（最大偏差 0.029 px @2×）。
- 最小复现：`/tmp/render/donuts2.svg`（2 个甜甜圈 = 4 条子路径）；或用
  `<path d="M0 0 L40 0 L40 20 L0 20 z M10 5 L10 15 L20 15 L20 5 z M26 5 L36 5 L36 15 L26 15 z" .../>`。
- 修法：`subpaths.len() >= 3` 的填充路径**不要合并成单 polygon**，直接输出展平后的 `<path>`（`M/L/Z`，
  resvg 完整支持、语义无损）。同时改掉 `translate/shapes/mod.rs:33-37` 的错误论证，并补 N≥3 回归用例。

### D2 末行格子尺寸随"该行实际有几个图"变（E 被拉长的根因）

- 位置：`src/layout.rs` 的 `row_aspect_sums`（`:197-208`，`index < count` 在 `:203`）与
  `natural = available_w / sum`（`:122-130`）。
- 现象：5 输入 `--ncol 2` ⇒ 末行只有 1 个面板 ⇒ 该格 **476×972.614**、`Δ=−50%`；其余 4 格 476×486.3。
  根因一句话：**行高按"本行几个图"算、列宽按"全网格几列"算**，两套基准。
- cowplot 1.2.0 实测（`Rscript --vanilla`）：5 图 `ncol=2` 的 5 个格子全 `1/2 × 1/3`，**同格、不拉伸**；
  5 图默认 3 列 ⇒ 3+2、左对齐、第 6 格空。
- 修法：**格子尺寸只由网格决定** —— 行高 `= available_w / Σ(column weights)`（`--rel-heights` 仍作倍数），
  末行少几个图不影响格子大小。**不要**改成"末行跨满宽"（那是早期被旧措辞带偏的错误方向）。

### D3 默认网格是"一行"，与 cowplot 相反

- 位置：`src/layout.rs:69-74`，`(None, None) => (1, spec.count)`。
- 现象：5 图不给 `ncol/nrow` ⇒ `1×5`（`--plan` 实测 canvas 1000×242.5）；用户要 `2×3`（末格 NULL）。
- 修法：`cols = ceil(sqrt(n))`、`rows = ceil(n/cols)`；末格自动当 NULL。

### D4（修前）`plot-grid` 不会自动 convert

- 位置：`src/geom.rs:21`（`find_panel_box` 的报错）；唯一调用点 `src/plan.rs:144`
  （`plot_grid` 与 `plan_grid` 共用）。
- 现象：未打标签输入 ⇒ `rc=2`，`input SVG must contain a <rect data-panel-box="main" .../> element`。
- 现成钩子：`svg_grid::convert::convert(&str, bool)`（`src/convert/pipeline.rs:22`，纯字符串、无 I/O；
  R42 起转换器已是同库模块，**无需新依赖**）。`doctor` 的进程内调用可作范式：
  `src/cli/doctor/run.rs:205` → `convert_in_process`（`:467-484`）→ `run_in`。
- 注意：已组好的组合图有多个 panel-box，`convert` 会报 `Incomplete`——但这类输入本来就有 panel box，
  不会进自动分支。

### D5（修前）`--labels` 的 NULL 规则与"必要信息"报错

- 现状：`src/labels.rs:16-18` 按**输入下标**取字母；空位（`None`）在 `lib.rs` 的循环里 `continue`，
  所以 NULL 槽永远拿不到字母。
- 需要：裸 `--labels` 的 AUTO **跳过 NULL**；显式列表**按槽位**（含 NULL）——给 NULL 槽写字母就画、`""` 跳过。
- W3（给已带字母的输入再赋非空字母）现在只是告警（`src/lib.rs:367-379`、`src/warnings/labels.rs:18-36`），
  必须改成**报错**（属于 `--verbose` 规则里的"必要信息"）。
- W8（缺字体族、静默回退）也要改成**报错**。
- 已验证：`--labels ,C` 对已带字母的输入**不触发 W3**，输出 `A、B（继承）+ C`——这就是用户 `plot_grid.R` 的写法。

### D6（修前）`--verbose` 未实现；`rows×cols` 上界没校验

- `--verbose`：现在 W1–W9 与聚合 `note:` 一律打 stderr（`src/warnings/mod.rs` 的 `emit`）。
  需要默认静音、`--verbose` 才展开；`--plan` 的 stdout `note:` 行不受影响（它是计划契约）。
- 网格校验：现在只查 `rows*cols < count`（`src/layout.rs:75-77`）；需补"不许有整行全空"。
  注意这个规则等价于 `rows == ceil(count/cols)`（装得下 + 无空行 ⇒ 行数必为最小）。

## 4. 环境与验证入口

- 二进制：`~/.local/bin/svg_grid`（本次核实与 `target/release/svg_grid` 同 sha256）。
  **改完代码必须重跑 `./install.sh`**（见 `ARCHITECTURE/install.md`）。
- 回归：`cargo test`（**单 crate 747**：452 lib + 62 bin + 233 selftest；R42 起转换器是同库模块），
  `svg_grid doctor`（**57 pass / 0 fail / 1 skip**；`matplotlib/mathtext` 是登记的 SKIP）。
  语料/引用面的总闸：`pitfalls-manager verify`（黑盒跑全部 `round-data/*/test.py`）。
- R：`~/micromamba/envs/r453/bin/Rscript --vanilla`——**必须加 `--vanilla`**，否则启动时 BiocManager
  联网校验会卡 60 s 然后失败。cowplot 1.2.0 已装。
- 本次会话留下的临时材料（重启后可能没了，需要就重建）：`/tmp/sgtest/`（ncol=2 复现、嵌套）、
  `/tmp/render/`（字形孔对比渲染、donuts2）、`/tmp/svg_grid_audit_report.md`、`/tmp/svgaudit/REPORT.md`。

## 5. 账本（`pitfalls-manager`）——已迁移，别再当坑

**2026-11-22 已完成 reanchor**（用户指定锚点改为工具自己的仓）：

- `meta.repo_paths` = **`["/home/nihao/bioagentforge/tools/svg_grid", "/home/nihao/bioagentforge"]`**。
  第一个是工具仓（新记录用），第二个是旧 monorepo（**必须保留**：R1–R26 的 `code_before/after`
  与报告 commit 只在 monorepo 里，去掉会让 `record` 直接失败）。
- `pitfalls-manager check --repo .../tools/svg_grid`：迁移前 **46 条** → 迁移后 **0 条**（0 字节）。
- `query src/layout.rs` 仍命中 `R14-F1`；`ids` 迁移前后逐字一致；对 46 条 distinct `where` 全扫，
  **`⚠ suspect anchor` = 0**；新轮 `record` 在临时库里 `rc=0`。
- **`where` 故意没重写**（仍是 `tools/svg_grid/...`）：那是旧快照里的真实坐标，也是 `git show` 的取回坐标；
  改成 `src/...` 会让 `validate_ledger` 整本失败并毁掉旧证据取回。并轨靠新增的 **`link` 组 L11–L55**
  （`tools/svg_grid/<p>` ↔ `<p>`）与 1 条 retired（L56，R11-F2 的跨项目路径）。
- **报告路径未改**（`tools/svg_grid/docs/PITFALLS/...`），旧证据仍可 `git -C /home/nihao/bioagentforge show <sha>:<path>`。
- 工具侧新增受支持的 **`reanchor`** 子命令（`tools/pitfalls-manager` commit `bcb19c0`，已装 `~/.local/bin`）。
- 回滚备份：`/home/nihao/pitfalls-manager-backups/svg_grid-reanchor-20261122-221338/`（含迁移前 `history.db`、
  前后 `check/query/ids`、REPORT.md）。
- **新轮次**：`where` 用工具仓相对路径（`src/layout.rs`、`src/convert/...`）；查询旧路径靠后缀匹配 + link。
- `docs/PITFALLS/` 目前**没有待处理报告**（只剩 `round-data/R3…R26` 的可跑证据）。

## 6. 本次会话的提交（都在本目录的仓）

```
9335155 HARNESS: 第五次对齐（ncol/nrow 校验、--verbose、margin 24、修正末行修法）
8f1571b HARNESS: 第四次对齐（两个 5 图经典例子 / NULL 字母规则 / margin 默认 24）
2bdcdc2 HARNESS: 第三次对齐（默认网格 / NULL 声明 / margin 分层）
2124c70 HARNESS: 第二次对齐（二维网格语义、NULL 声明、labels 分层）
5b06f14 HARNESS: 首轮验收审计（四条要求全部未满足 + file:line）
```

> `INSTRUCTIONS.md` 是用户的文件；这次每轮对齐都把它一起提交了。用户随后又改过一版，本交接文档
> 与它同一提交入库。

## 7. 建议的下一步

1. ~~先解账本锚点~~ —— 已于 2026-11-22 完成（`reanchor`，`check` 0 条）。
2. ~~按 D1 → D2 → D3 → D6 → D4 → D5 动手~~ —— **六项全部完成（R27–R36）**，见 §0。
   每轮仍走 `query <位置> --repo .` → 改 → `cargo test` → `doctor` → `install.sh` → commit → `record`；
   若与旧轮同址（复发），在 `docs/PITFALLS/round-data/<新轮>/` 留可跑证据。
3. ~~消费 `blocker-convert-defs-mask-dropped.md`~~ —— 已完成（R37），后续 R38–R41 收掉了嵌套 `<defs>`、
   pattern 相位与 pattern 单位语义；R42 把转换器并入 `src/convert/`；R43 收掉单位属性语义；
   R44 收掉组合图字母重号；R45 收掉 doctor 告警码漂移；R46 收掉定义元素的缩放单位；R47 收掉文本字符引用；
   R48 收掉字母查重的漏判与误伤；R49 收掉嵌套定义克隆的重复 id；R50 收掉非 XML 1.0 字符被解码进产物；
   R51 收掉字母查重的剩余缺口；R52 收掉诊断文案的合成偏移与账本行号；R53 收掉告警码遮蔽；
   R54 收掉字母判据的误伤面；R55 收掉 paint server 的 transform 长度量；R56 收掉 `!important` 级联与零 alpha；R57 把用例数门禁改回精确相等并修掉 `temp_path()` 的并发假红；R58 收掉滤镜图元长度、定义体 `font-size` 与 filter 区域补 0；R59 收掉光源长度、单轴双值与畸形长度口径；R60 把光源整体进 user space（RMS z、位置平移、obb 不再跳过）；R61 收掉乘积溢出的 `inf`/`NaN` 写回（保原文 + W7）；R62 收掉内容域第二条轴（含 href 选靶）与 `<ellipse>` 未缩放；R63 让 W7（值被保原文）默认出声；R64 收掉 `<use>` 引用的定义不缩放、`foreignObject`/`<switch>` 改 fail-closed（R64b 补强测试与证据）；R65 让 `convert --check` 对非有限数值 fail-closed 并给 R19 加源码级断言守卫；R66 让 W6/W5 只数真正上色的内容、`--rel-*` 拒非正数。
   **进行中 = R67**（R65 评审收尾：非有限值覆盖面 + obb clipPath 几何 + 门禁守卫）。
4. 交付前全量复核：`cargo test`（单 crate **747**）、`doctor` 57/0/1、`pitfalls-manager verify`（rc=0）、
   `round-data` 顶层全绿、两副本指南 `diff -r`、`pitfalls-manager check`。

## 8. 未决 / 需要用户点头的点

- ~~§5 的账本锚点~~ —— 已解决。
- ~~显式 `NULL` 是否允许出现在**列表中间**~~ —— 用户裁决：**允许**（已实现，`--input NULL` 可在任意位置）。
- ~~W8 改成报错后，只出 PNG/AVIF 时的缺字体是否也报错~~ —— 用户裁决：**所有输出格式都报错**（rc=3，已实现）。
- 仍待用户裁决：W6 容差 1 px 与嵌套假阳性、RDKit 方言重测、`research/` 两份旧 `DECISIONS.md` 是否迁、
  是否引入 span 能力（见 `PROGRESS.md` §Known Issues）。
- `--verbose` 是否也覆盖 `convert` 子命令的告警（建议覆盖，保持一致）。
