# svg_grid 阻塞点 — `--normalize-input-max-side` 把 objectBoundingBox 的 pattern 分数再缩一次

场景：`tools/svg_grid` HEAD `3938656`（2026-11-24，R44 之后），`svg_grid plot-grid`
（`./install.sh` 装到 `~/.local/bin/svg_grid`）。

## 现象

`--normalize-input-max-side` 走的是 `src/normalize.rs:scale_element`（35-77），它按一张**固定属性表**
盲缩放 `x/y/width/height/...`，不区分这些属性是用户空间长度还是 `objectBoundingBox` 分数。
于是 `patternUnits="objectBoundingBox"` 的 tile 尺寸与 `patternContentUnits="objectBoundingBox"`
的内容尺寸都被**再乘一次**归一化因子：

```bash
# 输入：tile 0.25×0.25（bbox 分数）、内容 0.5×0.5（bbox 分数）、引用 rect 80×60
svg_grid plot-grid --input obb3.svg --width 400 --ncol 1 --plot-margin 0 --output a3.svg
#   <pattern width="0.25" height="0.25" patternContentUnits="objectBoundingBox">
#     <rect width="0.5" height="0.5" fill="#000"/>          ← 正确
svg_grid plot-grid --input obb3.svg --width 400 --ncol 1 --plot-margin 0 \
  --normalize-input-max-side 50 --output b3.svg
#   <pattern width="0.125" height="0.125" patternContentUnits="objectBoundingBox">
#     <rect width="0.250" height="0.250" fill="#000"/>      ← 分数被减半，hatch 变稀
```

渲染（resvg，`/tmp/d1_eval/render/target/release/d1render`，scale 1）暗像素：

| 文件 | 暗像素 |
|---|---|
| 源 `obb3.svg` | **4800** |
| 不归一化 `a3.svg` | **4800**（正确） |
| 归一化 `b3.svg` | **1200**（**只剩 1/4**） |

即：**归一化路径静默丢掉 75% 的 hatch 墨量**，rc=0、stderr 0 字节。

## 复现步骤

```bash
mkdir -p /tmp/normprobe && cd /tmp/normprobe
cat > obb3.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>
  <defs><pattern id="h" width="0.25" height="0.25" patternContentUnits="objectBoundingBox">
    <rect x="0" y="0" width="0.5" height="0.5" fill="#000"/>
  </pattern></defs>
  <rect x="10" y="10" width="80" height="60" fill="url(#h)"/>
</svg>
EOF

svg_grid plot-grid --input obb3.svg --width 400 --ncol 1 --plot-margin 0 --output a3.svg
svg_grid plot-grid --input obb3.svg --width 400 --ncol 1 --plot-margin 0 \
  --normalize-input-max-side 50 --output b3.svg

grep -o '<pattern[^>]*>[^<]*<rect[^>]*>' a3.svg   # width="0.25" ... width="0.5"
grep -o '<pattern[^>]*>[^<]*<rect[^>]*>' b3.svg   # width="0.125" ... width="0.250"  ← 错
```

## 影响面

- 只影响**显式给了 `--normalize-input-max-side`** 的调用；不给该参数时走 transform 路径，
  R38–R43 已修好（`src/transform/patterns.rs` 的 `pattern_body_is_verbatim`）。
- 触发条件：输入里有 `patternUnits="objectBoundingBox"`（**默认值**，所以只要 pattern 没写
  `patternUnits="userSpaceOnUse"` 就中招）或 `patternContentUnits="objectBoundingBox"`。
- 语料：`tests/fixtures/**` 里 pattern 只有 1 处且是 `userSpaceOnUse`，所以 220 fixture 抓不到；
  `research/**` 3 个文件也都是 `userSpaceOnUse`。但 matplotlib 的 hatch 默认就是
  `objectBoundingBox`（`tests/fixtures/matplotlib/bars_grouped.svg` 显式写了 `userSpaceOnUse`，
  是少数派写法）。
- 同一张属性表还盲缩放 `markerWidth/markerHeight/refX/refY`（`markerUnits="strokeWidth"` 时
  这些是 stroke-width 的倍数，不是用户空间长度）与 `mask`/`filter` 的 `x/y/width/height`
  （`*Units="objectBoundingBox"` 时是分数）——**同一类缺陷，只是 pattern 最容易看到**。

## 建议

1. `scale_element` 不能按属性名盲缩放，要按**元素 + 单位属性**分流，与 `src/transform/units.rs`
   的 `convert_units` 用同一套判据（R43 已建立该模块，normalize 路径是第二份实现）。
2. 具体：`pattern` 的 `x/y/width/height` 只在 `patternUnits="userSpaceOnUse"` 时缩放；
   pattern 子树只在 `patternContentUnits="userSpaceOnUse"`（默认）时缩放；
   `marker` 的 `markerWidth/Height/refX/refY` 只在 `markerUnits="userSpaceOnUse"` 时缩放；
   `mask`/`filter` 的 box 只在对应 `*Units="userSpaceOnUse"` 时缩放。
3. 更稳的做法：让 normalize 复用 transform 的 `pattern_body_is_verbatim` / `units_of`，
   而不是维护第二张属性表（`ARCHITECTURE/tag-protocol.md` 已登记"path 双实现防漂移"，
   这是第三对）。
4. 补测试：`--normalize-input-max-side` 下 obb pattern 的产物渲染与源图逐像素一致
   （不能只断 markup——本轮就是 markup 看着"合理"、渲染差 4 倍）。
