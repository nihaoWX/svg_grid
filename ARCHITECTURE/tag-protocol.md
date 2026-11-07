# 标签协议（组图器接受的输入格式）

> 主库 `src/` 的**输入契约**：输入长什么样、主库认哪些构造、与 `convert/` 的分界在哪。
> 布局语义见 `layout.md`；告警见 `warnings.md`；转换器侧见 `convert.md`。

## 1. 两条必需项

组图器**逐图元重写坐标，不套外层 transform**。

| 项 | 形式 | 缺失后果 |
|---|---|---|
| 面板盒 | `<rect data-panel-box="main" x y width height/>`（树内任意位置） | **直接报错退出** |
| 缩放策略 | `<g data-scale="xy\|x\|y\|position\|none">`（逐层继承，子级覆盖） | 默认 `none`：只平移不缩放，面板不适配格子 |

策略语义：`xy` 坐标双向重映射、尺寸按 `sqrt(sx*sy)` 缩放；`position` 坐标重映射、**尺寸一律不变**；
`none` 整块平移、内部坐标不重算。**`font-size` 在任何策略下都不缩放。**

**`data-panel-box="main"` 的出现次数不是面板数**：`plot-grid` 的**每个输出**（含只作容器的"行块"）都带一份，
嵌套后层层叠加（实测 3 面板 2 行 ⇒ 9 次、4 面板 4 行 ⇒ 13 次）。要断言面板数就用 `data-scale="xy"`
（每个真实面板恰好一个）或内层 `data-svg-grid-cell` 的格子数。

## 2. 主库受支持的构造

`circle` / `text` / `tspan` / `use` / `line` / `rect` / `image` / `polyline` / `polygon` /
**`path`**（2026-09-28 R5 起），外加 `stroke-width` 与 `rotate(angle, x, y)`。

- `<image>` 与 `<rect>` 同款处理 `x/y/width/height`；
- `<tspan>` 的 `x`/`y` 是**绝对坐标**（覆盖父 `<text>` 的当前位置），按轴独立重写、与 `<text>` 一起搬运
  ——否则 matplotlib 的 mathtext 逐字 `<tspan>` 会留在原坐标（标签静默错位）；
- `<defs>` 子树里的受支持图元**同样被搬运**（`<use>` 引用的几何因此跟着走；id 被命名空间化成
  `svg-grid-cell0-g`，`<use>` 同步指向）；
- 渲染内嵌栅格需要 resvg 的 `raster-images` feature（**已启用**）：缺它时 `resvg/src/image.rs` 在
  `not(feature = "raster-images")` 分支**直接放弃解码、只打一条 `log::warn!`** ⇒ 成品 PNG 里内嵌的
  `<image>`（matplotlib 的 colorbar 渐变、`imshow` 热图）**静默变白**。

**主库不处理**（原样留在原坐标 ⇒ 版面错乱）：**任何非 `rotate` 的 `<g transform>`**。
几何写在 CSS `style` 里的值**不重写**（例如 svglite 的 `stroke-width`）——唯一例外是
`--normalize-input-max-side` 归一化时会一并缩放 `style` 里声明的长度与 `textLength`（见 `layout.md`）。

## 3. 分界原则（2026-09-28 R5 确立，取代旧的"主库只认协议图元"）

主库要具备**最小的、完备的 SVG 表达能力**——**凡"被画出来的东西"（图元自身的几何）主库都要能表达并搬运**。

- **`<g transform>` 不属于**：它**不是图元、是坐标系机制**，效果可**等价约化**为"重写该范围内的全部坐标"，
  故主库不需要把它当成一种能力，**烘焙它归 `convert/`**
  （用户原话：*我可以直接调整这个地方的全部坐标实现整体变换*）。
- **`<use>/<defs>` 属于**：它不是"画的形状"，但组图要把**多个来源文件**的面板**内联进同一棵树**，
  **id 必须命名空间化**，否则两个面板的 `#glyph1` 会互相覆盖 ⇒ **不可约化**。
  实测：`<defs>` 里的 polyline 坐标与直写 polyline **被同样重写**。

**判据（以后遇到新构造照这个判）**：*它是「被画出来的图元」，还是「坐标系 / 引用 / 语义」？*
图元 ⇒ 主库；**不可约化**的引用机制 ⇒ 主库；**可约化为坐标重写**的坐标系机制、以及需要判断"怎么画"的
语义（paint/样式继承、`<clipPath>` 矩形化、非轴对齐 `<image>` 翻转）⇒ `convert/`。

## 4. `<path>` 的保真边界

- 坐标按轴重写：`H` 只动 x、`V` 只动 y、`A` 的 `rx/ry` 按轴；**`A` 的 `x-axis-rotation` 与两个 flag
  一个字节都不许动**；小写（相对）命令当矢量按轴缩放；隐式重复照展。
- **唯一不能保真的情形**：**非等比缩放（`sx ≠ sy`）撞上旋转的椭圆弧** ⇒ 该弧**展平成折线**
  （容差 `DEFAULT_TOLERANCE = 0.02` 源单位 ≈ 0.2 px，与 `convert/src/path.rs` 同值、两侧注释互指）；
  其余命令保真重写。
- **主库的 path 只搬、不改语义**：填充/描边 → polygon/polyline 形态（`fill-rule`、显式闭合、多子轮廓拆分）
  **仍只在 `convert/` 做**——走 convert 的链路一切照旧；**直接喂主库**的 path 保持原形态、只重写坐标，
  字形孔这类语义问题因此只在 convert 侧解决（见 `convert.md` 的出形态决策表）。
- **双实现防漂移**：主库 `src/path/` 与 `convert/src/path.rs` 各有一份 path 处理（用户裁决"直接做进主库"）。
  缓解：**同一容差常数、同一语义、两侧模块注释互指**；任何一侧改 path 语义时**必须同时看另一侧**。
- **镜像（负）等比 + 旋转弧**：单轴镜像会翻转弧的**手性**，必须翻转 `sweep` 并取反 `x-axis-rotation`；
  而守卫只判 `|sx| ≠ |sy|` ⇒ 该情形会漏掉、静默失真。**当前 CLI 不可达**（`sx,sy = cell/panel` 恒正；
  `rel_*` 拒绝负值；`sx = sy < 0` 那种是 180° 旋转、保手性），故**只登记**：若将来引入翻转，
  必须在此处补 sweep/rotation。

## 5. 失败口径

- **结构垃圾 fail-closed**：缺面板盒、非 `svg` 根、非正画布尺寸等 ⇒ **报错退出并列出位置**，不产图。
- **属性值层面的宽松**（2026-09-28 R6）：畸形 `<path d>` 与非有限数值（`nan`/`inf`）⇒
  **警告（W7）+ 原样透传/写入**，`rc` 与产物字节都不变（理由：浏览器对轻微畸形的宽容有道理，
  R5 的严格解析曾把"以前跑得通的输入"变成整图 exit 1 ⇒ 通配面收窄）；
  而**属性串根本不成形**（如 `rect x="abc"`）**仍 fail-closed**——那是另一种错误，不可能来自 convert。
