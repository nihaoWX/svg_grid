# 阻塞点：`patternTransform` / `gradientTransform` 里的长度量没有跟随面板缩放

## 现象

面板被 `plot-grid` 放大时，tile box、pattern body、gradient 的 `x1/y1/x2/y2`、`cx/cy/r` 都按轴缩放了，
但同一个 user space 里的 `patternTransform` / `gradientTransform` 保持原值，于是图案/渐变相对几何错位：

- 源 panel 100×80、`data-scale="xy"`、cell 200×160（s=2），
  `<pattern patternUnits="userSpaceOnUse" x=10 y=10 width=10 height=10 patternTransform="translate(3 4)">`
  输出仍是 `translate(3 4)`；与 oracle（源图整体放大 2 倍）逐像素比：
  d1render **xor 3648 / maxdiff 255**（`docs/PITFALLS/round-data/R55/test.py` A1；其它用例见下表）。
- `gradientUnits="userSpaceOnUse"` 的 `gradientTransform="translate(3 4)"` 同理：60×40 → 240×200，
  d1render **xor 9800 / maxdiff 58**（B3）。
- 同一个错误也出现在 `patternUnits="objectBoundingBox"` 的 pattern、带 `viewBox` 的 pattern 父级、
  嵌套 pattern 的内层（每层都各自缩放，不累积），以及 `gradientUnits` 默认（obb）的 gradient 上
  —— 后者**不该**缩放，是"保持"对照组。

`src/transform/walk.rs` 的旧注释把这条语义写反了：它说 patternTransform "is already relative to the tile,
so it keeps working … If it ever needs rewriting, it must be *accumulated*, not replaced"。实测（R55）
证明：必须共轭 `P' = S·P·S⁻¹`，且嵌套时每层各缩一次，累积反而把内层缩两次（xor 14880 的旧值）。

## 复现

```bash
cd tools/svg_grid && cargo build --release --offline
python3 docs/PITFALLS/round-data/R55/test.py            # 应全绿
SVG_GRID_BIN=/tmp/r50base/target/release/svg_grid \
  python3 docs/PITFALLS/round-data/R55/test.py          # 修前二进制：30 条断言红
```

单条最小复现（A1）：

```python
panel = ('<svg width="100" height="80" xmlns="http://www.w3.org/2000/svg">'
         '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>'
         '<g data-scale="xy"><defs><pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" '
         'width="10" height="10" patternTransform="translate(3 4)">'
         '<rect x="0" y="0" width="4" height="3" fill="#000"/></pattern></defs>'
         '<rect x="10" y="10" width="80" height="60" fill="url(#p)"/></g></svg>')
# plot-grid --width 200 --height 160 --ncol 1 --plot-margin 0 ⇒ 期望 translate(6, 8)
```

oracle：把**源** panel 整体渲染到 cell 尺寸 —— 等向直接按 `s` 渲染；各向异性用
`viewBox="0 0 W H" preserveAspectRatio="none"`（等价于 `<g transform="scale(sx sy)">` 包住源内容）。
d1render 与 cairosvg 的修前/修后数字见 `round-data/R55/test.py` 末尾的四列表；cairosvg 对 pattern
有固有底噪（无 transform 时也有 xor），只用于"修后不劣于修前"的方向性交叉验证。

## 期望语义（渲染器实证，不是选择）

- `patternTransform` / `gradientTransform` 与 tile box、body、gradient 坐标同处**面板 user space**，
  必须共轭：`T' = S·T·S⁻¹`。
- 等向 `s`：线性部分不动、translate 分量乘 `s`（`rotate(a cx cy)` 的圆心同 translate）⇒ 保持关键字写法。
- 各向异性 + 纯平移：仍写 `translate(sx·tx, sy·ty)`，d1 精确 xor 0。
- 各向异性 + 旋转/斜切/非均匀缩放：共轭不再是关键字能表达的形状 ⇒ 写 `matrix(a b c d e f)`
  （SVG `matrix(a b c d e f)` = `[a c e; b d f]`）。非正交 pattern 格点无法用单个 transform 精确表达，
  残余已写明（A7 d1 1689/243、B4 d1 12/1），**不声称"已修好"**。
- `scale()` / `skew()` 在等向缩放下与 `S` 可交换 ⇒ 一个字节都不改（与修前逐字节相同）。
- `patternUnits` 默认（`objectBoundingBox`）的 tile：patternTransform 仍按 user space 共轭；
  `gradientUnits` 默认（obb）的 gradient：`gradientTransform` **保持**（由判定组 D1 用"共轭会 xor 42853"证伪）。
- 没有该属性时**不新增**属性（语料 220 文件 + 58 fixture 逐字节 0 差异的前提）。

## 四列实测（`run_old_evidence.sh R55` 同一套用例）

oracle：把**源** panel 按 cell 尺寸重渲染 —— 等向 = `d1render 源图 s`；各向异性 =
`<svg width=W·sx height=H·sy viewBox="0 0 W H" preserveAspectRatio="none">` 包住源内容。
"修前" = `/tmp/r50base/target/release/svg_grid`（`e0c6033`，sha `75def0b8…`），"修后" = 本轮二进制。
数字为 `xor / maxdiff`（RGB 三通道逐像素：像素数 / 最大通道差）。

| 用例 | d1 修前 | d1 修后 | cairo 修前 | cairo 修后 |
|---|---|---|---|---|
| A1 pattern 等向 translate(3 4) | 3648/255 | **0/0** | 3648 | 0 |
| A2 pattern 等向 rotate(30 3 4) | 3877/255 | **0/0** | 3692 | 1541 |
| A3 pattern 等向 matrix e·f | 3648/255 | **0/0** | 3648 | 0 |
| A4 pattern 等向 scale(2 3)（保持） | 0/0 | 0/0 | 1208 | 1208 |
| A5 pattern 等向 skewX(20)（保持） | 0/0 | 0/0 | 2715 | 2715 |
| A6 pattern 各向异性 translate(3 4) | 7200/255 | **0/0** | 7200 | 0 |
| A7 pattern 各向异性 rotate(30) | 8077/255 | 1689/243（残余） | 7944 | 1081 |
| A8 obb pattern translate(3 4) | 1200/255 | **0/0** | 1200 | 150 |
| A9 viewBox pattern translate(3 4) | 3648/255 | **0/0** | 3648 | 0 |
| A10 嵌套 pattern 内(2 2)/外(4 4) | 3792/255 | **0/0** | 3792 | 0 |
| A11 pattern 无 transform | 0/0 | 0/0 | 0 | 0 |
| B1 linear gradient 等向 translate(3 4) | 3680/39 | **0/0** | 3680 | 0 |
| B2 radial gradient 等向 translate(3 4) | 24340/15 | **0/0** | 24343 | 72 |
| B3 linear gradient 各向异性 translate(3 4) | 9800/58 | **0/0** | 9800 | 0 |
| B4 linear gradient 各向异性 rotate(30) | 2268/50 | 12/1（动画/AA 残余） | 2267 | 13 |
| B5 obb gradient rotate(45)（保持） | 0/0 | 0/0 | 2443 | 2443 |
| B6 gradient 无 transform | 0/0 | 0/0 | 0 | 0 |

cairo 对 pattern 有固有底噪（A11 无 transform 也是 0/0，但 A4/A5 的"保持"行修前修后同为
1208/2715 ⇒ cairo 的 pattern 光栅化本身就有相位差）；因此 cairo 只作方向性交叉：
**所有"A 修后"行都降到该族底噪或 0**，而"保持"行前后逐字节相同。
A7/B4 是非正交格点的固有残余，不写成"已修好"。
