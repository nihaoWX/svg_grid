# svg_grid 阻塞点 — R44 的字母重号检测有三类漏判（仍 rc=0 静默重号）

场景：`tools/svg_grid` HEAD `cec8eff`（2026-11-24，R44 之后），`svg_grid plot-grid`
（`./install.sh` 装到 `~/.local/bin/svg_grid`）。R44 把"字母唯一性"提升为整张图的性质，
但**字母的提取方式**只覆盖了"marker 元素直接包着文本"这一种形态。

## 现象

三类输入仍然 **rc=0、stderr 0 字节**，产物里同一字母出现两次：

| # | 形态 | 结果 |
|---|---|---|
| 1 | `<text data-svg-grid-label="0"><tspan>A</tspan></text>` | rc=0，产物两个 `>A</text>` |
| 2 | `<g data-svg-grid-label="0"><text>A</text></g>` | rc=0，产物两个 `>A</text>` |
| 3 | 一个写 `A`、另一个写 `' A '`（或 `&#65;`） | rc=0，视觉上都是 A |

根因在 `src/label_conflicts.rs:124-146` 的 `collect_letters`：

```rust
if let Some(marker) = element.attr("data-svg-grid-label") {
    let letter = element
        .children
        .iter()
        .filter_map(|child| match child {
            dom::Node::Text(text) => Some(text.as_str()),
            dom::Node::Element(_) => None,      // ← tspan / g 包裹的文本被丢弃
        })
        .collect::<String>();
    if !letter.is_empty() { out.push(...); }    // ← 空串直接跳过，连"有 marker"都不记
}
```

- 形态 1/2：marker 元素的**直接** `Text` 子节点为空 ⇒ `letter` 为空 ⇒ 整条被跳过。
- 形态 3：`letter` 原样保留（不 trim），`" A "` ≠ `"A"` ⇒ 分组时算两个不同字母。
  另外 `--labels " A ",B` 走的是显式路径，那里**会** trim 成 `A,B` ⇒ 两条路径口径不一致。

## 复现步骤

```bash
mkdir -p /tmp/labgap && cd /tmp/labgap
cat > base.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>
  <circle cx="50" cy="40" r="20" fill="#333"/>
</svg>
EOF

# 形态 1：tspan 包裹
sed 's|</svg>|<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"><tspan>A</tspan></text></svg>|' \
  base.svg > t1.svg
cp t1.svg t2.svg
svg_grid plot-grid --input t1.svg --input t2.svg --width 400 --ncol 2 --output tspan.svg
echo "rc=$?"                                        # rc=0
grep -o 'data-svg-grid-label="[^"]*"' tspan.svg     # "0" "0"  ← 重号
grep -o '<text[^>]*>[^<]*</text>' tspan.svg         # 两个 <tspan>A</tspan>

# 形态 2：marker 挂在 <g> 上
sed 's|</svg>|<g data-svg-grid-label="0"><text x="4" y="14" font-size="12" font-family="Arial">A</text></g></svg>|' \
  base.svg > g1.svg
cp g1.svg g2.svg
svg_grid plot-grid --input g1.svg --input g2.svg --width 400 --ncol 2 --output gw.svg
echo "rc=$?"                                        # rc=0

# 形态 3：空白差异
sed 's|</svg>|<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">A</text></svg>|' \
  base.svg > w1.svg
sed 's|</svg>|<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial"> A </text></svg>|' \
  base.svg > w2.svg
svg_grid plot-grid --input w1.svg --input w2.svg --width 400 --ncol 2 --output ws.svg
echo "rc=$?"                                        # rc=0
```

## 影响面

- 形态 1（`<tspan>`）**不是臆造，语料里就有**：`tests/fixtures/matplotlib/mathtext.svg` 有 39 个
  `<tspan>`，形态是 `<text><tspan x="0" y="-0.6875" style="…">x</tspan><tspan …>…</tspan></text>`
  ——matplotlib 的 mathtext 后端把一段文字拆成多个 tspan（不同字号/上下标），
  `tests/fixtures/matplotlib/log_axes.svg` 同样有。R/ggplot2 的 `ggsave` 也会这样拆。
  只要字母被拆进 tspan，检测就失效。
- 形态 2（marker 在 `<g>` 上）在手工编辑或某些导出器里常见。
- 形态 3（空白/实体）在 `&#65;` 这类数字实体上尤其隐蔽——它与
  `docs/PITFALLS/blocker-svg_grid-numeric-entities-double-escaped.md` 是同一族问题：
  **文本在 DOM 里是字面量，在渲染后才是字符**，按字面量比较必然漏。

## 建议

1. `collect_letters` 改为**递归收集 marker 子树里的全部文本**（`Text` 节点拼接，跳过
   `display:none`/`visibility:hidden` 子树），而不是只看直接子节点。
2. 比较前做**规范化**：trim 首尾空白、把连续空白折叠成一个空格；数字实体在
   `resolve_reference` 修好后自然一致（见数字实体报告），修好前至少把 `&#…;` 也纳入规范化。
3. 显式 `--labels` 路径与继承路径用**同一个规范化函数**，消除"显式 trim、继承不 trim"的口径差。
4. 补测试：tspan 包裹、`<g>` 包裹、空白差异、数字实体差异，各一条；并加渲染对照
   （不能只断 markup）。

## 附：R44 的另外两处非阻断（同一轮发现，可一并处理）

- **`cell` 字段恒等于 `input`**（`src/warnings/labels.rs:178-214` 的 `LetterSite`）：深层嵌套时
  文案 `"A" at #0 (cell 0) and #1 (cell 1)` 会误导——内层字母实际在更深的 cell 里。
- **≥5 个重号时站点列表塌成 plain**：n=2/3/4 ⇒ 152/168/184 字符（列出 `#0..#3`）；
  n≥5 ⇒ 121 字符只剩 `"A"`（`assemble_within` 的 `fold` 折叠），用户看不到具体位置。
  实测 6 个输入各带 A ⇒
  `svg_grid: invalid input: duplicate panel letters: "A". Fix: letter only at the outer layer, or pass unique --labels here.`
