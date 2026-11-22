# svg_grid 阻塞点 — `convert` 丢弃 `<defs>` 内的 `<mask>`/`<marker>`，遮罩静默失效且 rc=0

场景：`tools/svg_grid` HEAD `64e4551`（2026-11-22，R27/R28 之后），`svg_grid convert`（release 构建，
`~/.local/bin/svg_grid`），输入为手写 / Illustrator 常见的 `<defs><mask>` 写法。渲染计数用
cairosvg 2.9.1（`~/.venv`），4× 缩放、白底、亮度 < 128 记黑。

## 现象

`<defs>` 里的 `<mask>` 被整块删掉，而引用它的 `mask="url(#m)"` 原样保留 ⇒ 引用悬空，遮罩静默失效：
本该被遮掉的区域照常画出来，**退出码 0、stderr 0 字节、无任何告警**。

- 复现输入（矩形 40×20，遮罩只留左半 20×20）在源图渲染黑墨 **6,396 px**（400×240 画布）；
  转换后产物渲染 **12,800 px**（= 整个 40×20 矩形，遮罩完全没生效）。
- 产物里 `<defs>` 与 `<mask id="m">` 全部消失，只剩 1 处 `mask` 字样——即 `<rect ... mask="url(#m)"/>`
  上的悬空引用。
- 同一机制影响 `<marker>`：箭头定义被删、`marker-end` 引用悬空（箭头消失）。

`convert` 的契约（`docs/HANDOFF.md`、`INSTRUCTIONS.md`）要求"convert 必须自动且不得损坏原图"，
本项属于**静默损坏**，且因为产物已无 `<mask>` 节点，`self_check` 的"未翻译方言节点"检查也不会触发。

## 复现步骤

```bash
mkdir -p /tmp/defs_mask && cd /tmp/defs_mask
cat > in.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60" viewBox="0 0 100 60">
  <defs>
    <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="100" height="60">
      <rect x="10" y="10" width="20" height="20" fill="white"/>
    </mask>
  </defs>
  <rect x="10" y="10" width="40" height="20" fill="black" mask="url(#m)"/>
</svg>
EOF

svg_grid convert --input in.svg --output out.svg
echo "rc=$?"                 # rc=0
cat out.svg                  # <defs>/<mask> 消失，剩 <rect ... mask="url(#m)"/>
```

计数（cairosvg 2.9.1，4×、白底、<128 记黑）：

```bash
~/.venv/bin/python - <<'PY'
import cairosvg, io
from PIL import Image
def dark(path, scale=4):
    png = cairosvg.svg2png(url=path, scale=scale, background_color='white')
    im = Image.open(io.BytesIO(png)).convert('L')
    return sum(1 for p in im.getdata() if p < 128)
print('src', dark('in.svg'))   # 6396
print('out', dark('out.svg'))  # 12800
PY
```

实测：`src 6396 / out 12800`，即遮罩右半 6,400 px 的差异被静默丢失。

`<marker>` 变体（同样实测）：`<defs><marker id="a">…</marker></defs>` +
`<path d="M 10,30 L 90,30" stroke="black" marker-end="url(#a)"/>` ⇒ convert rc=0，产物里 `<defs>` 整块消失、
`marker-end="url(#a)"` 悬空；cairosvg 渲染该产物直接抛
`AttributeError: 'NoneType' object has no attribute 'get'`（`path.py:54 draw_markers`），即箭头丢失且下游渲染器可能崩。

## 追加实测（2026-11-23，D5 之后；`prune_defs` 之外还有两层）

把 `<mask>`/`<marker>` 手工留在产物里（绕过 `prune_defs`）再走 `plot-grid`，暴露两个**独立的**
合成期缺陷——所以修法不能只是往 `REFERENCE_TARGETS` 里加两个名字。

### A. `<mask maskUnits="userSpaceOnUse">` 的区域不跟着缩放

输入（100×60 画布，遮罩区域 50×30 只留左上四分之一）：

```xml
<defs><mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="50" height="30">
  <rect x="0" y="0" width="50" height="30" fill="white"/></mask></defs>
<rect x="0" y="0" width="100" height="60" fill="black" mask="url(#m)"/>
```

`plot-grid --width 400 --ncol 1` 后：内容被搬到 `x=24 y=24`，但 `<mask>` 的
`x/y/width/height` 仍是 `0/0/50/30` ⇒ 遮罩区域与图形错位，黑墨 **23,996 → 2,492 px**（几乎全被遮掉）。
`src/transform/mod.rs` 的 `rewrite_primitive` 只处理 `linearGradient`/`radialGradient`/`pattern`，
没有 `mask` 分支；`maskUnits` 默认是 `objectBoundingBox`（分数），只有 `userSpaceOnUse` 才该缩放。

### B. `<marker>` 的内容被当成用户空间坐标平移

`markerUnits` 默认 `strokeWidth`：marker 内容坐标的单位是**引用元素的 stroke-width**，不是用户空间。
`src/transform/mod.rs:83-88` 无条件递归所有子节点 ⇒ marker 内容被加上格偏移：

- 输入 `M 0,0 L 6,3 L 0,6`（`markerUnits` 默认）⇒ 产物 `M 24 24 L 30 27 L 24 30`（多了 24,24）。
- 黑墨 3,520 → 3,712 px（箭头相对顶点偏移）。
- `markerUnits="userSpaceOnUse"` 时才该跟着变换。

### C. `convert` 的 `self_check` 会拒绝保留下来的 mask/marker 内容

`convert/src/self_check.rs:49-96` 的 `in_mask_or_marker` 把 mask/marker 内的 `<path>` 一律判残渣
（R28 的 deliberate fail-closed），而 `convert/src/translate/bake.rs:70-82` 只对 `defs`/`pattern`
递归、对 `mask`/`marker` 走 `_ =>` 原样保留（不翻译）。⇒ 一旦 `prune_defs` 开始保留带 `<path>` 的
mask/marker，`convert` 会 rc=1。两者必须一起改：`bake` 对 mask/marker 也按 `Affine::IDENTITY` 递归，
`self_check` 的 mask/marker 例外收窄到"非 canonical 展平 path"。

### 结论（给 R34 的口径）

1. `prune_defs` 改为**按引用保留**（`url(#id)` / `href="#id"` / `mask=` / `marker-*=` / `clip-path=` /
   `fill=` / `stroke=` 里出现的 id），`<style>` 恒留；未被引用的仍丢弃。
2. 剪完之后**悬空引用 fail-closed**（rc≠0、点名属性与 id），不再静默丢遮罩/箭头。
3. 合成期按单位处理：`mask` 的 `x/y/width/height` 只在 `maskUnits="userSpaceOnUse"` 时缩放，
   内容只在 `maskContentUnits="userSpaceOnUse"`（默认）时缩放；`marker` 的内容与
   `markerWidth/Height/refX/refY` 只在 `markerUnits="userSpaceOnUse"` 时缩放，默认 `strokeWidth` 一律不动。
4. `convert` 侧 `bake` 递归进 mask/marker（IDENTITY），`self_check` 相应收窄。

## 追加实测（2026-11-23，R35 派单前；`<filter>` 同样被丢，且后果更重）

`REFERENCE_TARGETS` 是**按元素名**的白名单，所以同一机制还命中 `<filter>`：`<defs><filter id="f">`
被整块删掉，`filter="url(#f)"` 悬空。

- resvg 对悬空 `filter` 引用的处理是**整个图形不画**（不是"忽略滤镜照常画"）：
  `/tmp/r35probe/filt8.svg`（一个 dilate 滤镜矩形 + 一个普通矩形）源图黑墨 **15,616 px**，
  `convert` 产物 **6,400 px** —— 带滤镜的那个矩形**完全消失**。
- 把 `<filter>` 手工塞回产物（`filt6.restored.svg`）⇒ 黑墨回到 **12,321 px**，与源图一致，
  证明差异就是被删的 defs。
- cairosvg 忽略滤镜，所以这条在 cairosvg 下看不出来（两个文件都 6,400 px）——**渲染器相关**，
  但 resvg 是真实下游，属于静默丢墨。
- `plot-grid` 的 W9 能抓到（`--verbose` 下 `input #0 <rect filter> -> url(#f)`），但 `convert`
  自己 rc=0、stderr 0 字节，且 W9 默认静音。

⇒ R35 的修法**不能**是往 `REFERENCE_TARGETS` 里补名字（补了 `filter` 还会有下一个）：
按"文档里被 `url(#id)` / `href="#id"` 引用的 defs 子项"保留，剪完对悬空引用 fail-closed。
`<filter>` 的内容不需要合成期缩放（`filterUnits` 默认 `objectBoundingBox`，且滤镜参数不是
用户空间坐标），所以它只吃第 1、2 步，不吃第 3 步。

## 真实语料证据（2026-11-23，R35 派单前）

`research/` 下 617 个 SVG 里 **0 个**含 `<mask>`/`<marker>`/`<filter>`，但**已经有一个真实产物被这类
缺陷咬过**：

- 源图 `research/白藜芦醇胰腺癌/steps/09_上游转录因子/出图/S36_S37/figure_panels/S36_panelA_matrix.svg`
  含 `<defs><pattern id="h9f23a50880" patternUnits="userSpaceOnUse" width="72" height="72">`（斜线 hatch 填充）。
- 历史产物 `.../work/tagged/S36_panelA_matrix.svg` 里 `<pattern>` **已消失**，只剩
  `style="fill: url(#h9f23a50880)"` 悬空 ⇒ 该 hatch 区域变成纯色。
- resvg 渲染对比：668 个像素颜色不同（bbox `(553,109)-(590,230)`，最大通道差 364），黑墨计数相同
  （66,996 vs 66,996）——**只看黑墨会漏掉，必须比颜色**。
- 当前 HEAD 的 `convert` 已能保住这个 `<pattern>`（`REFERENCE_TARGETS` 里有 `pattern`），
  所以这是**历史遗留产物**，不是当前回归；但它证明"按元素名白名单"这条路已经漏过一次。

⇒ 修法必须是"按引用保留"，而不是继续往名单里加名字。
