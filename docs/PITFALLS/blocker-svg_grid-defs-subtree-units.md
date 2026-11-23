# 阻塞点：定义元素（`<defs>` 子树）没有按「引用者的缩放单位」走

- 轮次：R46　日期：2026-11-24　代码基线：`196da47`
- 本轮只碰定义元素这一族：`src/transform/**` + `src/normalize.rs` + 测试；`src/warnings/**`、`src/cli/doctor/**`、`src/dom.rs`、`src/convert/**` 一行未动。

## 现象

`<defs>` 里的 pattern / gradient / mask / filter / marker 按**书写位置**继承 `data-scale`，而引用它的图形按**自己**的 `data-scale` 缩放。两边不一致时，定义里的长度要么没缩放（图形缩放了）、要么多缩了一遍。

最刺眼的一例：`<pattern patternUnits="userSpaceOnUse" width="20" height="20">` 写在根（没有 `data-scale`），引用它的 `<rect>` 也在根，但 panel 放大 2.48 倍时：

- 产物 tile 仍是 `width="20" height="20"`；
- pattern body 却被缩成 `M 0 49.600 L 49.600 0` + `stroke-width="4.960"`；
- body 落到了自己 tile 的窗口外 ⇒ **渲染出来 ink = 0**（源 1128 个非白像素）。

也就是说，同一份 pattern 的「窗口」和「内容」用了两套坐标系，结果什么都没画出来。这是本轮四项里唯一的 P0。

## 复现

源（`docs/PITFALLS/round-data/R46/test.py` 里的 `A1_none`，注入到 `test.py` 的临时目录）：

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>
<defs><pattern id="h" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20">
<path d="M 0,20 L 20,0" stroke="#000" stroke-width="2"/></pattern></defs>
<rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
</svg>
```

```bash
svg_grid plot-grid --input A1_none.src.svg --output A1.svg --ncol 1 --plot-margin 0 --width 248
d1render A1.svg A1.png 1          # 248x198.4 画布
```

修前：tile `width="20"`、body `M 0 49.600 L 49.600 0`、`stroke-width="4.960"`，ink **0**（参考 1128）。
修后：tile `width="20"`、body `M 0,20 L 20,0`、`stroke-width="2"`，渲染与手写 oracle **xor 0 / maxdiff 0**。

同一族还有 4 例，源、命令、期望值都在 `round-data/R46/test.py`（`A1`–`C2`、`D1`–`D3`），逐条给数字。

## 根因

`src/transform/walk.rs::transform_element` 只按书写位置取策略：

```rust
let policy = element.attr("data-scale").map(parse_policy).unwrap_or(inherited);
```

`<defs>` 走普通递归，`inherited` 一路是 `transform_svg` 传下来的 `ScalePolicy::None`（根本身没有标签）。于是「定义里的长度不缩、引用它的图形缩」。

resvg 不是这么算的：它在**引用者各自的 user space** 里解释定义里的数值（`usvg-0.47.0/src/parser/paint_server.rs` 的 `resolve_*` 全部跑在引用者的 `abs_transform` 下）。所以一份被两个不同 policy 引用的定义，本来就等价于两份不同的定义——这也是本轮选「按引用者复制」而不是「取最宽 policy」的原因：取最宽只是把错缩放换个范围保留。

另外三处同源（都是「拿错了坐标系」）：

1. **marker + `viewBox`**：`usvg-0.47.0/src/parser/marker.rs` 的 `draw_marker` 在有 `viewBox` 时已经 `ts = ts.pre_scale(sx, sy)` 把内容映射进视口，我们却再把 body 缩一遍 ⇒ 二次缩放。
2. **gradient / filter 的 units**：`paint_server.rs:230-233` 的 `convert_units` 走 `resolve_attr`，会沿 `href` 链找 `gradientUnits`；我们只读自己身上的属性。并且 `rewrite_pair` 要求 x1+y1 成对出现，`<linearGradient href="#base" x1="10">` 这种只声明一个坐标的真实形态直接被跳过。
3. **`src/normalize.rs::scale_element` 盲缩放**：一张固定属性表（含 `x/y/width/height/markerWidth/refX/...`）不区分「用户空间长度」和「bbox 分数」。对 `patternContentUnits="objectBoundingBox"` 的元素再缩一次，等于把 0.25 变成 0.125。

## 修法

### A. 定义按引用者走（`src/transform/defs.rs` 新增）

- `defs::split(root)` 先扫全文档：非定义元素身上的 `url(#id)` / `href="#id"` 记为 `direct[id] ⊆ {引用者的 policy}`；定义内部出现的引用记为边 `owner → id`。再做一次单调不动点 `refs[D] = direct[D] ∪ ⋃ refs[owner]`，**沿引用图传递**——pattern `p` 里 `fill="url(#g)"` 时，`g` 同样拿到 `p` 的全部 policy。
- 一个 id 只被一种 policy 引用（真实缺陷形态）⇒ **不复制**，行为靠新语义自然修正，产物只改该改的数字。
- 被多种 policy 引用 ⇒ 按 policy 排序（`Xy < X < Y < Position < None`），第一个沿用原 id，其余各复制一份、id 加 `--svg-grid-<tag>` 后缀（`xy|x|y|position|none`），撞名加 `-2`、`-3`；每个副本内部的引用改成指向**自己那份**副本。
- 无引用者 ⇒ 保持书写位置的策略（不 fail-closed）。理由：`<style>` 里的 `url(#…)` 我们不解析（R37 已定的既有限制），硬判 `None` 会让「写在 `data-scale` 组里、暂时没人引用」的定义静默停止缩放——那是另一类回归。这一点偏离了派单字面上的「无引用者按 None」，已在报告与账本里点明。

`walk.rs::transform_element` 现在先问 `context.defs.policy_of(element)`，命中就以它为准。

### B. body 用「只缩不平移」的策略

pattern / marker 的 body 是**tile 局部**且被 tile 裁剪的（`clip1/2/3` 实测：把 body 平移一个 tile 原点，ink 从 1128 掉到 0），所以 body 不能跟着 `map_point` 平移。新增 `ScalePolicy::ScaleX` / `ScaleY`，由 `body_policy()` 从定义自己的 policy 派生：

| 定义 policy | body 策略 |
|---|---|
| `xy` | `Scale`（双轴缩、不平移） |
| `x` | `ScaleX` |
| `y` | `ScaleY` |
| `position` / `none` | 不递归进 body（逐字保留） |

`A2_x` 就是这条：body 必须是 `14.880 × 6`，不是 `14.880 × 14.880`。

marker 有合法 `viewBox` 时 body 直接不碰（B 项），只缩 `markerWidth/markerHeight/refX/refY`。

### C. units 沿 `href` 解析（`src/transform/links.rs` 新增）

`links::resolved_units(element, index, attr, default)` 按 usvg 的 tag 规则沿链找：gradient 的坐标只认同 tag 的链接，`gradientUnits`/`spreadMethod`/`gradientTransform` 两种 gradient 都认，pattern 只认 pattern，filter 只认 filter。gradient 坐标改为**逐属性**改写（`rewrite_axes`），所以 `x1="10"` 单独出现也能缩。

`maskUnits` / `maskContentUnits` **不**沿 `href` 继承——`usvg-0.47.0/src/parser/mask.rs:20-30` 用的是 `node.attribute(AId::MaskUnits)`，没有 `resolve_attr`。这条写进了 `links.rs` 的模块注释和 `dispatch.rs` 的 mask 分支注释，**下一轮不要"顺手统一"**。

### D. 归一化按 units 门控（`src/normalize.rs::scale_definition`）

`scale_element` 遇到定义标签就分流，逐组判 units：

| 标签 | 缩什么 | 条件 |
|---|---|---|
| pattern | tile `x/y/width/height` | `patternUnits="userSpaceOnUse"`（沿 href 解析） |
| pattern | 子树 | `patternContentUnits="userSpaceOnUse"` 且**没有**合法 `viewBox` |
| pattern | `viewBox` | 总是缩 |
| marker | `markerWidth/markerHeight/refX/refY` | `markerUnits="userSpaceOnUse"`（只看自身） |
| marker | 子树 | 同上且**没有** `viewBox` |
| mask | box | `maskUnits="userSpaceOnUse"`（只看自身） |
| mask | 子树 | `maskContentUnits="userSpaceOnUse"`（只看自身） |
| filter | box | `filterUnits="userSpaceOnUse"`（沿 href 解析）；子树保持历史行为 |
| gradient | 坐标 | `gradientUnits="userSpaceOnUse"` |
| clipPath | 子树 | `clipPathUnits="userSpaceOnUse"`（默认 userSpace） |

`markerWidth/markerHeight/refX/refY` 已从通用属性表里删掉，否则 `<marker>` 之外的地方没有第二道闸门。

## 实测（修前 / 修后 / 参考）

渲染器 `d1render`（resvg 0.47.0），画布 248 × 198.400，缩放 1；参考值 = 手写 oracle SVG 渲染结果。`xor` = 与 oracle 不同的像素数，`maxdiff` = 最大通道差。

| 用例 | 修前（`196da47`） | 修后 | oracle ink |
|---|---|---|---|
| A1 无 `data-scale` 的 pattern | xor 1128 / max 255 / ink **0** | **xor 0 / max 0 / ink 1128** | 1128 |
| A2 `x` 引用者 | xor 6030 / max 255 / ink 6750 | **0 / 0 / 1080** | 1080 |
| A3 `position` 引用者 | xor 2331 / max 255 / ink 2806 | **0 / 0 / 475** | 475 |
| A4 两种 policy 引用同一 pattern | xor 4130 / max 255 / ink 4200 | **0 / 0 / 480** | 480 |
| B1 marker + `viewBox` | xor 194 / max 255 / ink 1720 | **0 / 0 / 1551** | 1551 |
| C1 gradient units 沿 href | xor 30000 / max 169 / ink 11400 | **0 / 0 / 30000** | 30000 |
| C2 根上的 gradient | xor 30000 / max 152 / ink 11400 | **0 / 0 / 30000** | 30000 |
| D1 归一化 + bbox pattern | xor 18432 / max 255 | **0 / 0** | — |
| D2 归一化 + strokeWidth marker | xor 2360 / max 255（ink 4880→2600） | **0 / 0**（4880→4880） | — |
| D3 对照：userSpace marker | 0 / 0 | 0 / 0 | — |

D1 值得单说：旧行为把 tile 0.25 缩成 0.125，**ink 前后都是 12288**——只数墨点根本看不出来，必须逐像素 `xor`。这是本仓「同一处三连翻车」（R38 退化用例 / R39 宽松判据 / R40 方向写反）之后新加的硬要求：**先拿参考渲染器定哪边对，再写期望值**。

## 变异

`cargo test --offline <filter>`，5 条全部变红（`/tmp/mutate.py`）：

| # | 变异 | 结果 |
|---|---|---|
| M1 | `context.defs.policy_of(element)` → `None`（退回纯继承） | RED |
| M2 | `X => Some(ScaleX)` → `Some(Scale)`（x 也缩 y） | RED |
| M3 | `marker_body_is_verbatim` → 恒 `false`（viewBox 不再保留 body） | RED |
| M4 | dispatch 的 `resolved_units(..,"gradientUnits"..)` → `units_of(..)`（units 不沿 href） | RED |
| M5 | normalize 的 `patternUnits==UserSpaceOnUse` → `true`（tile 又盲缩） | RED |

## 回归

- `cargo test --offline`：**419 → 435**（238 lib + 59 main + 138 selftest），0 failed；未删任何既有测试。
- `cargo fmt --all --check` 干净；`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` OK；`svg_grid doctor` = **57 pass / 0 fail / 1 skip**（`matplotlib/mathtext`，注册为不支持）。
- 既有产物：**223 个 `*.svg` × 4 条命令（`convert`、`plot-grid --width 600 --ncol 3`、`plot-grid --plan`、`convert --report`）= 892 次比对，与基线二进制逐字节相同（0 差异）**。原因：语料里「定义写在 `data-scale` 组外、却被组内图形引用」的文件数为 0（220 fixture + 617 research SVG 里 `data-scale` 之前出现 `linearGradient|radialGradient|pattern|marker|mask|filter` 的文件数 = 0），单 policy 路径又不复制，所以本轮修的是语义而不是既有字节。真实用户路径（未打标签 → 自动 `convert`）不踩这个坑：`src/convert/tag.rs::wrap_root_content_in_xy` 会把 `<defs>` 一起塞进 `<g data-scale="xy">`；本轮**没有**改 convert。
- 旧证据 `docs/PITFALLS/round-data/R3..R45/test.py` 逐轮跑过：**除 R19 外全部 rc=0**（R45 的 E 段需要 `SVG_GRID_OLD_BIN`，不给则 SKIP，属正常；R27 没有 `round-data` 目录）。
- **R19 是既有失效，不是本轮回归**：它的 `test.py` 把用例数写死（`BASELINE = 190`、`COUNTS = {lib 60, main 57, selftest 73}`，见 `docs/PITFALLS/round-data/R19/test.py:47-49`），而基线 `196da47` 的实际计数是 231 / 59 / 129 = 419。在 `/tmp/r46base`（`196da47` 的 worktree）里跑同一份脚本得到 **12 条 FAIL**，与本轮无关。本轮没有改它：改旧证据的期望值等于造假材料，且 R19 的 `meta.toml` 自己写明「用例数写死 190」。

## 遗留

1. `<style>` 文本里的 `url(#…)` 不解析（R37 已定），所以 CSS 引用者拿不到 policy；无引用者的定义按书写位置缩放，不 fail-closed。
2. 复制只发生在 `transform_svg` 内，`--plan` / `--report` 的计数不变；本轮没有出现因复制导致的产物差异，因此无需点名前后数字。
3. `patternTransform`、pattern 内 `<text>`、滤镜图元长度、嵌套 pattern 仍是遗留（R43 起就在）。
4. `clipPath` 本轮只在 normalize 里加了 units 门控，transform 侧仍不缩放其子树（与 R45 一致，未扩大范围）。
5. 定义被复制时 id 后缀是 `--svg-grid-<tag>`；`src/ns.rs::namespace_ids` 在其后加 `svg-grid-cellN-` 前缀，两者不冲突（已由 `A4_copy` 用例钉住）。
