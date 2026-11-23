# svg_grid 阻塞点 R37 — `convert` 的 `<defs>` 必须按引用保留，悬空引用必须硬错

## 现象

`docs/PITFALLS/blocker-convert-defs-mask-dropped.md` 报的静默损坏：`convert` 的 `prune_defs`
按**元素名白名单**保留 `<defs>` 子项（`REFERENCE_TARGETS = ["clipPath","style","linearGradient",
"radialGradient","pattern"]`），`<mask>`/`<marker>`/`<filter>` 不在名单里 ⇒ 整块删掉，而引用它的
`mask="url(#m)"` 原样保留 ⇒ 引用悬空、遮罩静默失效、**rc=0、stderr 0 字节**。

- 复现输入（40×20 矩形，遮罩只留左半）源图黑墨 **6,396 px**（400×240 画布），转换后 **12,800 px**
  （= 整个矩形，遮罩完全没生效）。
- 同一机制影响 `<marker>`（箭头消失）与 `<filter>`（滤镜整个不画：resvg 源 15,616 → 产物 6,400）。
- 产物里已无 `<mask>` 节点，所以 `self_check` 的"未翻译方言节点"检查也不会触发。

## 复现步骤

```bash
cd tools/svg_grid && cargo build
mkdir -p /tmp/r37r && cd /tmp/r37r
cat > in.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60" viewBox="0 0 100 60">
  <defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="60">
    <rect x="10" y="10" width="20" height="20" fill="white"/></mask></defs>
  <rect x="10" y="10" width="40" height="20" fill="black" mask="url(#m)"/>
</svg>
EOF
svg_grid convert --input in.svg --output out.svg; echo "rc=$?"   # 修前 rc=0，out.svg 里没有 <mask>
```

或直接 `python3 docs/PITFALLS/round-data/R37/test.py`（① 段就是四组探针）。

## 根因

三处，同一件事的三个面：

1. **保留判据错**（`convert/src/translate/uses.rs:192-215`）：`prune_defs` 用
   `REFERENCE_TARGETS.contains(&child.name.as_str())` 决定去留——按名字而不是按引用。名字白名单
   天然无法覆盖 `<mask>`/`<marker>`/`<filter>`，也无法覆盖"被 `<pattern>` 间接引用的
   `<linearGradient>`"这类传递关系。
2. **悬空引用不报错**：剪完之后没有任何阶段检查"引用还在不在"。`bake::apply_clip_refs` 只处理
   **被变换上下文**引用的 `clip-path`，未变换形状上的悬空 `clip-path` 直接漏过（实测 rc=0）。
3. **内容不翻译**（`convert/src/translate/bake.rs:37-133`）：`bake_element` 的 match 只有
   `defs`/`pattern` 用 `Affine::IDENTITY` 递归，`mask`/`marker` 落到 `_ =>` 原样 push ⇒ 里面的
   `<path>` 保持未翻译形态。`self_check` 的 `check_residue` 因此把 mask/marker 整棵子树排除
   （R28 的 `in_mask_or_marker`），这是"因为不翻译所以放行"的权宜之计。

## 修法

### ① 按引用保留（`convert/src/translate/uses.rs:192-405`）

- `prune_defs`（:192-215）：先 `collect_references_outside_defs` 收**`<defs>` 之外**的引用，
  再 `loop { visit_defs(...) }` 做传递闭包（只对已被引用的定义收引用，`grown` 为假即停），
  最后 `prune_defs_with`（:217-240）按 `referenced` 集合保留。
- 引用识别：`url_ids`（:298-325）扫 `url(`…`)`，支持 `url( #id )` 与 `url('#id')`；
  `href_id`（:334-340）只认 `href`/`xlink:href`，且 `element.name == "a"` 时返回 `None`
  （`<a href="#…">` 是超链接，不是渲染引用）。
- `REFERENCE_TARGETS` 五类与 `<style>` 仍**无条件**保留（:177），未引用的 mask/marker/filter
  仍会被丢——白名单从"唯一判据"降级为"额外保险"。

### ② 悬空引用 fail-closed（`convert/src/translate/uses.rs:356-405`）

`check_dangling_references` 在 `prune_defs` 之后、`bake` 之前调用
（`convert/src/translate/mod.rs:64-68`）：收全树 id，遍历所有属性的 `url(#…)` 与 `href="#…"`，
缺失即 `ConvertError::Translate`：

```
{n} reference(s) point at an id that nothing in the input defines: <rect> mask="url(#nope)" -> #nope; add the missing definition or remove the reference
```

`clip-path` 也在此查**存在性**（是否为真 `<clipPath>`、能否烘焙仍归 `apply_clip_refs`），
否则未变换形状上的悬空 clip 会漏过。`--force` 不放行（force 只影响幂等判定）。

### ③ mask/marker 内容翻译（`convert/src/translate/bake.rs:80-93`）

```rust
"mask" | "marker" => {
    bake_children(&mut element.children, Affine::IDENTITY, clip_refs)?;
    out.push(Node::Element(element.clone()));
}
```

累加器重置：mask 在被遮元素的用户空间求值，marker 内容在自身坐标系（由 `markerUnits` 缩放）。
`<filter>` **故意不递归**——滤镜原语不是形状，里面的 `<path>` 是真残渣。

`self_check` 的 `in_mask_or_marker` 参数随之删除（`convert/src/self_check.rs:57-88`）：
`"path" if is_flattened_fill_path(element) => {}`，不再按容器一刀切。R28 的
`self_check_rejects_flattened_paths_inside_masks_and_markers` 改名为
`self_check_accepts_flattened_paths_inside_masks_and_markers`，并新增
`self_check_rejects_a_path_inside_a_filter` 把边界钉在 filter 上。

### ④ 合成期按单位缩放（`src/transform/mod.rs:56-129`、`:340-356`）

- `transform_element` 新增三个早返回：`marker` 一律不重写（:97）、`mask` 的
  `maskContentUnits != "userSpaceOnUse"` 不重写（:105）、`filter` 不重写（:118）。
- `rewrite_primitive` 新增：`"mask" if user_space_units(element, "maskUnits") => rewrite_rect(...)`
  （:346）、`"filter" if user_space_units(element, "filterUnits") => rewrite_rect(...)`（:352）。

**marker 为什么两种单位都不重写**（这是本轮唯一一处"看起来该缩放却没缩放"的地方，理由写进了
代码注释）：渲染器把 marker 平移到顶点、按 `orient` 旋转、再平移 `-refX,-refY`，body 被裁剪到
`(0,0,markerWidth,markerHeight)` 视口 ⇒ body 的数字是**相对参考点**的。平移 body 会把它移出
自己的视口，箭头整个消失（实测 ink 184 → 160，`xor=264 maxdiff=128`）。默认
`markerUnits="strokeWidth"` 下渲染器按 stroke-width 缩放 body，而 `stroke-width` 本身会被重写，
箭头自然随面板缩放。`userSpaceOnUse` 下 body 与视口都不动，也正确。

## 验证

### 四组探针（`plot-grid --ncol 1 --width 148`，s=1；resvg 渲染 + ink bbox 平移对齐）

| 探针 | ink 源→产物 | shift | XOR | maxdiff |
| --- | --- | --- | --- | --- |
| mask `maskUnits="userSpaceOnUse"` | 400 → 400 | (24,24) | **0** | 0 |
| mask 默认 `objectBoundingBox` | 400 → 400 | (24,24) | **0** | 0 |
| marker 默认 `strokeWidth` | 184 → 184 | (24,24) | **0** | 0 |
| marker `markerUnits="userSpaceOnUse"` | 184 → 184 | (24,24) | **0** | 0 |
| marker s=2（`--width 248`） | 404 → 404 | (24,24) | **0** | 0 |
| filter（`feMorphology dilate 4`） | 976 → 976 | (24,24) | **0** | 0 |
| pattern→gradient（convert-only） | 1600 → 1600 | (0,0) | **0** | 0 |

单位矩阵（6 个定义、s=2）：`maskUnits=userSpaceOnUse` 的 box 与 body 都缩放；默认
`objectBoundingBox` 的 `x/y/width/height` 保持 `0/0/0.5/1` 而 body 平移；`markerUnits` 两种取值
下 marker 属性与 body 全不变；`filterUnits=userSpaceOnUse` 的 box 缩放、默认保持分数。

### 220 个 `tests/**/*.svg` 逐字节回归

基线 `/tmp/r37/base` = `git worktree add /tmp/r37/base e39b3e9`：

```
total=220 same=179 both_fail=41 newfail=0 diffs=0
```

41 个两边都失败的是既有不兼容 fixture（`<text>` 旋转等），**没有新增失败、没有字节差异**。
另外每个成功产物都幂等（再 convert 得 `AlreadyConverted`），且产物内无悬空
`url(#…)`/`href="#…"`（0 条）。

### 真实语料 `S36_panelA_matrix.svg`（hatch pattern，104,788 B）

- convert rc=0，`<pattern id="h9f23a50880" patternUnits="userSpaceOnUse" x="0" y="0" width="72"
  height="72">` 保留；convert 产物与基线**逐字节相同**。
- 源 vs convert-only（同 scale 1.0）：`xor=39 maxdiff=3 ink 230845→230845 shift=(0,0)`（纯抗锯齿）。
- 源 vs grid（源 `width="533.76pt"` ⇒ 1pt=4/3px，故源 scale 0.75 / 产物 1.0）：
  `ink 132084→131854 shift=(24,24) xor=3169 maxdiff=132`，其中 323 px 是大幅差（186 px 源
  `(201,123,166)` 粉 → 产物白，bbox `(439,105)-(465,117)`），其余 2182 px 是抗锯齿。

## 未覆盖情形（交父裁决）

### pattern 相位错位 —— R16 时代遗留，**不是本轮引入**

上面 S36 的 3169 px 残差全部落在 hatch 区域，根因是 pattern 的**相位**：

- resvg 把 pattern 内容渲染进 `rect.width×rect.height` 的 pixmap
  （`resvg-0.47.0/src/path.rs:179-205 render_pattern_pixmap`：`Transform::from_scale(sx,sy)` 渲染
  `pattern.root`，再 `ts.pre_translate(rect.x(), rect.y())` 平铺）⇒ **内容坐标是 tile 局部坐标，
  原点 = tile 左上角**。
- 而 `src/transform/mod.rs` 对 `patternUnits="userSpaceOnUse"` 的 pattern 会递归重写 body
  （tile 平移 + body 也平移）⇒ 相位错 `24 mod tile`。

实测矩阵（`/tmp/r37/q0.svg` 等）：`tile=(0,0) content=(0,0)` ⇒ 粉块在 x=0..9；
`tile=(24,24) content=(0,0)` ⇒ x=24..33；`tile=(24,24) content=(24,24)` ⇒ x=8..17（24+24 mod 40）。

最小复现 `/tmp/r37/pat.svg`（120×80，tile 20×20 全填 `#c97ba6` + 一条对角线）：
grid 后 tile `x=24 y=24 w=12 h=12`、body `<rect x="24" y="24" width="12" height="12">`，
渲染 `ink_src=4000 ink_out=0`（整块 hatch 消失）。

`tests/fixtures/matplotlib/bars_grouped.svg`（唯一含 `<pattern>` 的 fixture）：
`plot-grid --width 400 --ncol 1` 后 tile `x=24 y=24 w=117.333`、body `<rect x=24 y=24 w=118.963>`；
源 vs grid `ink 22190→27584 xor=38278 maxdiff=255`。

**为什么不在本轮修**：`bg.base.svg`（基线 convert）与 `bg.new.svg`（新 convert）逐字节相同，
基线 binary 与新 binary 的 grid 产物也逐字节相同 ⇒ R37 没有改变任何 pattern 渲染。按派单
"若改动让某 fixture 渲染变化，停下在报告列清楚"的口径，这里单列交裁决。正确做法应是
**tile 平移、body 只缩放不平移**（或 body 完全不动），但这会改
`tests/selftest/transform/mod.rs:386 paint_server_coordinates_follow_a_user_space_reference` 与
`convert/src/translate/mod.rs:225 pattern_body_is_translated_like_any_other_geometry` 两个既有
测试的断言，属于独立一轮的范围。

### 其它已知边界

- `<filter>` 内容不翻译：滤镜原语里的 `<path>` 会被 `self_check` 判残渣（rc=1）。这是**故意**
  的 fail-closed，不是遗漏。
- 重复 id：`collect_defined_ids` 用 `HashSet`，两个同名定义都保留、不报错（SVG 本身不允许，
  但报错会误伤真实语料）。
- 自引用 pattern（`fill="url(#p)"` 在 `#p` 内部）：传递闭包用 `grown` 判停，不死循环。

## 测试与回归

- 新增 `convert/src/translate/defs_tests.rs`（13 个测试）：引用保留/未引用丢弃、传递闭包、
  `href="#id"`、悬空 mask/clip-path/href 硬错、`<a href>` 不算引用、`url( #id )` 带空格、
  自引用不死循环、重复 id、mask/marker 内容翻译、filter 内容留残渣。
- 新增 `tests/selftest/defs/mod.rs`（7 个测试）：mask 两种 `maskUnits`、mask 的
  `maskContentUnits`、marker 两种 `markerUnits`、filter 两种 `filterUnits`、未引用定义仍被丢。
- `convert/tests/cli.rs` +2：悬空引用不写产物（`--force` 也不放行）、used/unused defs。
- 计数：根 crate 89 lib + 59 main + **104** selftest（原 97）；convert **103** lib（原 88）
  + **14** cli（原 12）。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 两 crate 干净；
  `./install.sh` rc=0；`svg_grid doctor` **57 pass / 0 fail / 1 skip**（SKIP 仍是
  `matplotlib/mathtext` 旋转文本）。
- 旧证据：`round-data/{R8,R9,R10,R13,R14,R29..R36}/test.py` 全 rc=0。**R28 需要适配**：
  它的 ① 段判"mask 内与展平产物同形的 `<path>` 仍判残渣"，而 R37 起 mask 内容会被翻译 ⇒
  该判据已被取代。已把 ① 段改成"`<filter>` 内未翻译的 `<path>` 仍判残渣"（新 fixture
  `round-data/R28/data/filter_path.svg`，删掉 `mask_path.svg`），R28 现 3 pass / 0 fail。
- 文件体积（R8 上限 20000 B）：`convert/src/translate/uses.rs` 14066、
  `convert/src/translate/mod.rs` 16472、`convert/src/translate/bake.rs` 13273、
  `convert/src/self_check.rs` 12009、`convert/src/translate/defs_tests.rs` 11247、
  `src/transform/mod.rs` 13042、`convert/tests/cli.rs` 17503、`tests/selftest/defs/mod.rs` 7911、
  `docs/PITFALLS/round-data/R37/test.py` 18762；`docs/general-figure-guide/references/02a_svg_grid_assembly.md`
  19972 B 未动。
