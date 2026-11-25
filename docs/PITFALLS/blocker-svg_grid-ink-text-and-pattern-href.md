# 阻塞点：W6/W5 看不见「真正上色的是 `<tspan>`」与「pattern 只靠 href 拿内容」

- 发现于：R66 对抗评审（裁决 FAIL），相对 base `41face5` 的回归。
- 影响：`--check`/写模式都静默（rc=0、无 W6），产物里文字或 pattern 填充被画布边界裁掉却没人说；
  反向也有一批假阳性（pattern 的子元素全都不上色时仍报 W6）。
- 复现环境：`d1render` 需 `D1_FONT_DIR=<repo>/fontlib`，`d1render IN.svg OUT.png <SCALE>`；
  cairosvg 用 `~/.venv/bin/python`（透明白底跑 `svg2png`）。

## 复现 1（假阴性，P1）：文字的上色在 `<tspan>` 上

面板 100×80（`--plot-margin 0`，`--width 100 --height 80 --ncol 1`），面板外放：

```svg
<text x="200" y="40" font-size="20" fill="none"><tspan fill="#000">A</tspan></text>
```

- 期望：x≥100 有墨 ⇒ 报 W6。
- 实际（修前）：rc=0、无 W6；d1render 92 px 墨（cairosvg 95）。`fill="none"` 在外层 `<text>` 上，
  上色的 `fill` 在 `<tspan>` 上，`add_text_box` 只看外层 `ctx.paints()`，walker 也不为文本下钻。
- 同类漏判：`<tspan stroke>`；外层 `visibility="hidden"` + 内层 `visibility="visible"`；
  `<g fill="none"><text>…<tspan fill>`；同样内容经 `<use href="#t">` 引用（d1 均 92–158 px 墨，修前全静默）。
- 反向（修后必须修掉的假阳性）：`<text fill="#000"><tspan fill="none">A</tspan></text>` ⇒ d1 0 墨，修前报 W6。

## 复现 2（假阴性，P1）：pattern 的内容只靠 `href` 继承

```svg
<defs>
  <pattern id="b" patternUnits="userSpaceOnUse" width="4" height="4">
    <rect width="4" height="4" fill="#000"/>
  </pattern>
  <pattern id="p" href="#b"/>
</defs>
<rect x="200" y="10" width="20" height="20" fill="url(#p)"/>
```

- 期望：有墨 ⇒ W6。实际（修前）：rc=0、无 W6；d1render 400 px（cairosvg 400）。
- `pattern_paints` 只看自己的子元素，不跟 `href`/`xlink:href`（`gradient_paints` 早就跟了）。
  链式 `p→b→c` 同样漏；`p` 只有 `<title>` 子元素时 d1 仍按 href 画 400 px。

## 复现 3（假阳性，P2）：pattern 子元素不上色也算墨

`<pattern id="p">` 里只有一个 `fill="none"` / `transparent` / `fill-opacity="0"` /
`opacity="0"` / `visibility="hidden"` 的 rect（或 `<g fill="none"><rect/></g>`）⇒ d1 0 墨，修前报 W6。
修前 `pattern_paints` 只查子元素名字与 `display`，不查 paint。

## 复现 4（假阳性，P2）：`currentColor` 没解析

`<rect … fill="currentColor" color="transparent"/>` ⇒ d1 0 墨，修前报 W6（继承来的 `color` 可以是
`style` 或带 `!important`）。大小写敏感：`fill="currentcolor"` 是非法颜色 ⇒ d1 画黑 400 px。

## 复现 5（P3）：`--rel-widths 1e-9,1` 仍 rc=0

`--rel-widths 1e-9,1 --width 200 --ncol 2` ⇒ rc=0，`--plan` 打出 `cell 0x60.800`；
解析后 0.0002 px 的格子被当成有效几何。期望：宽度四舍五入到 0（<0.5 px）即 rc=2，点名 flag 与原始 token。

## 复现 6（P3）：R66 账本的定位信息不准

`round-data/R66/meta.toml` 记 `visit` 从 95 行开始（实际 94）、F5 的范围写成整个 `own_box` 区段
（实际是 `carries_supported` 154-160）；F12 把 `skilltest3/Figure.svg` 的 24 px 归因于 clip-path，
实测来自 `<rect x="24" y="24" width="4961" height="1919.428" fill="#FFFFFF" stroke="none"/>`
（max_x 4985）：删掉所有白底矩形后越界只剩 3.642 px（墨 x=4964.642），删 `visibility="hidden"` 面板不改变。
