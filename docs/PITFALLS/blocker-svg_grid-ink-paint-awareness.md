# 阻塞点：W6/W5 的墨迹判据把「不上色」的内容也算成墨；`--rel-widths/--rel-heights` 静默接受 0 权

- 轮次：R66
- 日期：2026-11-25
- 代码前：`2449f1e`（HARNESS 收口；代码同 `71ae88f`）
- 类别：① 工具缺陷（两处）

## 现象

### 1. 不画东西的元素被算成墨（W6 假阳性）

`src/ink/walk.rs::ink_bbox` 是 W6（内容溢出自画布）与 `<symbol viewBox>` 实例裁剪
（`src/label_clip.rs::instance_draws`）的唯一输入，但它只跟踪 `stroke` / `stroke-width`，
不看 `fill` / `fill-opacity` / `stroke-opacity` / `visibility` / `opacity`。
于是「声明了但不上色」的内容照样贡献墨：

```bash
cat > /tmp/fp.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<rect x="200" y="10" width="20" height="20" fill="none"/>
</svg>
EOF
svg_grid plot-grid --input /tmp/fp.svg --output /tmp/o.svg --width 200 --ncol 1 --verbose   # 修前：W6 报溢出 120 px
```

修前以下全部报 W6（`right by 120.000 px`），而两台参考渲染器在整个视口里一个墨点都没有：

| 输入 | 修前 | 修后 | d1render 墨（x≥100） | cairosvg 墨（x≥100） |
|---|---|---|---|---|
| `visible_rect`（`fill="#000"`） | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 400 |
| `fill_none` | 报 (right by 120.000 px) | — | 0 | 0 |
| `fill_none_stroke_w0` | 报 (right by 120.000 px) | — | 0 | 0 |
| `fill_transparent` | 报 (right by 120.000 px) | — | 0 | 0 |
| `fill_opacity_half` | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 400 |
| `stroke_opacity_0` | 报 (right by 120.500 px) | — | 0 | 0 |
| `stroke_opacity_half` | 报 (right by 120.500 px) | 报 (right by 120.500 px) | 156 | 160 |
| `stroke_w_tiny`（`1e-9`） | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 0 | 0 |
| `vis_hidden_attr` | 报 (right by 120.000 px) | — | 0 | 0 |
| `vis_hidden_style` | 报 (right by 120.000 px) | — | 0 | 0 |
| `g_vis_hidden` | 报 (right by 120.000 px) | — | 0 | 0 |
| `g_vis_hidden_child_visible` | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 400 |
| `g_opacity_0` | 报 (right by 120.000 px) | — | 0 | 0 |
| `g_opacity_half` | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 400 |
| `display_none` | — | — | 0 | 0 |
| `image`（真黑 1×1 PNG） | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 1580 |
| `image_opacity_0` | 报 (right by 120.000 px) | — | 0 | 0 |
| `use_visible` | 报 (right by 120.000 px) | 报 (right by 120.000 px) | 400 | 400 |
| `use_fill_none` | 报 (right by 120.000 px) | — | 0 | 0 |
| `line_stroke` | 报 (right by 120.500 px) | 报 (right by 120.500 px) | 20 | 62 |
| `line_fill_only` | 报 (right by 120.000 px) | — | 0 | 0 |

`stroke_w_tiny` 是**故意的 fail-open**：`1e-9` 在任何渲染器里都栅格化成 0 像素，但判据只问
「声明的宽度是否非零」，所以修前后都报——宁可漏报也不误报，并保持 R5 以来「非零 stroke-width
就按半笔宽外扩」的口径。

同一判据也是 `<symbol viewBox>` 实例裁剪的输入（`src/label_clip.rs:53`），所以**必须两侧都只数
真正上色的内容**，否则「只数可见内容」会让 `<use>` 掉进/掉出裁剪框。58 个 fixture 的产物与 stderr
对 base `e0c6033` 逐字节 0 差，说明可见内容的裁剪行为没变。

已上报的复合症状（`PROGRESS.md` Known Issues）：嵌套输出回灌时 W6 误报，因为内层 panel-box
`<rect ... fill="none" stroke="none" visibility="hidden">` 残留；普通话料里 6 个这类文件。

### 2. `--rel-widths` / `--rel-heights` 接受 0 权

```bash
svg_grid plot-grid --input a.svg --input b.svg --output /tmp/o.svg --width 200 --ncol 2 --rel-widths 0,1
# 修前 rc=0；--plan 里第一格是 `cell 0x60.800`，产物里两格重叠
```

| `--rel-widths` | 修前 | 修后 |
|---|---|---|
| `0,1` / `1,0` / `0.0,1` | rc=0，写出 0 宽格子的产物 | rc=2 `--rel-widths must be positive, got "0"`，不写产物 |
| `-1,1` | rc=2 `missing value for --rel-widths`（负号被当成旗标） | rc=2 `must be positive, got "-1"` |
| `1,-2` | rc=2 `must be a comma-separated list of non-negative numbers` | rc=2 `must be positive, got "-2"` |
| `0`（`--ncol 1`） | rc=2 `at least one positive`（布局兜底） | rc=2 `must be positive, got "0"`（解析期就拒） |
| `abc,1` | rc=2 `must be a number, got "abc"` | 同一句，不动 |
| `1,1` / `1e-9,1` / `1,2` / `--rel-heights 1,2` | rc=0 | rc=0 且产物逐字节不变 |

## 根因（已证）

1. `ink_bbox` 的 `Ctx` 只有 `stroke_painted`/`stroke_width`/`font_size`/`in_text`；
   `own_box` 对每个图元无条件加框，`half_stroke()` 只看 stroke。`fill`、`fill-opacity`、
   `stroke-opacity`、`visibility`、`opacity` 一个都没进判据。`display="none"` 是唯一的
   「不画」特例（`is_display_none`）。
2. `label_paint.rs::Paint` 早已实现同一套语义（`fill`/`fill_opacity`/`stroke`/`stroke_opacity`/
   `stroke_width`，含 `url(#id)` 解析到渐变/pattern、不可解析 ⇒ 不上色、`url(#id) red` 回退色上色），
   并且 `labels/letters` 的 `<text>` 判据已经在用它；ink 走的是另一套手写判据 ⇒ 两处语义分叉。
3. `parse_rel_list` 只拒 `< 0.0`（`0` 合法），负号开头的一整串又被 `take_value` 当成旗标。

## 修法

- `src/label_paint.rs`：`Paint` 增加 `pub(crate) fn fill_paints()`, `stroke_paints()`,
  `stroke_width()`（字段保持私有）；`any()` = `fill_paints() || stroke_paints()`。
  这样 ink 与 text 复用**同一份**上色判据，不再写第二份。
- `src/ink/walk.rs`：`Ctx` 改成 `{ paint: Paint, visible: bool, opaque: bool, font_size, in_text }`；
  `inherit_ctx(element, ctx, ids)` 折叠 `Paint::inherit`、`visibility`（`hidden`/`collapse` ⇒ false，
  `visible` ⇒ true，可被后代恢复；关键字**大小写敏感**，实测 d1render/resvg 如此）、
  `opacity`（`parse_number`，`<= 0` ⇒ `opaque=false`，**不可恢复**，整棵子树直接返回）；
  `paints()` = `visible && paint.any()`。`visit` 在 `inherit_ctx` 之后先看 `opaque`。
- `src/ink/shapes.rs`：closed 图元（circle/rect/polyline/polygon/path）要求 `ctx.paints()`；
  `line` 要求 `visible && paint.stroke_paints()`；`image` 要求 `visible`（<image> 没有 fill/stroke，
  其自身 `opacity` 已由 `opaque` 处理）；`text`/`tspan` 同上。`fill` 对 `line` 无效（实测 d1 0 墨）。
  `use` 的 Unsupported 分支只在 `visible` 时保留占位框。
- `src/ink/uses.rs` + `shapes.rs::carries_supported`：`<use>` 展开后引用框为空时，若目标子树里
  没有任何受支持图元 ⇒ 不上色（`Measured` 而不是 `Unsupported`），避免 `use` 指向纯 `fill="none"`
  内容时留下占位框假阳性。
- `src/cli/parse.rs`：`parse_rel_list` 对任何 `<= 0.0` 的原始 token 报
  `"{flag} must be positive, got {bad:?}"`（`got "0"` / `got "0.0"` / `got "-1"`）；
  新增 `take_rel_value` 让 `-1,1` 这种「负号开头但整串不是旗标」的值能被取到，
  `--rel-widths --width 200` 仍是 `missing value for --rel-widths`。

## 为什么这样修不弄坏相邻代码（实测）

- **语料产物**：214 个 tracked `tests/**/*.svg` 走 `convert`、58 个 `tests/fixtures/**/*.svg` 走
  `plot-grid --width 600 --ncol 1`，rc / 产物字节 / stderr 对 base `e0c6033` **0 差**（214/214、58/58）。
- **W6 文案**：214 个语料里有 6 个变化，全部是「嵌套输出回灌」的 `visibility="hidden"` panel-box
  贡献被去掉——把每个带 `visibility="hidden"` 的开始标签注入 `display="none"` 后，**修前**二进制
  打出的文案与**修后**逐字节相同（6/6 `MATCH`）。所以每一处差异 100% 是本次要修的假阳性，
  没有丢真阳性。58 个 fixture 的 W6 文案 0 差。
- **不误杀**：`fill="none"` 在祖先生效 `stroke` 下仍上色；`fill-opacity="0.5"` 仍上色；
  `stroke-width="1e-9"` 仍上色（fail-open）；`fill="transparent"` 不上色；
  `url(#missing)` 不上色、`url(#missing) red` 上色；`g opacity="0"` 下后代 `visibility="visible"`
  也救不回来（实测 d1 0 墨）；`g visibility="hidden"` 下后代 `visibility="visible"` 恢复上色。

## 证据

`docs/PITFALLS/round-data/R66/test.py`（157 checks，A/B/C/D/T 五族，默认 rc=0）：

```bash
cd <repo> && python3 docs/PITFALLS/round-data/R66/test.py                          # 157 passed, 0 failed
SVG_GRID_PIN_BIN=/tmp/r66_before_bin python3 docs/PITFALLS/round-data/R66/test.py  # 有牙：A 11 条 + B 7 条红
SVG_GRID_BIN=/nope python3 docs/PITFALLS/round-data/R66/test.py                    # rc=2（不静默 skip）
```

修前二进制（`2449f1e` 的 release 构建，sha256
`22441fa19f51ade41d2c7ee2c9e9eb43e09fc66df91a3fa9d37697d4acdff637`）：

```bash
git worktree add --detach /tmp/r66base 2449f1e
cd /tmp/r66base && cargo build --release --offline && cp target/release/svg_grid /tmp/r66_before_bin
```

变异（6 个，实跑，红名见 `meta.toml`）：`paints()` 退回只看 `visible` ⇒ 8 条红；`visibility` 分支清空
⇒ 6 条红；`opacity` 不传播 ⇒ 4 条红；`<= 0.0` 退回 `< 0.0` ⇒ 1 条红；`image` 忽略 `visible` ⇒ 1 条红；
`carries_supported` 恒 false ⇒ 1 条红。

## 不在本轮

`tspan` 视觉序、单轴 `data-scale` 配 `rotate`/`stroke` 的 `<ellipse>`、obb clipPath 语义、
`feImage` 各向异性；另：`skilltest3/Figure.svg` 残留的 24.000 px 是 clip-path 抑制的真实墨
（判据没有 clip-path 感知，R64 前就存在，原文案 79.272 px 同样不精确），只登记不改。
