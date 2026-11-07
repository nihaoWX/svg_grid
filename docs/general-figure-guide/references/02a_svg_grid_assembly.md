# 组图首选路线：`svg_grid`（文件级矢量组图）

> **首选路线**——**所有组图默认走这里**，包括全 R、同会话的情形（面板先各自落成 `.svg`，再交给 `svg_grid` 组）。
> **panel 从哪来见 `00_native_plots.md`**（原生出图优先）；本文件只负责把已有 `.svg` 拼成成品。
> 只有本路线**走不通**时才回退 `02b_cowplot_assembly.md`，判据见下节。
> 工具侧的协议、布局语义与设计依据见 `tools/svg_grid/ARCHITECTURE/tag-protocol.md` 与 `…/layout.md`。

## 为什么默认走这条

面板**跨语言/跨步骤**（R svglite、matplotlib、渲染器位图）是常态，文件级是唯一通用解；成品**保留矢量**
（`.svg` 是主素材，`.png` 只是预览与嵌入）；具备 cowplot 式的"谁跟谁一组 + 各自占比"但不绑定语言。


## 什么时候才回退 `02b`（cowplot）

**判据只有三条**，任一成立才回退：① 转换器对面板 **fail-closed**，且无法改用受支持方言（`svglite` /
matplotlib / RDKit 矢量后端）重出；② `svg_grid`（含 `convert` 子命令）**不可用**（没编译、环境不支持、
离线构建失败）；③ 面板**拿不到独立 `.svg`**（只能以会话内对象组合）。

**完整清单**（仍 fail-closed 的构造）与**回退的代价**见 `02b_cowplot_assembly.md` §0——本指南不复述。

## 流水线

```
① 出图：每个 panel 单独出，png + svg 成对同名
     R         : ggsave(..., device = svglite::svglite)
     matplotlib: rcParams["svg.fonttype"] = "none"     # 保留 <text>，不要转轮廓
                 savefig(name.svg, format="svg")
② 转换：svg_grid convert --input panels/A.svg --output tagged/A.svg
     补 <rect data-panel-box="main"> + data-scale 分组；把外源方言翻译成协议图元；
     并给描边图元补上显式 stroke-width
③ 计划：同一套输入/布局参数加 --plan，读它给的「每格多大 / s / 要不要先归一化」
     svg_grid plot-grid --input tagged/A.svg --input tagged/B.svg \
         --plan --width 4961 --ncol 2 --labels
④ 组图：一行一次调用，只给"内容"与列数（需要时先把面板缩到 --plan 给的格宽）
     svg_grid plot-grid --input tagged/A.svg --input tagged/B.svg \
         --output row1.svg --width 4961 --ncol 2 --labels
⑤ 纵向拼：把各"行块"当外层输入
     svg_grid plot-grid --input row1.svg --input row2.svg --input row3.svg \
         --output Figure.svg --output-png Figure.png --png-dpi 600 --ncol 1 --width 4961
```

**组图期不需要给 `--height`，也不需要给 `--rel-*`**：

- 省略 `--height` → 画布高由"各行自然高 + margin + gap"推出，**该值就是输出画布的宽高**（`viewBox` 与 PNG 像素，输入什么就输出什么）；
- 省略 `--rel-widths` → 列宽按**各面板自己的 canvas 纵横比**分配 ⇒ 每个格子的宽高比 = 该面板的宽高比 ⇒ **零变形**；
- 省略 `--rel-heights` → 各行按自己的自然高。

`--rel-*` 只在想**偏离自然尺寸**时给，语义是"**乘在自然尺寸上的倍数**"（`1,1,2` ≡ `3.2,3.2,6.4`，只算比值）。

## 硬规则

| 项 | 规则 |
|---|---|
| 面板盒 | 每个输入必须有 `<rect data-panel-box="main">`；由转换器补，不要手写 |
| 尺寸 | `--width`/`--height` 就是输出画布（SVG 与 PNG 同一尺寸）。**只给 `--width` 即可**；显式给 `--height` 时按"填满画布"布局（需自行保证与内容比例一致） |
| 格子比例与变形 | `--rel-*` 只决定**格子之间的比例**，与最终画布宽高**互不冲突**——拼出来永远是一个填满的矩形，同层各块共享同一列宽/行高（块内部比例再极端也不影响这一点）。要留意的只是：面板被**两轴独立**缩放去填满格子，若某格的宽高比 ≠ 该面板画布的宽高比，**该面板内容会被非等比缩放**（圆点变椭圆、线宽一边粗一边细），工具会告警。默认（省略 `--rel-*`）格子按各面板纵横比分配 → 天然等比；显式给 `--rel-*` 就按你给的比例分配 |
| height 与 rel 的关系 | **显式给 `--width/--height` 时完全正交**：画布你定，`rel_*` 只在矩形内重分配格子。**省略 `--height`（auto）时**，`--rel-heights` 还会**决定推出的画布高**（画布高 = Σ行高，行高 = 自然高 × 倍数）——此时它不只是内部比例 |
| 缩放语义 | 组图**只重写坐标**（另按策略缩放 `stroke-width`），**不缩放 `font-size`** → 见下节 |
| 字母开关 | **不传 `--labels` = 不加字母**；**裸传 `--labels` = 自动编号 A/B/C…**；但自动编号**每次调用都从 A 重新开始**，所以逐行组图时**必须逐行显式给该行的字母**（`--labels A,B` / `--labels C,D,E` / `--labels F`）；`--labels none` 关闭 |
| 字母字体 | 默认 **`Arial`** + 粗体；族名可换成任意族或 **CSS 列表（回退链）**。字体**只**从字体库读（`$HOME/svg_grid/fontlib`）、不读系统字体 ⇒ 缺族会 W8、空库要出 PNG/加字母会**非零退出**。细则（serif 写法、`--list-fonts`/`--font-dir`、`fontlib/setup.sh`）见 `01_canvas_layout.md` §2 |
| 字母空间 | 用 `--labels` 且**未显式给 `--plot-margin`** 时，会自动留出字母带；显式给了 margin 就尊重它，并在字母会被裁时向 stderr 告警 |
| 字母加在哪层 | **只为"拥有该面板作为直接输入"的那一次调用加字母**；**组合图本身（行块）绝不要加字母**。嵌套时：各行为自己的面板加字母，**外层不要传 `--labels`**——传了会给行块整体重新编号，造成**字母重号/串位**（中途给组合图加字母是最常见的错法）；某格要跳过就留空（`--labels A,,C`）。平铺图（一次调用）则天然就是"最后一步加" |
| 空位/垫片 | 用 `--input none` 在序列里插一个空白格（R 的 `plot_grid(NULL, pp, NULL, rel_widths=c(0.08,1-0.16,0.08))` 等价物）；垫片的自然纵横比按 1:1 计，可用 `--rel-widths` 调 |
| 边距 | `--plot-margin` 给 1 个值时**四边同值**（给 4 个才是 `top,right,bottom,left`） |
| 描边宽度 | 描边图元必须**带显式 `stroke-width`**（自身或祖先声明）；靠默认值的图元放大后不会变粗（转换器会自动补齐） |
| 位图 | 位图面板（渲染器出图、`imshow` 热图、colorbar 渐变）以内嵌 `<image>` 进入成品，属正常 |
| dpi | `--png-dpi`（默认 600）**只写 PNG 的 pHYs 元数据**，不改像素数 |

**交付前必须成立的 5 件事**（可直接断言）：① 每个格子 `sx = sy ≈ 1`（`--plan` 直接读出；不为 1 须能说明为什么、且文字比例仍可接受）；② 每个格子**宽高比 == 面板画布宽高比**（防变形）；③ 转换产物里**不再有**
`<path>` / `<use>` / `<g transform>`；④ 外层拼装**给了 `--plot-margin 0`**；⑤ **一行内的面板等宽等高**。
（字母的样式/位置/墨宽、配色、落盘命名与图注另见 `01` / `03` / `04`；判据脚本见 `05` 与硬规则表的 dpi 行。）

## 位图 panel（svg + 位图混合组图）

位图 panel（ChimeraX 渲染的 pose、`imshow` 热图、外部图片）**可以**与本管线的矢量 panel 混在同一张图里：位图以内嵌 `<image>` 的形式进入成品（矢量图里带一小块位图，是正常且预期的）。

**做法**：给每个位图造一个"只含 `<image>` 的包装 SVG"，然后走正常流水线：

```xml
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="W" height="H" viewBox="0 0 W H">
  <image x="0" y="0" width="W" height="H" preserveAspectRatio="none"
         xlink:href="data:image/png;base64,…"/>
</svg>
```

要点：

- **`viewBox` 用"它将要占据的格子尺寸"**（不是 PNG 的原生像素尺寸）→ 组图时 `s = 1`，不需要 `--normalize-input-max-side`，也不触发告警。用原生尺寸而格子更小 → 会触发 W1。
- **必须写 `preserveAspectRatio="none"`**：SVG 默认的 `xMidYMid meet` 会把位图**留白居中**而不是铺满，与主库按 `x/y/width/height` 拉伸的语义相反。（转换器对**它自己**产出的 `<image>` 会自动补上；手写包装要自己写。）
- **位图清晰度 = "原生像素 → 格子像素"的倍率**：**倍率 > 1 就是放大糊**。要么让格子不小于原生像素，要么回渲染端按目标尺寸重出。
- 反过来，**矢量 panel 要按目标尺寸出图**：matplotlib 的 SVG `viewBox` 单位是 **pt**（`figsize × 72`），所以按设计尺寸出的面板其画布比 600 dpi 的格子小 `600/72 ≈ 8.33` 倍——直接放进去 `s ≈ 8.3`、文字只剩约 1 pt。见上一节的缩放规则。

## 让面板落在它将要占据的尺度上（关键）

组图端**不缩放 `font-size`**，所以：

> 组图后文字的相对大小 = 设计相对大小 ÷ `s`，`s = 格子尺寸 ÷ 面板画布尺寸`

`s ≈ 1` 时文字比例与设计一致；`s` 明显大于 1（面板被放大）时文字会**相对变小**——这是协议的设计（保证不同面板字号一致），**不是渲染或标签出错**。

**正规做法：不再手算格宽 —— 让工具算（`--plan`，只读、不出图）。**

给**与真跑完全相同**的输入/布局参数（`--ncol/--rel-*/--width/--height/--plot-margin/--gap`），加 `--plan`：

```
$ svg_grid plot-grid --input tagged/E.svg --input tagged/F.svg --plan --width 4961 --ncol 2 --labels
canvas 4961x1902.082
#0 tagged/E.svg  canvas 1052x744  cell 2456.500x1737.297  s=2.335x2.335  Δ+0.00%
note: #0 is not at its cell size (s=2.335); re-emit it at the cell size first:
svg_grid plot-grid --input tagged/E.svg --output tagged/E.cell.svg --width 2456.500 --ncol 1 --plot-margin 0 --normalize-input-max-side 2456.500
#1 tagged/F.svg  …（每个输入一行，形状同上）
```

`canvas` 是**成品画布**（= 最终 `.svg` 的 width/height、PNG 像素数）；`cell` 是该面板落进的格子；`s` 是
`格子 ÷ 面板画布`（`Δ` = 非等比的那部分，`0.00%` 即不变形）。`note:` 说这个输入要不要先归一化，要就
**把下一行命令照抄执行**：按格宽出图 + 归一化到格长边 ⇒ 画布正好等于格子 ⇒ 之后组图**不用再带**
`--normalize-input-max-side`。计划与真跑共用同一处布局计算，所以它报 `s=1` 就是 `s=1`；
改了 `--rel-*`/`--width`/`--plot-margin` 就再跑一次。

代价：**逐行按各自格宽归一化 ⇒ 不同行的纸面字号会不同**（例如 5.85 / 3.96 / 9.10 pt）。这是"保证 `s = 1`"的必然结果；若要全图字号一致，只能放弃 `s = 1`（文字与几何脱钩）或让各行格宽一致。

**同一行内的面板必须"同宽 + 同高"（不只是同宽）**：行内两格共享行高，所以纸面字号实际由 `格长边 ÷ 面板长边` 决定——两个面板**不同高**时，必然一个被放大、一个被缩小（实测同行 `2180×1500` 与 `2180×2500`：`1875/1500 = 1.25×` vs `1875/2500 = 0.75×`，后者从 5.0 pt 掉到 3.75 pt；逐格归一化只能保证每格 `s = 1`，改不了这个倍率的绝对值）。**定版式前先把同行的面板画成同一画布尺寸**，则每格 1:1、`sx = sy = 1`、纸面字号 = 设计字号。校验式：一行两张等宽 `W₀`、等高 `H₀` 的面板，当 `2·W₀ + gap + 2·margin` 恰等于行宽时，行高就是 `H₀`（例 `24 + 2180 + 40 + 2180 + 24 = 4448` ✓）。

**替代做法**（面板还没出图、或想一次定死）：在**出图端**按目标尺寸画 —— `figsize` 与所有以 pt 为单位的尺寸（字号、线宽、标记、刻度）同乘 `k = 目标格子宽 ÷ 画布宽`。注意 matplotlib 的 rcParam 里有些项的单位是**字号倍数**（`legend.handlelength/borderpad/labelspacing` 等），它们会随 `font.size` 一起缩放，**不能**再乘一遍。**要求全图纸面字号一致时首选这条**（出图端按 `figsize(in) = 格px/600` 画 + 全图统一 base 字号）；逐格归一化只适合"各路面板本来就按同一 scale 出图"的情形。

**别把字号放大到超过几何缩放比例 `s`**：matplotlib 面板的轴外元素（标题、y 轴标签、图例）是出图时按点烘死的，放大超过 `s` 会把它们裁掉或压到刻度上。`k = s` 是唯一可证安全的档位。

**核对 `s` 的方法**：用**带 `data-scale` 标签**的元素坐标反解（`sx = (out.x − out_panelbox.x) / (in.x − in_panelbox.x)`）。**不要**用 `data-panel-box` 那个 rect 判断——它通常没有 `data-scale` 标签，默认策略是"只平移"，尺寸本来就不变。

同理 **`data-panel-box="main"` 的出现次数不是面板数**：`plot-grid` 的**每个输出**（含只作容器的"行块"）都带一份，嵌套后层层叠加（实测 3 面板 2 行 ⇒ 9 次、4 面板 4 行 ⇒ 13 次）。要断言面板数就用 `data-scale="xy"`（每个真实面板恰好一个）或内层 `data-svg-grid-cell` 的格子数。

## 嵌套拼装（2 列 / 3 列 / 1 列）

1. **逐行组图**：`--ncol <行内面板数>` + `--width`，**不给 `--height`、不给 `--rel-*`** → 行块画布 = 内容自然尺寸。
2. **纵向拼**：把行块当外层输入，`--ncol 1` + `--width` + **`--plot-margin 0`**（同样不给 height/rel）→ 行块按各自自然高叠起来；同层各块**共享同一列宽**（块内部比例再极端也一样）。
   外层**必须显式给 `--plot-margin 0`**：若不显式给，默认的 24 会把每个行块整体再缩一次（`(W−2×24)/W`，W=4961 时约 **0.99**），最终 `s = 行级 s × 0.99` 而不是 1——这是一次**静默的 1% 压缩**。
3. 字母在**第 1 步**（行那次调用）加；**外层不传 `--labels`**——规则见上表「字母加在哪层」。
4. 想改变某行的高度占比时，才在外层给 `--rel-heights`（倍数，如 `1.05,1,1`）。注意这**会改变该行格子的宽高比** ⇒ 该行面板被纵向拉伸（工具会因此告警）；要既改行高又不变形，得改画布宽或该行面板的纵横比。
5. **行调用的 `--plot-margin` 用 4 值给**：给单值 `150` 会让该行块**上下也各留 150**（3 行合计多 900 px 边距）⇒ 成品高比 A4 多出 ~600 px。想"横向留白、竖向交给外层"就用 `0,m,0,m`（`top,right,bottom,left`）——**`m` 必须等于你格子模型里的 margin**（见条目 8），否则该行字号会漂移。
6. **同纵横比的面板优先用"单次 flat 调用"**：逐行 + 外层只在各行纵横比**不同**时才必要。同纵横比硬走逐行时，行块自带的上下 margin + 字母带会把高度顶爆 A4——实测 2×3 逐行拼出 `4961 × 7618`（≠ 7016），外层强拉 `--height 7016` 则 `sy = 0.918`（纵向压缩 ~8%，触发 W1/W2）。同一批面板改用**一次 `--ncol 2` flat 调用** + `--width 4961 --height 7016` 即精确命中 A4、`sx = sy ≈ 1`。
7. **字母带按行累加**：逐行加 `--labels` 且未显式给 `--plot-margin` 时，**每一行都自动留一条字母带** ⇒ 成品高 ≈ Σ(各面板按画布宽归一后的高) + 行数 ×(字母带 + 底部边距)。实测每行 ~150 px（S36：4 面板内容和 7650 px → 成品 8168 px，+6.8%）。**别把"成品高 ≠ Σ面板高"当出错**——拿成品高反推面板尺寸会系统性偏大。**若要成品高精确等于 Σ 面板高**：把这几张面板当"**一行内 n 格**"（一次调用给 `--labels A,B,C,D`），或先把它们拼成一整张面板再当单输入——两者都只留一条字母带（代价是失去逐行独立排版）。
8. **行调用自动字母带的横向内缩 = 默认 `--plot-margin`（24）**：`s = 1` 成立的前提是"格子模型里的 margin"与"行调用留给字母带的边距"相等。若给行调用显式 `--plot-margin ≠ 24`（或用 4 值改了横向值）、或改画布宽使自动内缩 ≠ 24，该行面板就被 `(W−2m)/W ≠ 1` 缩放；协议**不缩放 `font-size`** ⇒ 该行字号漂移（W1 会报，但得有人读 stderr）。

## 跑完必须读 stderr

`plot-grid` 把"看着成功其实悄悄错"的情形打成 stderr 告警（**W1–W7，一律非致命**：退出码与产物字节都不变）。
**每条告警自己会说明"发生了什么 / 为什么重要 / 怎么修"**，照它给的修法改即可——本指南**不再逐条罗列**
（清单、判据与容差理由见 `tools/svg_grid/ARCHITECTURE/warnings.md`）。所以：**跑完先看 stderr**，
别只看产物在不在。

- **量 `s` 时分清两层**："层内格级 `s`"（应为 1）与"原始面板 → 成品的总缩放"（= 归一化倍数，本来 ≠ 1）
  是两回事，别把后者当成出错。
- 转换器另会报一条**看着吓人、实际多无害**的告警（`<style>` CSS 块不被解析）：matplotlib 的 `<style>` 块
  通常只声明 `stroke-linejoin`/`stroke-linecap` 这类**装饰**属性、没有东西需要重算。**判断方法**：打开该块
  看一眼——出现长度 / `font-size` 才真的没被缩放。

## 出图端与调用方仍须自己做的四件事（工具管不了）

**其余"出图端坑"都已进工具**（mathtext/`<tspan>`、祖先 `<g transform>`、`textLength`、零面积描边路径、
字形孔、`clipPath`、非等比 `<image>`）——正常链路里不会再遇到，遇到也会由 stderr 出声。仍归调用方的只有这四件：

1. **`Path.to_polygons()` 默认丢开放子轮廓**：matplotlib 默认 `closed_only=True`，把无 `Z`、末点 ≠ 首点的
   子轮廓**整批静默丢掉** ⇒ 传 `closed_only=False`，开放的按"仅描边"处理（`<line>`/`<polyline>` + 显式
   `stroke-width`），不要当零面积多边形。
2. **`pdftocairo -svg` 的 `-x/-y/-W/-H` 静默失效**（poppler 行为，产物永远整页 viewBox）⇒ 必须事后按
   `viewBox` / `<g transform>` 裁。
3. **幂等核对看 PNG、不看 svgz**：matplotlib 往 SVG 写 `<dc:date>`（并由此派生 clip-path id）⇒ 同一输入
   重跑 `.svgz` **字节会变**；判据应是"PNG 逐字节/像素一致"。
4. **CJK 面板的静默缺字**：matplotlib 的 `font_manager` **默认不索引 `.ttc` 字体集合**，而本机 CJK 字体只以
   `.ttc` 存在 ⇒ `font.serif = "Noto Serif CJK SC"` **匹配不上**、静默回落到无 CJK 字形的 `DejaVu Serif`
   （PNG/SVG 照出、退出码 0，只在 stderr 冒 `UserWarning: Glyph NNNNN missing from font(s) ...`）。
   - ① 出图脚本开头 `fontManager.addfont("/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc")`，
     并用**注册后的 face 名**（实测是 `Noto Serif CJK JP`，**不是**文件名里的 `SC`）；
   - ② **字形覆盖逐符号查**：CJK 字体集不含 `⋔ ⊣ ⊸ ⊥ ⊤` 这批数学/转向符号，而 matplotlib 只对**整段文本**
     选一个 family、**不逐字回退** ⇒ 缺哪个静默丢哪个（实测 `⊣` 整字消失，改用 `—|`）；
   - ③ 交付前把 `Glyph ... missing` **一律当失败**（死白带检查只查"有没有墨"，查不出"该有的字没画出来"）；
     别把它跟其它 warning 一起 `grep -v` 掉。

## 交付前核查

逐条自查见 `05_checklist.md`（内容 / 版式 / 程序化 / 落盘 / 观感五项，**vision 目视成品图是硬步骤**）。
本路线专有的不变量已并入上面的「硬规则」表与「交付前必须成立的 5 件事」；程序化判据**只跑一个脚本**：

```bash
~/.venv/bin/python tools/svg_grid/scripts/check_deliverable.py <成品.png> --margin <本图预算> [--gap]
```
