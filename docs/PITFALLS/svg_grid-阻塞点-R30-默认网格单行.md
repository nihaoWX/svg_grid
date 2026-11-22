# svg_grid 阻塞点 — 默认网格是“一行”，与 cowplot / 用户 INSTRUCTIONS 相反（D3）

场景：`tools/svg_grid` HEAD `26758ea`（D2/R29 已完成）；`cargo test --offline` 基线
79 lib + 59 main + 75 selftest 全绿。1000 宽、5 张 100×80（canvas aspect 1.25）面板、
`--plot-margin 0`。

## 现象

不给 `--ncol/--nrow` 时，所有输入被排成一行 `1×count`：

- 5 图默认 ⇒ `--plan` 输出 canvas `1000×160`、5 个 cell 全 `200×160`；cowplot 的
  用户预期是 `2×3`（3+2）、`1000×533.333`、`333.333×266.667`。
- 15 图默认 ⇒ `1000×53.333`、格 `66.667×53.333`；预期 `4×4`、`1000×800`、`250×200`。
- 违反 `INSTRUCTIONS.md` 第 26 行（默认近方形、末行不满合法、末尾槽位按 NULL 处理）。

## 复现步骤

```bash
cd /tmp/d3
for i in 0 1 2 3 4; do
  cat > p$i.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>
EOF
done
svg_grid plot-grid --input p0.svg --input p1.svg --input p2.svg \
  --input p3.svg --input p4.svg --plan --width 1000 --plot-margin 0
# 旧：canvas 1000x160；5 格全 200x160
```

## 根因

`src/layout.rs` 的 `compute_grid` 在“两个网格参数都没给”时的分支写死为一行：

```rust
(None, None) => (1, spec.count),
```

（旧 `src/layout.rs:69-74`，`(None, None)` 分支在 :73。）几何本身继承 D2 的“等行高、
短末行不拉伸”，所以一行时每格就是 `available_w/count` 宽、`宽度/aspect` 高；问题只在
**行列推导**，不在格子算术。

## 修法

`src/layout.rs:59-69` 新增默认网格函数，`compute_grid` 的 `(None, None)` 分支改为调用它
（`src/layout.rs:81-86`）：

```rust
fn default_grid(count: usize) -> (usize, usize) {
    if count == 0 {
        return (0, 0);
    }
    let cols = (count as f64).sqrt().ceil() as usize;
    (count.div_ceil(cols), cols)
}
```

- `cols = ceil(sqrt(count))`、`rows = ceil(count / cols)`；`count` 是输入**槽位数**，
  显式 `--input none`/spacer 也计入（`prepare` 传 `request.inputs.len()`，天然成立）。
- 末行不满合法；缺失的尾槽不生成 cell（D2 的 `cells.len() < spec.count` 逻辑未动）。
- 只给 `--nrow` 或只给 `--ncol` 的推导保持原样；两者都给的行为也不变（D6 才加上界校验）。
- 显式 `--height`、`--rel-widths`、`--rel-heights`、margin/gap 的语义全部未动。
- `src/cli/help.rs:41-42` 把“Both omitted ⇒ one row”改为近方形公式。

## 验证数字

1000 宽、100×80 面板、margin/gap 0。旧默认是一行；新默认：

| 输入数 | 新网格 | canvas | 每格 w×h | 旧 canvas | 旧每格 |
|---|---|---|---|---|---|
| 1 | 1×1 | `1000×800` | `1000×800` | `1000×800` | `1000×800` |
| 2 | 1×2 | `1000×400` | `500×400` | `1000×400` | `500×400` |
| 3 | 2×2 | `1000×800` | `500×400` | `1000×266.667` | `333.333×266.667` |
| 4 | 2×2 | `1000×800` | `500×400` | `1000×200` | `250×200` |
| 5 | 2×3 | `1000×533.333` | `333.333×266.667` | `1000×160` | `200×160` |
| 6 | 2×3 | `1000×533.333` | `333.333×266.667` | `1000×133.333` | `166.667×133.333` |
| 7–9 | 3×3 | `1000×800` | `333.333×266.667` | `1000×≤114.286` | `≤142.857` 宽 |
| 10–12 | 3×4 | `1000×600` | `250×200` | `1000×≤100` | `≤100` 宽 |
| 13–16 | 4×4 | `1000×800` | `250×200` | `1000×≤76.923` | `≤76.923` 宽 |
| 17 | 4×5 | `1000×640` | `200×160` | `1000×58.824` | `58.824×47.059` |

- 5 图默认：行 y=`0/266.667`；前三格 x=`0/333.333/666.667`，末行两格 x=`0/333.333`；
  末格在 `(333.333, 266.667)`，与首行同列格子同尺寸，无拉伸。
- 15 图默认：4×4，行 y=`0/200/400/600`，列 x=`0/250/500/750`，全格 `250×200`。
- 单图/双图默认输出与旧的显式一行写法（`--ncol 1` / `--ncol 2`）**逐字节相同**。
- 4 图 + 显式 `--input none` ⇒ 按 5 个槽位排 `2×3`（canvas `533.333`），不是 4 槽的 `2×2`。
- 显式网格回归：5 图 `--ncol 2` 仍 `1000×1200`；`--nrow 2` 仍 `1000×533.333`；
  15 图 `--ncol 4` 仍 `1000×800`、`250×200`。
- `--plan` 与真跑同源：5 图默认的 canvas 高、5 个 cell 与合成 SVG 的 `height`/cell 数一致。

## 有意改变的测试期望

**无。** 全仓 `cargo test` 未修改任何既有断言即全绿：

- 32 处 `plot_grid(PlotGridSpec)` / `plan_grid(GridRequest)` 都显式给了 `nrow`/`ncol`；
- 7 处 CLI `run_plot_grid` 调用也都带 `--ncol`（或只查 `--list-fonts`）；
- 既有测试没有断言“默认即一行”的几何，因此没有期望值需要更新。
- `tests/user_tests/2026-11-20/README.md` 的真实语料调用都用 `--ncol 2` / `--ncol 5`，
  不依赖默认网格；按约定只报告，不改语料文件。
- `doctor` 的 stage 2 每个 fixture 只传 1 个 tagged 输入 ⇒ 默认 `1×1`，几何与改前相同，
  结果仍 57 pass / 0 fail / 1 skip。

## 新增测试

- `src/layout.rs` 单测 `default_grid_is_the_near_square_cowplot_one`：1..17 全表。
- `src/layout.rs` 单测 `default_grid_inherits_the_d2_row_geometry`：5 图默认行/列/格子、
  15 图 4×4、1/2 图旧的 `1×1`/`1×2` 几何。
- `tests/selftest/layout/mod.rs` `default_grid_is_near_square_and_the_plan_matches_the_run`：
  5 图默认 `plan_grid` 与真跑 `plot_grid` 的 canvas/格/末格位置一致。
- `tests/selftest/layout/mod.rs` `explicit_null_counts_as_a_slot_in_the_default_grid`：
  显式 NULL 也计入槽位数。

## 全链结果

- 根 crate `cargo test --offline`：81 lib + 59 main + 77 selftest = 217，0 failed。
- `convert && cargo test --offline`：88 lib + 12 cli，0 failed。
- 两个 crate `cargo fmt --check` rc=0；`clippy --all-targets -- -D warnings` rc=0。
- `./install.sh` rc=0；`svg_grid doctor`：57 pass / 0 fail / 1 skip。
- R14 证据、R29 证据仍全 PASS（显式网格未被 D3 改坏）。

## 遗留（非阻断）

1. `--plan` 仍只输出 cell 的 w×h，没有 x/y；x/y 由 `compute_grid` 单测与合成 SVG 的
   `data-svg-grid-cell` 位置交叉确认。
2. `default_grid(0)` 返回 `(0, 0)`，随后被既有的“rows/cols 必须大于零”错误接住；
   CLI 在 `prepare` 早已拒绝空输入，正常路径不可达。
3. 只给 `--nrow`（如 5 图 `--nrow 4`）仍推导 `cols=ceil(5/4)=2`，会出现 4 行 2+2+1+0；
   这是既有推导 + D6 待加的上界校验，本轮未动。
4. `ARCHITECTURE/*.md`、`PROGRESS.md`、`INSTRUCTIONS.md` 未改（由 Arranger 更新）。
