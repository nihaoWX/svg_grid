# R43：`*Units` 的类型化解析、marker/mask 内容域、pattern tile 继承

## 现象

合成器把 `patternUnits` / `patternContentUnits` / `maskUnits` / `maskContentUnits` /
`gradientUnits` 当成"字符串比较"，而且比较时 `trim()` 掉空白、把任何"非
`userSpaceOnUse`"都当 `objectBoundingBox`。参考渲染器（`d1render` = resvg）不是这个语义：
它按 `usvg` 的 **类型化** 解析，非法值回退该属性的默认值。于是 s=2 时出现：

| 用例 | 面板 | resvg 参照 ink | 修前 ink / xor / maxdiff |
| --- | --- | --- | --- |
| `maskUnits=" userSpaceOnUse "`（非法 ⇒ bbox 区域） | 100×80 | 4800 | 2880 / 1920 / 255 |
| `maskContentUnits="foo"`（非法 ⇒ 用户空间） | 100×80 | 4800 | 600 / 4200 / 255 |
| `maskContentUnits=" objectBoundingBox "`（非法 ⇒ 用户空间） | 100×80 | 4800 | 600 / 4200 / 255 |
| `markerUnits="userSpaceOnUse"` | 100×80 | 1400 | 820 / 660 / 204 |
| 混合单位 tile（`base` 默认 bbox + `p href="#base" patternUnits="userSpaceOnUse"`） | 100×80 | 5280 | 19200 / 13920 / 204 |
| `href=" #base"`（IRI 前导空格合法） | 100×80 | 1440 | 0 / 1440 / 204 |
| `gradientUnits=" userSpaceOnUse "`（非法 ⇒ bbox 分数） | 100×80 | 32000 | 32000 / 26240 / 3 |

`href=" #base"` 在 s=1 也错（ink 0 vs 360），其余只在缩放时暴露。`s=1` 时修前 12 条用例里
只有这条红；`s=2` 时红 7 条。

`markerUnits="userSpaceOnUse"` 的 marker 被当成"和默认 `strokeWidth` 一样不需要改"：
视口 `markerWidth/markerHeight/refX/refY` 与内容都没随面板缩放，箭头被裁到旧的 12×12 框里。

`maskContentUnits` 非法值时内容被**跳过**（当成 `objectBoundingBox` 逐字保留），而渲染器回退
`userSpaceOnUse` ⇒ 内容不缩放，位置/大小全错。

pattern tile 的 `x/y/width/height` 是**逐个沿 href 链继承**的，且按**解析后的**
`patternUnits` 解释；修前按"元素自身声明的 units"决定整组，且把继承来的值当成自身值重写，
于是混合单位时把 tile 写成了三倍大。

## 复现

```bash
cd tools/svg_grid
cargo build --offline
SVG_GRID_OLD_BIN=/path/to/755ad5b/svg_grid \
  ~/.venv/bin/python docs/PITFALLS/round-data/R43/test.py     # 旧二进制 7 条为红；新树 rc=0
```

单条最小复现（s=2，面板 100×80，`<g data-scale="xy">`，`plot-grid --width 200 --height 160
--ncol 1 --plot-margin 0`，用 `d1render 源.svg out.png 2` 对照）：

```svg
<pattern id="base" x="2" y="3" width="20" height="18"><rect width="10" height="10" fill="#36c"/></pattern>
<pattern id="p" href="#base" patternUnits="userSpaceOnUse" x="10"><rect width="10" height="10" fill="#36c"/></pattern>
<rect x="10" y="10" width="80" height="60" fill="url(#p)"/>
```

## 根因

1. **单位属性不是字符串**。`usvg-0.47.0/src/parser/paint_server.rs:230-233`：

   ```rust
   pub(crate) fn convert_units(node: svgdom::Node, name: AId, def: Units) -> Units {
       let node = resolve_attr(node, name);
       node.attribute(name).unwrap_or(def)
   }
   ```

   `attribute()` 是类型化解析：只认 `"userSpaceOnUse"` / `"objectBoundingBox"` 两个精确
   token，**不 trim、区分大小写**，其余回退 `def`。默认值：`patternUnits` =
   `objectBoundingBox`（`:168`），`patternContentUnits` = `userSpaceOnUse`（`:169`），
   `gradientUnits` = `objectBoundingBox`（`:56`、`:93`），`filterUnits` =
   `objectBoundingBox`（`filter.rs:170`），`primitiveUnits` = `userSpaceOnUse`
   （`filter.rs:171`），`maskUnits` = `objectBoundingBox` / `maskContentUnits` =
   `userSpaceOnUse`（`mask.rs:20-30`，注意 mask 这两项**不**沿 href 继承），
   `markerUnits` = `strokeWidth`（`marker.rs:222`，也是字面比较）。
2. **marker 的画法**（`marker.rs:458-465`）：视口默认 `markerWidth/markerHeight = 3`、
   `refX/refY = 0`；内容变换是 `translate(vertex) → rotate → pre_scale(stroke_scale) →
   pre_translate(-refX, -refY)`，并按 `(0,0,w*stroke_scale,h*stroke_scale)` 裁剪。
   `markerUnits="userSpaceOnUse"` 时 `stroke_scale = 1`，于是视口和内容都是**用户长度**，
   面板缩放后必须一起缩放；默认 `strokeWidth` 时才靠 `stroke-width` 的改写免费拿到缩放。
3. **pattern tile 的解析**（`paint_server.rs:150-205`）：`units`/`content_units` 都经
   `convert_units`；tile 的四个数经 `resolve_number(node, AId::X|Y|Width|Height, units, …)`
   ——先沿 href 链取**第一个显式声明**，再按**解析后的** `units` 解释。所以"混合单位"时
   每个属性各按链上那处声明的 units 走，tile 的 `width` 不能因为 `p` 自己声明了
   `patternUnits="userSpaceOnUse"` 就把继承来的 `20` 当用户长度放大。
4. **IRI 词法**（`svgtypes-0.16.2/src/funciri.rs:19-29` + `Stream::parse_iri`）：先跳过
   字面空格，要求 `#`，片段吃到第一个**字面空格**为止，尾部只能是空格。所以 `" #base"`、
   `"#base "` 合法，`"#base extra"` / `"#\tbase"` 非法。修前 `namespace_ids` 只在
   `value.starts_with('#')` 时改写引用，`" #base"` 逃过改名 ⇒ 引用悬空，整块 pattern 消失。

## 修法

- 新增 `src/transform/units.rs`：`Units { UserSpaceOnUse, ObjectBoundingBox }`；
  `convert_units(Option<&str>, default)` 与 usvg 同名同义（精确 token、不 trim）；
  `units_of(element, attr, default)`；`marker_units_are_user_space`（字面比较）。
  默认值表写在模块文档里并标注 usvg 行号。
- `src/transform/mod.rs`：删掉 `user_space_units`；mask 内容只有
  `maskContentUnits` 严格等于 `objectBoundingBox` 才逐字保留，其余（含非法/缺省）缩放；
  mask/filter 区域、gradient 坐标改用 `units_of` + 各自默认值；marker 分支
  `userSpaceOnUse` 时先 `rewrite_marker_box`（`markerWidth`×X、`markerHeight`×Y、
  `refX`×X、`refY`×Y；缺省宽高按 usvg 的 `3` 物化），再把内容按 `ScalePolicy::Scale` 递归；
  默认 `strokeWidth` 时保持 R37 的早返回。
- `src/transform/patterns.rs`：`PatternSource` 记录 `x/y/width/height`；`Resolved` 的
  units 用类型化值（声明了非法值时"遮蔽"链并回退默认，与
  `resolve_pattern_attr` + `convert_units` 一致）；`rewrite_pattern_tile` 用
  `resolved_attr` 逐个取"自身优先、再沿链"，按解析后的 units 决定该属性的缩放因子；
  `internal_target` 按 svgtypes 的 IRI 语法严格解析。
- `src/lib.rs` + 新 `src/ns.rs`：`namespace_ids` 把 `href`/`xlink:href` 交给
  `namespace_iri`（保留前后空格，只改片段），`url(#id)` 交给
  `namespace_iri_references`；两处都拒绝非本文档引用。
- 为守住 `ARCHITECTURE.md` 的"单文件 ≤ 20 KB"：`src/lib.rs`（20,772 B）拆出
  `src/ns.rs`（2,743 B）；`src/transform/tests.rs`（23,342 B）拆成
  `src/transform/tests.rs` + `src/transform/tests/units.rs`。

## 验证

- `docs/PITFALLS/round-data/R43/test.py`：12 个用例 × s=1/s=2，resvg 逐像素
  （`xor = 0`、`maxdiff = 0`、两侧 `ink > 0`）：

  | 用例 | s=2 修前 ink/xor | s=2 修后 |
  | --- | --- | --- |
  | `mask_units_ws` | 2880 / 1920 | 4800 / 0 |
  | `mask_content_invalid` | 600 / 4200 | 4800 / 0 |
  | `mask_content_ws_obb` | 600 / 4200 | 4800 / 0 |
  | `marker_user` | 820 / 660 | 1400 / 0 |
  | `tile_mixed` | 19200 / 13920 | 5280 / 0 |
  | `href_leading_space` | 0 / 1440（s=1: 0 / 360） | 1440 / 0 |
  | `gradient_units_ws` | 32000 / 26240 | 32000 / 0 |

  `mask_content_strict_obb`、`marker_default`、`tile_inherited`、`href_trailing_space`、
  `href_invalid_fragment` 修前修后都绿（对照组，防过度缩放）。
- fail-closed：`SVG_GRID_OLD_BIN=/tmp/r43_base_target/release/svg_grid`（755ad5b）时，
  同一批 s=2 用例红 7 条，脚本要求 ≥5，否则 FAIL。
- 存量不动：220 条 `tests/**/*.svg` 的 `convert` 产物与 755ad5b **逐字节相同**
  （179 成功/41 双方失败、`new-only`/`base-only` 0、`changed` 0），幂等 0 问题、
  悬空引用 0（按 `id="…"` 与 `url(#…)`/`href="#…"` 交叉核对）；58 条
  `tests/fixtures/**` 的 `plot-grid --width 600` 产物 0 差异、`--plan` 0 差异
  （`mathtext.svg` 双方 rc=2）；`convert --report` 在样例上 stdout/stderr/rc 逐字节相同。
- `cargo test --offline`：221 lib + 59 main + 126 selftest = **406**，0 fail（R42 为 394）。
  `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` rc=0；`svg_grid doctor` = `57 pass / 0 fail / 1 skip`
  （`convert: in-process (svg_grid::convert module)`）。
- 旧证据 R8..R42 顶层逐个复跑全 rc=0；`pitfalls-manager verify` rc=0、`check` 静默。

## 遗留（本轮不修，已确认未恶化）

- 滤镜图元长度（`feMorphology radius` 等，xor 1136）——`primitiveUnits` 已解析但未用于图元长度。
- 嵌套 `<pattern>`（xor 11808）、`patternTransform` 长度量（1561/927）、pattern 外的
  `clip-path`（20736/6264）、pattern 内的 `<text>`（源图 ink 0）：R43 脚本 G 族对
  `nested_pattern` / `pattern_transform` 与 755ad5b 输出逐字节相同。
- `src/normalize.rs:scale_element` 仍然按固定属性表"盲缩放"（不看 units）；本轮修的是
  `transform` 路径，`--normalize-input-max-side` 路径对 `objectBoundingBox` 内容仍会二次缩放。
  这是独立引擎、独立回归面，留给下一轮。
- 混合单位 tile 的**来源判定**已实现（自身优先、沿链继承），但"继承来的 tile 数值本身是否
  该按继承处的 units 记"只按 usvg 的 `resolve_number(…, units, …)` 解释，未覆盖
  `patternUnits` 在链尾才出现且 tile 在链首的极端组合（无 fixture 命中）。
