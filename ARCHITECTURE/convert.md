# 转换器 `convert/`（外源方言 → 协议格式）

> 独立 crate，与主库 `src/` **解耦**（不改主库代码与依赖）。协议与分界原则见 `tag-protocol.md`。
> 主库的 path 只搬坐标，**"填充/描边 → polygon/polyline"这类语义定型只在 `convert/` 做**。

## 1. 职责与边界

- **翻译手段**：`<g transform>` 烘焙进叶子坐标（含 `clipPath` 一致性）、`<use>`/`<defs>` 按引用展开后删除、
  `<path d>` → `<polyline>`/`<polygon>`、文字 `translate(x y) rotate(a)` 与裸 `translate(tx,ty)` 归一化为
  `x/y + rotate(a x y)`、`<image>` 的**轴对齐（含非等比）**缩放与翻转烘焙进 `x/y/width/height` 并补
  `preserveAspectRatio="none"`、`clipPath` 体内的矩形 `<path>` 矩形化为 `<rect>`、
  **描边图元补显式 `stroke-width`**、`style` 百分比解析。
- **纯平移**的祖先可**精确**烘进 `<text>`/`<tspan>` 的 `x/y` 与 `rotate(a,cx,cy)` 中心
  （依据 `T(e,f)·R(a,cx,cy) == R(a,cx+e,cy+f)·T(e,f)`）；**带缩放/旋转的祖先**仍 fail-closed
  （烘进去会静默改变字号/朝向）。
- `translate_dialect` 的阶段顺序：`normalize_transforms` **先于** bake，否则 svglite 的
  `translate(x,y) rotate(a)` 文本碰上平移祖先会先被写入 x/y、再也归一化不了。
- **`<path>` 一项如今主库也能搬，但 convert 仍要转**——"填充/描边 → 形态"是 **paint 语义**（见 §3）。
- **`convert/` 与主库各有一份 path 处理**（用户裁决"直接做进主库"）：同一容差常数（`0.02` 源单位）、
  同一语义、两侧注释互指（防漂移，见 `tag-protocol.md` §4）。

## 2. 仍然 fail-closed 的构造（其余都已能进流水线）

非 `<svg>` 根、嵌套 `<svg>`、`<symbol>`、解析不到的 `<use>`、**祖先带缩放或旋转的 `<text>`**、
**非轴对齐**的 `<image>` 变换、非轴对齐矩形的 `clipPath` 子轮廓、逗号分隔的 `viewBox`、非正画布尺寸、
`<defs>` 内可渲染图元（允许出现的是 `<clipPath>` 下的 `<rect>`，须与几何同策略，裁剪框才跟着缩放）、
以及未验证的第三方导出（Inkscape/Illustrator）。

**失败必须响亮**：遇到不能处理的构造**报错退出并列出位置**，绝不产出"看起来成功"的错图
（本机实测过 R `cowplot` 文件级组图**静默产出 0.00% 墨量的全白图**，仅告警、退出码 0——正是要避免的失败模式）。

## 3. `<path>` 的出形态决策表

`convert/src/translate/shapes/`（原 `shapes.rs`，R8 拆成目录）。**这是"字形孔被填实"的关键。**

| 情形 | 出形态 |
|---|---|
| 只填充 | 全部子轮廓合进**一个** `<polygon>`，**每条子轮廓显式闭合**（首尾点不等就补首点）；`fill-rule` **照抄，不改成 `evenodd`** |
| 只描边 | 每条子轮廓一个 `<polyline>` |
| 填充 + 描边 + **多**子轮廓 | 拆成"只填充 polygon（`stroke="none"`，连 `style` 里的 stroke 声明也删）+ 每子轮廓只描边元素（`fill="none"`）" |
| 填充 + 描边 + **单**子轮廓 | 保持修复前形态（`closed` ⇒ polygon，否则 polyline），**逐字节不变** |

**为什么（两条都是实测出来的）**

1. **不合并** ⇒ 字形（外轮廓 + 内轮廓在**同一条 path 里**、常无 `Z`、无自身 `fill`）被拆成各自 nonzero
   填充的图形 ⇒ **孔被填实**（真实 `pdftocairo -svg` 产物复现）。
2. **合并但让环开放** ⇒ 环之间的"连接边"是**真实边**，会改变绕数（实测源渲染 (31,29) 为黑、转换后为白）；
   **显式闭合**后连接边正反各走一次、净绕数贡献 0，填充域等于源"所有子轮廓共用同一 `fill-rule`"的语义。
   同理，合并后的 polygon 若仍带 `stroke`，连接边会被描出来（实测两个互不相连的闭合矩形被一条对角黑线连起来）
   ⇒ 多子轮廓 + 描边**必须**拆成 fill/stroke 两类元素。

**已知边界**：判断"是否有描边"只看该图元**自身**的属性/`style`；描边只来自祖先 `<g stroke=…>` 时，
多子轮廓填充路径仍会画出连接边——两个受支持方言都把 paint 写在图元自身（实测 0 例），**明确不做**。

**未采用**：直接 fail-closed（多子轮廓 + 描边在 ggplot2 `geom_sf` 多环、matplotlib `PathPatch` + `edgecolor`
下是真会出现的，拒收会逼调用方回到栅格化）；强行改成 `evenodd`（会改变重叠子轮廓的并集语义）。

**调用方自己的矢量手术仍要小心**（工具管不了）：matplotlib 的 `Path.to_polygons()` **默认
`closed_only=True`**，会把开放子轮廓（无 `Z`、末点 ≠ 首点）整批静默丢掉 ⇒ 必须传 `closed_only=False`。

## 4. 幂等语义：校验先行，完整产物才跳过

任何输入都**先跑完整校验**；只有"恰好一个 panel box **且**存在 `data-scale` 分组 **且**无不兼容构造"
才判为已转换。此时退出 0，且**若显式给了 `--output` 就逐字节复制过去**。"半成品"不 `--force` 一律报错退出 1。

**原因**：修复前只要输入含 panel box 就跳过全部校验、且显式 `--output` 也不写文件却退出 0——
成批跑时下一级会以"文件不存在"莫名失败而退出码报成功，属**静默失败**。

**幂等核对看 PNG、不看 svgz**：matplotlib 会往 SVG 写 `<dc:date>`（并由此派生 clip-path id），
同一输入重跑 `.svgz` **字节会变**；判据应是"PNG 逐字节/像素一致"。

## 5. 输入来源兼容性（2026-09-16 实测，2026-09-20/27 更新）

| 来源 | 图元构成 | 结论 |
|---|---|---|
| R `svglite`/`ggplot2` | 绝对坐标、`<path>` = 0；circle/rect/line/polyline/polygon/text + `<clipPath>` | **已可适配**：`convert/` 后可直接组图（真实 panel 端到端跑通）；`textLength` 随归一化一起缩放 |
| Python `matplotlib` | `<path>` 292、`<use>` 436、带 transform 的 `<g>` 42、`<image>` 6 | **已可适配**：由翻译层吸收（烘焙/展开/path 转换/位图翻转）；S22 端到端验证。**自动 mathtext**（对数轴/`LogNorm` 色标 → `<g translate><text><tspan x y>`，源码里一个 `$` 都没有也会出现）与**多行文本裸 `translate`** 也已支持（2026-09-20） |
| 外源 PDF `pdftocairo -svg` | 字形轮廓 `<use>`→`<defs><g><path>`（无 `Z`/无自身 `fill`）、`<clipPath>` 内 `<path>`、非等比 `<image>` | **已可适配**（2026-09-20）：字形孔靠"单 polygon + 显式闭合"保留、clip `<path>` 矩形化、非等比 `<image>` 烘框。**仍须调用方处理**：`-x/-y/-W/-H` 对 `-svg` **静默失效**（poppler 行为，永远整页 viewBox）⇒ 事后按 viewBox/`<g transform>` 裁 |
| 栅格（PNG/JPEG、ChimeraX 渲染） | — | 主库支持 `<image>` 坐标搬运，可随面板进入成品（矢量图内含一小块位图）；**保持 600 dpi 原分辨率、不降采样** |

## 6. 渲染与依赖注意

- 主库 resvg 需 `raster-images` feature（**已启用**）：缺它时内嵌 `<image>` **静默变白**
  （`resvg/src/image.rs` 只打一条 `log::warn!`）；实测同一区域唯一颜色数 6 → 792（cairosvg 817）。
- `--target-width`/`--scale` 未实现（**不做**，见 `../ARCHITECTURE.md` §已定型的决定）；`--auto-fix` 一类的上游文档声明
  而实现里没有的功能，**一律不按文档实现**（按实现取舍，标"待议"）。
