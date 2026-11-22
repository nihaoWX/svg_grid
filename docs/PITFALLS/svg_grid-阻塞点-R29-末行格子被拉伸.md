# svg_grid 阻塞点 — 短末行的格子被按“本行图数”拉伸（D2）

场景：`tools/svg_grid` HEAD `66fba3c`（2026-11-22，D1 注释提交之后）；`cargo test --offline` 的
207 个基线测试全绿。1000 宽画布、5 张 100×80（canvas aspect 1.25）面板、`--plot-margin 0`。

## 现象

显式 `--ncol/--nrow` 且末行图数少于 `cols` 时，末行格子被放大到“只按本行几个图铺满整宽”：

- 5 图 `--ncol 2`（3 行 2+2+1）旧输出 canvas `1000×1600`；前 4 格 `500×400`，
  末格 **`500×800`**（E 被纵向拉成整行高），`--plan` 的 `Δ=−50.00%`。
- 真实语料（`docs/HANDOFF.md` §D2）里同一机制给出 `476×972.614`、`Δ=−50%`。
- 5 图 `--ncol 3`（2 行 3+2）旧输出 canvas `1000×800`；前 3 格 `333.333×266.667`，
  末行两格 **`333.333×400`**。末行与上一行不等高。
- 满行网格不受影响（每行 aspect 和恰好等于全网格列权重和），所以 4 图 2×2、6 图 2×3 的
  旧数字看起来“正确”，缺陷只在短末行暴露。

## 复现步骤

```bash
cd /tmp && mkdir -p d2 && cd d2
for i in 0 1 2 3 4; do
  cat > p$i.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>
</svg>
EOF
done
svg_grid plot-grid $(for i in 0 1 2 3 4; do echo --input p$i.svg; done) \
  --plan --width 1000 --plot-margin 0 --ncol 2
# 旧：canvas 1000x1600；#0..#3 cell 500x400，#4 cell 500x800（Δ=-50%）
```

## 根因（已证）

`src/layout.rs` 的 `compute_grid` 用了两套基准：

1. 列宽：`column_signal = natural_w * rel_widths`，再按 `available_w * column_signal / sum_w`
   分配；`natural_w` 是**全网格每列**第一张非 NULL 图的 aspect（`natural_column_weights`）。
2. 行高：旧 `row_aspect_sums(rows, cols, count, aspects)` 只累计 `index < count` 的格子
   （旧 `src/layout.rs:195-208`，过滤条件在 `:203`），再算 `available_w / 本行 aspect 和`
   （旧 `:122-130`）。

于是末行只有 1 个图时，行高 = `available_w / 1.25` = 800，而格宽仍按“只占 1 列” = 500：
格子被纵向拉伸；`row_aspect_sums` 与列宽逻辑对“一行应该多宽”的回答不一致。

## 修法

`src/layout.rs:112-140`（真实函数体在 `compute_grid`，`src/layout.rs:112-140`）：

```rust
let natural_w_sum: f32 = natural_w.iter().sum();
if natural_w_sum <= 0.0 { /* fail-closed，同旧错误文案 */ }
// ...
let natural_row_h = available_w / natural_w_sum;
let row_signal: Vec<f32> = rel_heights
    .iter()
    .map(|multiplier| natural_row_h * multiplier)
    .collect();
```

- 行高信号 = `available_w / Σ(全网格 cols 列的 natural 列权重)`，所有行共享；短末行不再参与
  “本行几个图”的计算。缺失的尾格仍不生成 cell（`cells.len() < spec.count` 逻辑未动）。
- `row_aspect_sums` 整段删除（旧 `src/layout.rs:195-208`），`natural_column_weights` 保留。
- `--rel-heights` 仍是纯倍数（`row_signal = natural_row_h * rel_heights[i]`）；显式 `--height`
  仍按行权重比例填满画布。
- `--rel-widths` **不进入行高分母**：`rel_*` 是比值（`1,2` ≡ `2,4`，`ARCHITECTURE/layout.md` §1），
  且满行 + `--rel-widths` 的几何必须保持旧值（4 图 2×2 + `1,2` 仍是 `500×800` canvas 下的
  `333.333/666.667×400`）。若把倍数并入分母，把 `1,2` 写成 `2,4` 就会改变画布高度，
  违反该比值语义。
- 面板仍按 `cell_scale = (cell.w/canvas.w, cell.h/canvas.h)` 非等比填满格子；W1 Δ 告警未改。

## 验证数字

`--plan`（1000 宽、零 margin/gap，面板 aspect 1.25）：

| 输入 | 网格 | canvas | 每格 w×h | 行起点 y | 列起点 x |
|---|---|---|---|---|---|
| 5 图 `--ncol 2` | 3 行 2+2+1 | `1000×1200` | 全 `500×400` | `0 / 400 / 800` | `0 / 500` |
| 5 图 `--ncol 3` | 2 行 3+2 | `1000×533.333` | 全 `333.333×266.667` | `0 / 266.667` | `0 / 333.333 / 666.667` |
| 5 图 `--nrow 2` | 推导 cols=3，2 行 3+2 | `1000×533.333` | 全 `333.333×266.667` | `0 / 266.667` | 同上 |

- 5 图 `--ncol 2`：index 0 与 index 4 都是 `500×400`；末格 x=0、y=800；可用宽 1000 被
  `500 + 500` 恰好吃满（gap=0；gap>0 时的 `gap*(cols-1)` 扣除逻辑未动）。
- 5 图 `--ncol 3` / `--nrow 2`：两行等高；末行只有 `#3 (0,266.667)`、`#4 (333.333,266.667)`，
  不生成第 6 个空 cell。
- 满行回归：4/2×2 → `1000×800`、`500×400`；6/2×3 → `1000×1200`、`500×400`；
  9/3×3 → `1000×800`、`333.333×266.667`；16/4×4 → `1000×800`、`250×200`；
  15/5×3 → `1000×480`、`200×160`。数字与改前逐项相同。
- `--rel-heights 1,1,2` + 5 图 2 列：canvas `1000×1600`，行高 `400/400/800`，末格仍 `500` 宽
  （只被 rel-heights 拉高，不被末行逻辑拉宽）。
- `--rel-widths 1,2` 与 `2,4`：canvas、每格尺寸完全一致。

**注意（验收文字的内部冲突）**：任务第 5 条把“15 图 4×4”列为“满行网格不得变化”，但 15 图在
4 列下必然是 `4+4+4+3` 的短末行（这正是 D2 要修的形状）。以 aspect 1.25 为例，旧几何
canvas `1000×866.667`（末行 3 格 `250×266.667`），新几何 `1000×800`（全 `250×200`）——
**必然变化**，否则 D2 不成立。真正“满行”的 16/4×4 与 15/5×3 已钉死不变量；若第 5 条原意是
16 图 4×4，则与之相符。此点已在返回里标为需要确认的口径问题，未按错误示例改代码。

## 测试与全链

新增 6 条测试（5 单元 + 1 集成）：

- `src/layout.rs:266` `short_last_row_keeps_the_full_grid_cell_size`
- `src/layout.rs:286` `short_last_row_is_measured_as_if_it_filled_all_columns`（含 `--nrow 2` 等价）
- `src/layout.rs:312` `complete_rows_keep_the_exact_geometry_they_had_before`
- `src/layout.rs:329` `rel_heights_only_scale_the_shared_row_height`
- `src/layout.rs:354` `rel_widths_are_ratios_and_do_not_change_the_derived_height`
- `tests/selftest/layout/mod.rs:182` `plan_grid_keeps_a_short_last_row_at_the_full_grid_cell_size`
  （走 `plan_grid`，验证 `--plan` 与 `plot-grid` 共用的 `prepare`）

既有测试/快照：全仓搜索后**没有**依赖旧末行几何的断言，无需改期望值；`tests/user_tests/2026-11-20/`
的 `ABCDE.svg` 是 `--ncol 5` 的字节副本（HANDOFF 已注明），不受影响。

全链结果：

- 根 crate `cargo test --offline`：79 lib + 59 main + 75 selftest，**0 failed**。
- `convert && cargo test --offline`：88 lib + 12 cli，**0 failed**。
- `cargo fmt --check`：根 crate / `convert` 均 rc=0。
- `cargo clippy --offline --all-targets -- -D warnings`：两个 crate 均 rc=0。
- `./install.sh`：rc=0（release 重装 + skill/fixtures/fonts up-to-date）。
- `svg_grid doctor`：**57 pass / 0 fail / 1 skip**（skip 仍为 `matplotlib/mathtext` 的旋转文本）。
- `pitfalls-manager query src/layout.rs --repo .` 命中的 R14 证据：
  `docs/PITFALLS/round-data/R14/test.py` 全 39 项 PASS，rc=0。

## 遗留（非阻断）

1. **验收第 5 条的 15 图 4×4** 与 D2 的“短末行必须改”逻辑冲突，见上；需要用户/评审确认是否
   应为 16 图 4×4。未为了迎合该示例而保留旧末行拉伸。
2. **D3 默认网格**仍是 `(None, None) => (1, count)`，未抢做；5 图不给 `--ncol/--nrow` 依旧是一行。
3. **多出整行空位**（如 5 图给 `--nrow 4 --ncol 2`）目前只校验“装不下”，末行整行空时新逻辑会给它
   分配正常行高（旧逻辑为 0）；这属于 D6 的“rows×cols 上界”校验，本轮未动。
4. `--plan` 输出只有 cell 的 w×h，没有 x/y；报告中的 x/y 由 `compute_grid` 单元测试与合成 SVG 的
   `data-panel-box` 位置交叉确认（fixture 的 panel box 在 0,0）。
