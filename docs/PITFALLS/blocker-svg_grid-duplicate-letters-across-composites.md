# svg_grid 阻塞点 — 两个已带字母的组合图直接组合，产物出现重复字母且 rc=0

场景：`tools/svg_grid` HEAD `b55669e`（2026-11-23，D5b 之后），`svg_grid plot-grid`（`./install.sh`
装到 `~/.local/bin/svg_grid`），输入是两次**各自加过 `--labels`** 的组合图。

## 现象

`--labels` 的 W3 硬错误只在"**本次调用**给非空字母、而输入已带字母"时触发。若两个输入**各自**已经带
字母（例如 `ab.svg` 有 A、B，`c.svg` 有 A），本次调用**不给** `--labels`，则：

- 退出码 **0**、stderr **0 字节**（`--verbose` 下也没有任何与字母相关的告警）；
- 产物里同时存在两个 `data-svg-grid-label="0"`，渲染出来是 **A、B、A** —— 两块面板同名，
  读者无法区分，正是 `docs/general-figure-guide/references/02a_svg_grid_assembly.md:63`
  所说"字母重号"要避免的结果。

该行文档写的是"传了会给行块整体重新编号，造成**字母重号/串位**（中途给组合图加字母是最常见的错法；
**现在直接 rc=2 报错**，不再是静默重号）"。实测：**不传** `--labels` 时重号仍然静默发生，
文档承诺的"不再静默重号"只覆盖了"传 `--labels`"这一半。

## 复现步骤

```bash
mkdir -p /tmp/dup && cd /tmp/dup
cat > p.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
  <circle cx="50" cy="40" r="20" fill="#000"/>
</svg>
EOF
cp p.svg p2.svg; cp p.svg p3.svg

# 第一步：A、B 组合，加字母
svg_grid plot-grid --input p.svg --input p2.svg --width 600 --ncol 2 --labels --output ab.svg
grep -o 'data-svg-grid-label="[^"]*"' ab.svg     # "0" "1"

# 第二步：单独一张，加字母（又从 A 开始）
svg_grid plot-grid --input p3.svg --width 600 --ncol 1 --labels --output c.svg
grep -o 'data-svg-grid-label="[^"]*"' c.svg      # "0"

# 第三步：两个组合图直接组合，不给 --labels
svg_grid plot-grid --input ab.svg --input c.svg --width 800 --ncol 2 --output abc.svg
echo "rc=$?"                                      # rc=0
wc -c < /dev/null                                 # stderr 0 字节
grep -o 'data-svg-grid-label="[^"]*"' abc.svg     # "0" "1" "0"  ← 重号
grep -o '>[A-Z]<' abc.svg                         # >A< >B< >A<
```

`--verbose` 复跑第三步：stderr 只有 W1/W6 与两条 note，**没有任何字母重号告警**。

真实语料同样可复现：`tests/fig2_mixed/out/tagged/A_heatmap.svg` + `B_A.svg` 先组 `ab.svg`，
`B_B.svg` 单独组 `c.svg`，再组合 ⇒ 产物 `data-svg-grid-label` 为 `0,1,0`，可见字母 `A,B,A`，rc=0。
