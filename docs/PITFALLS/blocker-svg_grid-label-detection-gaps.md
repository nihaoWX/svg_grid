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

## 追加：R44 对抗评审的独立复现（2026-11-24，R45 之后）

评审（Evaluator，证据 `/tmp/r44_eval/evidence/README.md`）在 `4873156` 的 worktree
（`/tmp/r44_eval/pin`）上独立复现了上面三类，并补了两条本报告没写的：

### 第四类漏判：不可见字母被计入（反向误伤）

`<defs>` / `<symbol>` / `display:none` / `visibility:hidden` / `opacity="0"` 里的 marker
**全部参与查重**（五种形态全 rc=2），而 base 渲染"有/无该 marker"两版像素 diff `(0,0)`
—— 即对渲染完全正确的已 tagged 输入报错。原始未 tagged 输入不受影响（convert 会丢掉
`<defs>` 里的 text），所以只对"分步组图、手改中间产物"可达。

⇒ 修法必须**同时**做两件事：递归收集（补漏判）**且**跳过不渲染子树（防误伤）。
只做前者会把误伤面扩大。

### P 级（非阻断，建议同轮处理）

| # | 问题 | 实测 |
|---|---|---|
| P1 | `collisions` 用 `groups.iter_mut().find` 线性查找 ⇒ O(n²) | 40000 个互不相同字母 release 1.590 s vs base 0.075 s（21×）；5000/10000/20000 = 0.027/0.100/0.270 s |
| P2 | `LetterSite.cell` 恒等于 `input` | 内层字母实际在 cell 1 却报 cell 0；同一输入两个不同 cell 的 A 也报 `2 times inside #0 (cell 0)` |
| P3 | 文案建议 `pass unique --labels here` **不可执行** | 任何非空 `--labels` 都触发 W3 rc=2；`--labels none` / `''` / `,` 也 rc=2 |
| P4 | 生产/消费不对称 | `--labels A,A` 产出 rc=0（W4b 警告），把该产物再单独组合 ⇒ rc=2 |
| P5 | ≥5 处撞号折叠成 `"A"` 121 字符，丢掉全部 input#/cell 明细 | R44 脚本 A5 的 cell 断言在 5+ 时不再成立 |
| P6 | spacer 的字母不渲染（resvg 0 px）却参与查重 | — |
| P7 | 空文本口径无测试覆盖 | 变异 `if true` ⇒ 0 红 |
| P8 | 28 个带字母语料单独跑全 rc=0，两两组合 378 对里 227 对被拒 | 口径正确，但指南宜说明"中间产物不能再组合" |

## 附：R44 的另外两处非阻断（同一轮发现，可一并处理）

- **`cell` 字段恒等于 `input`**（`src/warnings/labels.rs:178-214` 的 `LetterSite`）：深层嵌套时
  文案 `"A" at #0 (cell 0) and #1 (cell 1)` 会误导——内层字母实际在更深的 cell 里。
- **≥5 个重号时站点列表塌成 plain**：n=2/3/4 ⇒ 152/168/184 字符（列出 `#0..#3`）；
  n≥5 ⇒ 121 字符只剩 `"A"`（`assemble_within` 的 `fold` 折叠），用户看不到具体位置。
  实测 6 个输入各带 A ⇒
  `svg_grid: invalid input: duplicate panel letters: "A". Fix: letter only at the outer layer, or pass unique --labels here.`
