# blocker: `<pattern>` body 被平移导致 hatch 静默消失；`<defs>` 只扫一层导致定义被误杀

- 轮次：R38（`code_before = 756dfb2`）
- 影响：任何「面板被缩放 + 图里有 `<pattern>` 填充」的图，hatch 整块消失且 **rc=0、stderr 0 字节**；
  `<defs>` 里包了一层容器的合法 SVG 被 fail-closed 门禁误杀（rc=1）。

## 现象 1 — pattern 相位（P0，静默）

`<pattern>` 的 `x/y/width/height`（tile）与 body 内几何被 **同时** 平移。resvg 0.47 与 cairosvg 2.9.1
都把 pattern 内容当 **tile 局部坐标**，body 一旦被平移就跑到自己的 tile 窗口外 ⇒ fill 整块消失，
而转换器与合成器都不报错（`rc=0`、`stderr` 0 字节）。

### 复现（1 分钟）

```bash
cd /tmp && mkdir r38rep && cd r38rep
cat > pat.svg <<'EOF'
<svg width="120" height="80" viewBox="0 0 120 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="120" height="80" fill="none" stroke="none"/>
<g data-scale="xy">
<defs><pattern id="h" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20">
<rect x="0" y="0" width="20" height="20" fill="#000000"/></pattern></defs>
<rect x="0" y="0" width="120" height="80" fill="url(#h)"/></g></svg>
EOF
/home/nihao/bioagentforge/tools/svg_grid/target/debug/svg_grid \
  plot-grid --input pat.svg --output pat.out.svg --width 148 --ncol 1 --plot-margin 24
```

修复前产物：`<pattern ... x="24" y="24" width="16.667" height="16.667">` + body
`<rect x="24" y="24" width="16.667" height="16.667">` ⇒ 源图 ink 9600 → 产物 ink **0**（两渲染器一致）。
修复后 body 为 `<rect x="0" y="0" width="16.667" height="16.667">` ⇒ 两渲染器 ink 6700、resvg diff **0**。

`objectBoundingBox` 一侧同样错：`/tmp/r38rep/patobb.svg`（默认 `patternUnits`、`<pattern width="0.25" height="0.25">`、
body 10×10、引用 `<rect x=10 y=10 width=100 height=60>`）修复前 body 被平移到 `x=24 y=24`
（源 ink 1600 → 产物 0）；修复后 body `x="0" y="0" width="8.333" height="8.333"`，
resvg 1296 / cairosvg 1260，与用户手工修过的 `patobb.fix.svg` 数字吻合。

## 现象 2 — `<defs>` 只下钻一层（fail-closed 误杀）

`convert/src/translate/uses.rs` 的 `visit_defs` 只访问 `<defs>` 的**直接子元素**，而 `prune_defs_with`
是递归剪枝 ⇒ 嵌在 `<g>` / `<symbol>` / 嵌套 `<defs>` 里的定义从未被扫描、被丢，随后
`check_dangling_references` 报：

```
cannot translate the input SVG to the tag protocol: 1 reference(s) point at an id that
nothing in the input defines: <rect> fill="url(#g)" -> #g; add the missing definition …
```

### 复现（7 种嵌套形态）

```bash
cd /tmp/r38probe/defs   # d1_flat d2_g d3_g_id d4_g_defs d5_defs_defs d6_symbol d7_g_mask
for f in d*.svg; do /home/nihao/bioagentforge/tools/svg_grid/convert/target/release/svg_grid_convert \
  --input "$f" --output "/tmp/$f.out" >/dev/null 2>&1; echo "$f rc=$?"; done
```

修复前：`d2_g` / `d3_g_id` / `d5_defs_defs` / `d6_symbol` / `d7_g_mask` 全 **rc=1**；修复后 **7/7 rc=0**，
且产物里 `<linearGradient id="g">` / `<mask>` 都在（`<symbol>` 被解包、定义提升到 `<defs>` 层），
渲染对照 resvg `xor=0 max=0 ink=6400`、cairosvg `xor=0 max=0 ink=6400`。

## 根因

1. **pattern**：composer 把 `<pattern>` 的 tile 与 body 都交给同一套 `map_point`（含平移）。
   body 的数字是 **tile 局部**坐标，正确映射是「按 sx/sy 缩放、不平移」。
2. **defs**：`visit_defs` 是 R37 新门禁的扫描器，只认直接子元素；`prune_defs_with` 却递归。
   两者遍历不一致 ⇒ 定义被剪、门禁报悬空。另外 `<defs><g><linearGradient>` 里 `<g>` 本身
   无 id、不在 `REFERENCE_TARGETS` 中，即使扫到了也会被连容器一起丢掉。
3. **symbol**：`convert/src/scan.rs` 无条件拒绝 `<symbol>`（自 first commit 起）。`<defs>` 里的
   `<symbol>` 不绘制，未引用时应当允许（由剪枝丢弃）；被 `<use>` 引用时 `expand_uses` 会把它
   拉进可绘制树，`self_check` 里的第二次 `scan` 仍会拒绝 ⇒ 收窄规则不丢 fail-closed。
4. **证据脚本**：`round-data/R37/test.py` 缺渲染器时 `skip()`，但仍打印「判据全部成立」并 `rc=0`。

## 修法（代码位置见 `meta.toml` / 账本 R38-F1..F7）

- `src/transform/mod.rs`：`ScalePolicy` 新增 `Scale`（只缩放不平移，仅 composer 对 pattern body 使用，
  `data-scale` 永不产生它）；`transform_element` 对 `<pattern>` 的子树统一用 `Scale` 递归（两种
  `patternContentUnits` 都这样，**不统一 obb 语义** —— 两渲染器本身不一致）。tile 本身仍走 `rewrite_rect`。
- `src/transform/lengths.rs` / `primitives.rs` / `paths.rs`：补 `Scale` 臂（`map_point` 为 `(x*sx, y*sy)`、
  `length_scale_factor` 与 `Xy` 相同、`rewrite_rect` 走缩放分支、`path_axis_map` 平移分量为 0）。
- 不写 `patternTransform`（语料 0 处使用）：既有值原样保留。若将来要写，**必须累积**而不是覆盖。
- `convert/src/translate/uses.rs`：`visit_defs` 递归到 `<defs>` 子树任意深度（`visit_defs_contents`）；
  新增 `keep_definitions`，在剪枝时把 wrapper（`g`/`symbol`/`defs`/`a`/`switch`）**解包**、把里面的
  定义提升到 `<defs>` 层，空容器丢弃。
- `convert/src/scan.rs`：`walk_scan` 增加 `in_defs` 上下文，`<symbol>` 只在**可绘制树**里拒绝。
- `docs/PITFALLS/round-data/R37/test.py`：支持 `SVG_GRID_BIN` / `SVG_GRID_CONVERT_BIN` /
  `SVG_GRID_RENDER_BIN`；渲染器缺失 ⇒ 渲染对照类判据 **FAIL**（除非显式 `--allow-skip`）。

## 双渲染器实测（resvg 0.47 = d1render；cairosvg 2.9.1，`background_color="white"`）

| 用例 | 修复前 | 修复后 |
|---|---|---|
| pat.svg（userSpaceOnUse, s=0.8333） | 源 ink 9600 → 产物 **0**（两渲染器） | resvg diff **0**（ink 6700/6700）；cairosvg diff 715（仅 tile 边界抗锯齿，ink 6700/6700） |
| us_src.svg（非对称 body 14,6,12,8） | diff 144 / ink 0 | resvg diff **0**（ink 288）；cairosvg diff 288（抗锯齿，ink 288/378） |
| patobb（objectBoundingBox 内容） | body 平移到 24,24 ⇒ ink 0 | body `0,0,8.333,8.333` ⇒ resvg diff **0**（ink 1296）、cairosvg 1260 |
| bars_grouped（真实 matplotlib 语料） | resvg diff 5589（maxdiff 255），4 根 hatch 柱 2 根整块缺失 | resvg diff **442**（maxdiff **7**，仅 hatch 边缘），ink 15878/15878 完全相等；cairosvg 9686→7224 |
| S36_panelA_matrix（真实语料） | resvg diff 448（maxdiff 132） | resvg diff **357**（maxdiff **2**），ink 28223/28223 相等；cairosvg 25398→25409 |

S36 的 scale 按 pt→px 换算：`s = 240/(533.76*4/3) = 0.3372`；bars_grouped `s = 240/288 = 0.8333`。
`plot-grid --width 288 --plot-margin 24` 自身 PNG 的 nonwhite 计数：bars_grouped base 13651 → 16195、
S36 base 31123 → 31153（这两组与「源图按 s 渲染」不是同一口径，源图 resvg ink 在 s=1.0 时
bars_grouped = 22190、S36 = 230845；差异属口径，不是缺陷）。

## 回归

- 根 crate：89 lib + 59 main + 108 selftest = **256** 全绿；convert：104 lib + 14 CLI = **118** 全绿。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 两个 crate 均过。
- 220 fixture 逐个 convert：与 base `756dfb2` 对比 `newfail=0`；pattern 相关产物只有 2 个文件变化
  （`tests/fixtures/matplotlib/bars_grouped.svg` 与 S36 语料），渲染 diff 见上表。
- 旧证据 R3–R37 逐个顶层复跑全绿（嵌套回归用 `/tmp/shim` 短路，只对已在顶层跑绿的轮次放行）。

## 非阻断遗留

1. `objectiveBoundingBox` 内容的语义两渲染器本身不一致（源图 resvg vs cairosvg diff 800），本轮按
   用户裁决**不统一**，只保证 body 不平移。若要统一需要先裁决「谁是标准」。
2. cairosvg 的 hatch 相位与 resvg 有系统性差异（源图就存在），bars_grouped/S36 的 cairosvg diff
   没有随本轮修复归零。这是渲染器差异，不是 composer 缺陷。
3. R18 的 `install.sh --dry-run` 断言在 base 上就已 FAIL（`4133c83` 起还会装 `general-figure-guide`
   skill，`would install` 变 2 行）。本轮改为精确计数并落 `modify R18 --by R38`。
4. 语料里 0 处 `patternTransform`；本轮不写它。若未来有真实用例，需按「累积」实现并补判据。
