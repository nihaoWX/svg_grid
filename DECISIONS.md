# DECISIONS

## 2026-09-17 skill 的组图路线改为 svg_grid 优先，cowplot 降为回退（用户要求）

- **决定**：`general-figure-guide`（规范源 `tools/manuscripts_skills/general-figure-guide/`；副本
  `tools/svg_grid/docs/general-figure-guide/`）改为 **svg_grid 优先**：
  `references/02a_svg_grid_assembly.md` = **首选路线**，**全 R、同会话也走**（面板先各自落成 `.svg` 再组）；
  `references/02_cowplot_assembly.md` → **`02b_cowplot_assembly.md`** = **回退路线**。
  回退判据**仅三条**：①`convert` 对面板方言 fail-closed 且无法改用 `svglite`/matplotlib 重出；
  ②`svg_grid`/`svg_grid_convert` 不可用（没编译/环境不支持/离线构建失败）；③面板拿不到独立 `.svg`。
- **原因**：用户明确要"把这个 skill 做成 svg_grid 组图优先的说法，走不通才回退 cowplot"，并选定
  "**全 R 同会话不算走不通**"。反过来 cowplot 的**文件级**读图在本机实测过**静默产出 0.00% 墨量的
  全白图**（仅告警、退出码 0），与本工具 fail-closed 的取向相冲——该代价已写进 `02b` 与 SKILL.md。
- **落地**：`02a` 顶部（首选 + 回退判据）、`02b` 顶部（回退 + §0 何时走这条 + 回退代价）、
  SKILL.md（规则表「代码组图」行、png+svg 硬规则后新增「组图路线」节、文档索引、反面清单各一条）、
  `04_manuscript_integration.md` §1.1。`01/03/05` 不涉及路线，未改。
- **同步约定**：副本 `tools/svg_grid/docs/general-figure-guide/` 必须与规范源**逐字节一致**
  （`svg_grid/README.md` 承诺"仓库自带这套 skill"）；本次以 `rsync -a --delete` 同步并 `diff -r` 校验为空。

## 2026-09-16 字母只加在"拥有面板作为直接输入"的那一层（用户规则）

- **决定**：组合图（行块）本身**不加**字母；字母只加在为该面板作为直接输入的那一次调用上；外层拼装
  **不传 `--labels`**；要跳过某格用空条目（`--labels A,,C`）。
- **原因**：用户规则——中途给组合图加字母会造成**重号与错位**；且 `AUTO` 编号**每次调用都从 A 重新开始**，
  所以逐行组图必须逐行显式给字母。已实现为 `plot-grid` 的 W3 告警，并写入 `ARCHITECTURE.md` 与 skill `02a`。

## 2026-09-16 布局语义改为 cowplot 式（5 项实施完毕）+ 一处破坏性变更

- **决定**（按 `ARCHITECTURE.md` "设计参照"节的偏差清单实施，commit `bfee815`~`919ec42`）：
  1. **`--height` 可省**：省略时画布高 = 各行自然高 + margin + gap；**该值即输出画布**（用户明确要求"输入什么就输出什么"，不做"逻辑尺寸 ≠ 输出尺寸"的分裂）。
  2. **纵横比进布局**：省略 `--rel-widths` → 列宽按各输入 canvas 纵横比分配（零变形）；省略 `--rel-heights` → 行高按各行自然高。
  3. **字母空间自动预留**：用 `--labels` 且未显式给 `--plot-margin` 时自动留字母带；显式给了则尊重并**在会裁时告警**。
  4. **空位/垫片**：`--input none` 占一个空格，`--rel-*` 允许 0。
  5. **`rel_*` fail-closed**：长度/取值非法直接报错（原 `weights()` 静默退等分）。
- **破坏性变更**：`--rel-widths`/`--rel-heights` 的语义改为 **cowplot 式"乘在自然尺寸上的倍数"**。以前把 `--rel-widths` 手填成"各面板纵横比"的调用方，结果会变（变成纵横比的平方关系）。本仓受影响调用点已枚举：仅 `tests/s22_repro/compose_s22.py`（已改为**省略 `rel_*`**，重跑后六格 `sx=sy=1.000000`，比改前更好——E 由 0.9524 变 1）与 `out/skilltest*/assemble.sh`（历史测试产物，已过期）；其余仅文档提及。
- **垫片的自然纵横比按 1:1 计**（R 的 `NULL` 靠 `rel_widths` 定份额；这里没有天然尺寸，取正方），需要"两侧留 8% 白"就显式给 `--rel-widths`。
- **未做**：文字条生成（R 的 `ggdraw + draw_label`）；`AlignMode`（R 脚本全程不用 `align`）。

## 2026-09-16 以 R 端 `plot_grid.R` 为组图逻辑的权威范本

- **决定**：`svg_grid` 的布局语义以用户既有 R 脚本 `research/白藜芦醇胰腺癌/steps/01_bulk筛选_单细胞空转基础分析/results/scripts/plot_grid.R`（cowplot `plot_grid` 编排）为准；"哪里该改"以该逻辑而非现状为准。该逻辑的归纳已写入 `ARCHITECTURE.md` 的"设计参照"节。
- **原因**：用户的原始设计意图就是复刻这套逻辑（"注意是逻辑"），并明确"要是可以复刻出 cowplot 类似的组图逻辑，agent 用起来将会很丝滑，就像我自己在 R 里使用一样，通常十多行代码就可以组好一张图"。
- **已确认的偏离**（改动清单待用户确认后逐条实施）：
  1. **画布宽高前置**：R 里组图期不给尺寸、只在落盘时给一次；svg_grid 的 `--width/--height` 直接决定行高 → 应支持"画布高由面板纵横比 + margin + gap 反解"（`--height auto`）。
  2. **面板纵横比要手喂**：R 的面板是弹性 grob、调用方不必知道纵横比；svg_grid 必须把 `--rel-widths` 填成各 canvas 宽高比否则拉伸 → 应让布局自己读面板纵横比。
  3. **字母空间与 margin 耦合**：R 的字号与 margin 同为 pt、随 dpi 同步缩放；svg_grid 的字号 auto 是画布宽的**比例**、margin 是**常数** → 改 `--width` 就要重算 margin，默认 24 在 W=1200 时直接裁掉字母（需 `margin.top ≥ ≈0.98×字号`）。
  4. **无"空位/垫片"**：R 用 `NULL` 面板当留白；svg_grid 没有占位符且 `--rel-*` 不接受 0。
  5. **`rel_*` 长度不符时静默退等分**（`layout.rs` 的 `weights()`）：与项目 fail-closed 原则不一致，应报错。
  6. 文字条（标题/图注条）R 靠 `ggdraw + draw_label` 生成；svg_grid 不生成文字，需先做成"只有文字的面板"。
- **不做**：`AlignMode`（R 脚本全程不用 `align`，靠嵌套绕开）；`convert` 的 `--target-width/--scale`。

## 2026-09-16 主库只加 `<image>` 坐标适配，翻译逻辑全留在 `convert/`

- **决定**：主库 `transform.rs` 的 `rewrite_primitive` 增加 `"image"`，按 **rect 同款**重写
  `x/y/width/height`；除此之外主库不新增任何方言处理。外源方言（`<g transform>` 烘焙、`<path>`→
  polyline/polygon、`<use>`/`<defs>` 展开、文字 transform 归一化、`<image>` 翻转烘焙）**全部在 `convert/`**。
- **原因**：`<image>` 与 `<rect>` 是同一套坐标语义，属协议内已有能力的最小扩展；而"把任意方言翻译成
  协议图元"是适配层职责，不应渗进核心组图算法。用户明确："主库只添加 `<image>` 适配，主要转换逻辑仍然在 convert/"。

## 2026-09-16 主库 resvg 启用 `raster-images`

- **决定**：`Cargo.toml` 的 `resvg` features 由 `["text","system-fonts"]` 改为加上 `"raster-images"`。
  不重新引入被回退的 `compose` 管线，也不加 `image`/`png` 直接依赖。
- **原因**：`resvg-0.47.0/src/image.rs` 在 `not(feature = "raster-images")` 分支下**直接放弃解码**
  （只打一条 `log::warn!`），导致成品 PNG 里内嵌 `<image>`（matplotlib 的 colorbar 渐变、`imshow` 热图）
  **静默变白**，而同一份 SVG 用 cairosvg 渲染正常。这是被回退的那批提交里 `1c5f4ac` 做过的事——
  说明那批中至少这一条是渲染内嵌栅格**必需**的。实测：同一区域唯一颜色数 6 → 792（cairosvg 817）。

## 2026-09-16 `convert/` 增加 matplotlib 方言翻译

- **决定**：转换器支持两种方言。新增仿射烘焙（`<g transform>` 压进叶子坐标，含 clipPath 一致性）、
  `<use>`/`<defs>` 按引用展开（未引用者删除、clipPath 保留）、`<path d>` → `<polyline>`/`<polygon>`
  （直线段精确、曲线按容差展平）、文字 `translate(x y) rotate(a)` 归一化为 `x/y + rotate(a x y)`、
  `<image>` 的轴对齐翻转烘焙（解码 PNG 翻转重编码；不可烘焙的变换仍 fail-closed）。
- **原因**：用户澄清"阻塞点就是让 convert 把 python 风格的 svg 转成支持的格式再喂给主库"——
  即适配层本就该承担方言翻译，而不是把 matplotlib 支持当作主库的大改造。实测 matplotlib 输出为
  `<path>`292 / `<use>`436 / `<g transform>` 42 / `<image>`6，正是翻译层要吸收的对象。

## 2026-09-16 面板必须按"目标格子尺寸"出图（协议不缩放 font-size 的必然要求）

- **决定**：走 `svg_grid` 组图时，panel 在出图端就按它将要占据的格子尺寸绘制（figsize 与所有以 pt
  为单位的尺寸同乘 `k_p`）；组图端保持"纯坐标搬运"。
- **原因**：协议**永不缩放 `font-size`**（`transform.rs` 只对 `stroke-width` 应用缩放因子），
  所以 组图后文字相对大小 = 设计相对大小 ÷ `s`，`s = 格子/画布`。S22 实测 `s = 4.8`（F 达 7.53）
  时文字相对只剩 13–21%，不可读。此语义已记入 `docs/PITFALLS/svg-composition-font-size-not-scaled.md`，
  并在 `general-figure-guide/references/02a_svg_grid_assembly.md` 中作为规则说明（该 skill 不指向 PITFALLS）。
- **顺带纠正**：不可用"`data-panel-box` rect 尺寸未变"论证 `s = 1`——该 rect 通常无 `data-scale`
  标签、默认策略 `none` 只平移，尺寸本就不变。真实 `s` 需用带标签的坐标反解（实测某次为 0.860/0.803）。

## 2026-09-16 三处缺陷修复并加回归断言

- **决定**：修 ①`POINT_RCPARAMS` 里 7 个"字号倍数"单位的 `legend.*` 项（与 `font.size` 一起乘
  导致图例被多放大 k 倍，实测比值 +91%）；②resvg `raster-images`；③F 热图行序（pcolormesh 默认
  `origin='lower'` vs imshow `'upper'`）。并在 `compose_s22.py` 加三条断言：内嵌 `<image>` 区域唯一
  颜色数 > 100、图例框/绘图区宽度比与参考 panel 相对误差 < 15%、F 的 y 刻度自上而下 == `F0..F6`。
- **原因**：这三处都是**不报错的静默失败**——用户明确不信任静默错误的产物；断言里附带负向测试
  证明其非空转。

## 2026-09-16 回退混合栅格-矢量 composer，`src/` 回到纯 SVG 组图器

- **决定**：把 `tools/svg_grid` 恢复到 `98fd91e`，移除 2026-08-16 ~ 08-17 那 20 个提交引入的
  混合输入 composer（PNG/JPEG 嵌入、去白边、白掩膜、PNG 预览、hero 行等）。
- **原因**：用户判定这批内容已无用处，且不希望后续工作被它干扰；主库只做 SVG 组图。
- **代价与可控性**：20 个提交保留在历史中可随时取回；`plot-grid` 未受影响（逐文件比对，
  `layout/transform/geom/output/raster/tests` 零改动，`main.rs` 的唯一改动即 compose 分支）。
  回退后重跑主库测试 19 项全绿。（其中 `raster-images` 一条已按需重新启用，见上。）

## 2026-09-16 删除 `adapters/`

- **决定**：删除 `adapters/svg_grid_plot_grid.py`。
- **原因**：用户判定该 MCP 风格适配层无用。

## 2026-09-16 外源 SVG 的适配放在 `convert/`，与主库 `src/` 隔离

- **决定**：外源 SVG → 协议格式 的转换器独立为 `tools/svg_grid/convert/` 下的 crate，
  不修改主库 `src/`、不改主库依赖。
- **原因**：主库要保持纯净；适配逻辑属于外围，且其目标来源（R/Python 方言）会变，不应渗进核心组图算法。

## 2026-09-16 转换器采用 fail-closed

- **决定**：转换器遇到组图器无法处理的构造（`<path>`、`<image>`、非 `rotate` 的 transform、嵌套 `<svg>`、
  `<symbol>`、带坐标 `<tspan>`、逗号 `viewBox`、非 `svg` 根、非正画布、`<defs>` 内可渲染图元）必须
  报错退出并列出位置，而不是产出"看起来成功"的错图。
- **原因**：用户明确不信任静默错误的管线产物；实测过 R `cowplot` 文件级组图在本机会
  **静默产出 0.00% 墨量的全白图**（仅告警、退出码 0），这正是要避免的失败模式。

## 2026-09-16 幂等语义：校验先行，完整产物才跳过

- **决定**：任何输入都先跑完整校验；只有"恰好一个 panel box **且**存在 `data-scale` 分组 **且**
  无不兼容构造"才判为已转换。此时退出 0，且**若显式给了 `--output` 就逐字节复制过去**。
  "半成品"不 `--force` 一律报错退出 1。
- **原因**：修复前只要输入含 panel box 就跳过全部校验、且显式 `--output` 也不写文件却退出 0——
  成批跑时下一级会以"文件不存在"莫名失败而退出码报成功，属静默失败。

## 2026-09-16 默认零变形；仍不做网格参数计算器；不一致时改为告警（推翻原"不加告警"）

- **机制背景（未变）**：`geom::scale_panel_to_cell` 用 `sx = 格宽/画布宽`、`sy = 格高/画布高`
  **两轴独立**地把面板盒"填满格子"。因此**格子宽高比 ≠ 面板画布宽高比 ⇒ `sx ≠ sy` ⇒ 面板被非等比拉伸变形**
  （是变形，不是留白）。布局五项改造只改了"格子怎么分配"，未改这条映射。
- **决定（2026-09-16 更新）**：
  1. **默认零变形**：省略 `--rel-*` 时，列宽按各面板 canvas 纵横比分配、行高按自然高 ⇒ 格子宽高比恒等于
     对应画布的宽高比 ⇒ `sx = sy`。原先"调用方必须把 `--rel-widths` 填成纵横比"的前提**已不再需要**。
  2. **不一致时告警**（推翻本条原先的"不加告警"）：显式给出与纵横比不符的 `--rel-*`、或与内容比例不符的
     `--height`/画布时，向 stderr 报出拉伸/缩放百分比（非致命，退出码不变）；同时报"输入被缩放 ≠1"这类
     会改变文字相对比例的静默情形。
  3. **仍不做网格参数计算器**：理由已从"cowplot 式 `rel_*` 够用、调完看图即可"变为
     "布局已按自然尺寸自动算（`--height`/`--rel-*` 均可省），不需要算参数"。
