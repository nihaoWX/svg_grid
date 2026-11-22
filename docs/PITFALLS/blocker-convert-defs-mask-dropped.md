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

