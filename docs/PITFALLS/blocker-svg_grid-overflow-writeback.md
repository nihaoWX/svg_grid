# R61 — 有限输入 × 轴因子溢出仍写 `inf`/`NaN`（P2），及三处证据/口径空转

**状态：A（P2）、B、C、D、E 已修。** 本报告是本轮账本的来源文件
（`pitfalls-manager record --from-json` 的 `reports`），可跑证据在
`docs/PITFALLS/round-data/R61/{test.py,meta.toml}`。每条先 d1render 实测再改码；
“修前”指 R60 的 HEAD `cd5875d`（评审也用它，`/tmp/r60_eval/pin`），“修后”是本轮
`target/release/svg_grid`。R60 评审裁决 PASS（无 P0/P1）；本轮修它留下的 P2 与
三条 P3。

## 1. 现象

R60 评审 P2：输入数值有限、因子也有限，但**乘积**溢出 `f32` 时，写回仍是
`format_number(value * factor)` ⇒ 产物出现 `inf`/`NaN`/`-inf`，且不报 W7。
`format_number`（`src/lib.rs`）对 `inf.fract()`（= NaN）走 `format!("{value:.3}")`
分支，于是打印 `inf`/`NaN`；这些不是合法的 SVG 数值，浏览器的容错纯属运气。

复现 fixture：源 100×80，`plot-grid --width 800 --height 640 --ncol 1
--plot-margin 0` ⇒ s=8。修前逐条（`round-data/R61/test.py` 的 A 族）：

| 行 | 输入 | 修前写回 |
|---|---|---|
| A1 | `<rect x="1e38" y="10" width="20" height="20">` | `x="inf" y="80" width="NaN" height="160"` |
| A2 | `<rect width="1e38" height="20">` | `width="inf"`（其余三边照缩） |
| A3 | `<circle cx="1e38" cy="10" r="40">` | `cx="inf"`（`cy` 也停旧坐标） |
| A4 | `<polygon points="1e38,10 20,20 10,30">` | `points="inf,80 160,160 80,240"` |
| A5 | `<path d="M 1e38 10 L 20 20 L 10 30 Z">` | `d="M inf 80 …"` |
| A6 | `<rect stroke-width="3e38">` | `stroke-width="inf"` |
| A7 | `<feGaussianBlur stdDeviation="3e38">` | `stdDeviation="inf"` |
| A8 | `<feDisplacementMap scale="3e38">` | `scale="inf"` |
| A9 | `<rect x="-1e38">` | `x="-inf" width="NaN"` |

两类因子本身非有限（源图自身被 `inf`/`nan` 污染）同样复现：canvas
`1e-45×1e-45` + `--normalize-input-max-side 400` ⇒ 归一化因子 `inf` ⇒ 面板
`x="NaN" width="inf"`；面板 `width="nan"` ⇒ 变换因子 `NaN` ⇒ 图元全部 `NaN`。
两者 rc 都是 0，且没有任何 W7。

## 2. 复现

```bash
cd /home/nihao/bioagentforge/tools/svg_grid && cargo build --release --offline

# 修前：R60 HEAD 的构建（评审用的同一份）
/tmp/r60_eval/pin/target/release/svg_grid
# 或自建：git archive cd5875d | tar -x -C /tmp/r61before && cp -r fontlib /tmp/r61before/
#         (cd /tmp/r61before && cargo build --release --offline)

# 全绿（85 checks）
python3 docs/PITFALLS/round-data/R61/test.py
# 有牙：被测指回 R60 ⇒ rc=1（非有限 token 与“原文保留”同时红）
SVG_GRID_BIN=/tmp/r60_eval/pin/target/release/svg_grid python3 docs/PITFALLS/round-data/R61/test.py
# 缺二进制/渲染器 ⇒ rc=1（--allow-skip 才 skip）
SVG_GRID_BIN=/nope python3 docs/PITFALLS/round-data/R61/test.py
```

fixture 用**已转换形态**（自带 `<rect data-panel-box="main">`），否则 `plot-grid`
的自动转换会把直线路径拍平成 polygon、把 `1e38` 饱和成
`9223372036854775807`（有限值，反而掩盖 d 的写回）。

## 3. 根因

- `src/transform/lengths.rs::rewrite_scaled_length` 与
  `src/transform/primitives.rs::{rewrite_pair,rewrite_axes,rewrite_rect,
  rewrite_points,rewrite_rotate_transform}` 只判**输入** `is_finite()`，不判
  **乘积**；`rewrite_rect` 的 `(x1-x0).abs()` 在 `inf-inf` 时得 `NaN`。
- `src/transform/filters.rs::scale_axis_with/scale_pair`、`src/transform/lights.rs`
  的缺省位置、`src/transform/patterns.rs` 的 tile、`src/transform/transforms.rs`
  的矩阵分量同理。
- `src/normalize.rs`（`--normalize-input-max-side`）每个缩放点都直接
  `format_number(n * factor)`，且因子自身可能是 `inf`。
- `src/path/rewrite.rs` 先把数字 `format_number` 成字符串再 join，溢出时 d 里出现
  `inf`。

## 4. 修法

一个判据：**乘积（以及因子本身）有限才写回；否则逐字节保留原文 + W7**。
`src/transform/lengths.rs` 新增：

```rust
pub(crate) fn scaled_finite(value: f32, factor: f32) -> Option<f32> {
    let scaled = value * factor;
    (factor.is_finite() && scaled.is_finite()).then_some(scaled)
}
pub(crate) fn note_overflow(…, raw: &str, factor: f32)  // W7: 溢出 / 非有限因子
```

所有计算写回点改走它（`lengths.rs`、`primitives.rs`、`filters.rs`、`lights.rs`、
`patterns.rs`、`transforms.rs`、`normalize.rs`、`paths.rs`）。`src/style.rs` 改为
`scale_length_report` 返回“改好的串 + 保留的 token 列表”，属性路径与 `style=`
路径**报同样的 W7**（D）。`src/path/rewrite.rs` 增加哨兵
`NON_FINITE = "scaled coordinate is not finite"`：d 里出现非有限 token 就整条
`d` 保留（畸形 path 的原文案不变）。

写回分组是**原子**的：一组里任一乘积非有限，整组保留（rect/image 四边、
`cx/cy` 一对、`points` 列表、`d`、filter 的 1–2 个数字）。理由：半组写回会让
一个坐标落在新空间、另一个留在旧空间；且这与既有规则“输入任一非有限就两个都
保留”一致。

因子非有限时 W7 点名 `input #0`（`Passthroughs::record(index, …)`），因为病在
源图自身；rc 仍为 0、产物照写（W7 非致命，浏览器宽松），只是产物必须良构。

## 5. 四列数字（修前产物 / 修后产物 / 原文 / d1render）

| 行 | 修前 | 修后 | 原文 | d1 |
|---|---|---|---|---|
| A1 | `x="inf" width="NaN"` | `x="1e38" y="10" width="20" height="20"` | 同修后 | rc=0 |
| A2 | `width="inf"` | `width="1e38" height="20"` | 同修后 | rc=0 |
| A3 | `cx="inf"` | `cx="1e38" cy="10" r="320"` | cx/cy 原文 | rc=0 |
| A4 | `points="inf,80 …"` | 原 `points` | 同修后 | rc=0 |
| A5 | `d="M inf 80 …"` | 原 `d` | 同修后 | rc=0 |
| A6 | `stroke-width="inf"` | `stroke-width="3e38"` | 同修后 | rc=0 |
| A7 | `stdDeviation="inf"` | `stdDeviation="3e38"` | 同修后 | rc=0 |
| A8 | `scale="inf"` | `scale="3e38"` | 同修后 | rc=0 |
| A9 | `x="-inf" width="NaN"` | `x="-1e38"` | 同修后 | rc=0 |
| B | `x="NaN" width="inf"` | `x="10"`（跳过缩放） | 源图 1e-45 | rc=0 |
| D | 无 W7 | `style token "3e38" kept verbatim` | 字节不变 | — |
| E | 位移过 4.5× | `14.142`（xor 0） | R58 A9 | rc=0 |

反证（同一套测试里）：普通值照缩（`x=10→80`、`w=20→160`）；有限大值
`1e9→8000000000` 照缩；`1e-45` 在 `--normalize-input-max-side 400` 下写 `0`
（有限乘积，`format_number` 的三位小数规则；修前修后渲染逐字节相同）。

## 6. 不变量测试

- 单测：`src/transform/tests_r61.rs` 13 条 + `src/normalize.rs` 3 条。断言
  “产物里 `inf`/`NaN` 计数不增加”“W7 点到元素/属性”“原文逐字节保留”。
- 证据：`round-data/R61/test.py` 85 checks。A 族 9 行 + B（非有限因子）+ C
  （正常值/`1e9`/`1e-45` 反证）+ D（style 两条）+ E（R58 可判别像素）+ F
  （`convert` 保字节）。每行都扫**全文属性**的正则
  `(?i)(?<![a-z0-9])(-?inf|nan)(?![a-z0-9])`，并跑 d1render。
- 语料：`tests/**/*.svg` 的 214 个（除 `tests/user_tests/`）经 `convert
  --report`；58 个 `tests/fixtures/**` 经 `plot-grid --plan`。

## 7. 口径与决策

- **`1e-45` 不算本缺陷。** 因子有限、乘积有限，只是三位小数格式化成 `0`；修前
  修后渲染一致。保留语义，在 meta 里写明。
- **源图自带 `nan`/`inf` 时保留原字节。** 这是 “kept verbatim” 契约本身；
  不变量语料只用有限源，避免正则把源图的病算到工具头上。
- **`--plan` 不打印 W7。** `plan_grid_checked` 用临时 `Passthroughs`，保持
  `--plan` 现有告警面（它不写产物）；`plot-grid` 正常路径共享同一个收集器。
  这是与“所有路径都报”的一处自觉偏离，已在本报告与 meta 登记。
- **B 口径更正。** 评审实测：spot z 三位小数 `45.277` 只差 **5 像素**
  （maxdiff 1），point light **1 像素**，精确值 **0 像素**；原文档“11 像素 /
  精确值仍留 4 像素”是错的。方向（必须最短往返）不变。R60-F1 的 reason 不可
  改写 ⇒ 本轮记一条 `related R60-F1` 的更正 finding。
- **C 证据空转。** R60 F5-xy 的 `body.replace("const_map(255,128)", …)` 发生在
  data URI 插值之后 ⇒ 恒为 128、只测了属性。改为把 map 作为 `body(map_uri)`
  的参数在插值前注入，并用 `scale="0"` 的对照测真实像素位移（40→38、50→48，
  各 1..6 px）。R58 A9 的 fixture 两种取值 xor 都是 0（不可判别）⇒ 注释指向
  判别证据，并把可判别用例放进 R61：`14.142→xor 0`、`20→39420`、`13.5→4072`。
- **C rotate。** `rotate(nan 50 40)` 端到端不可达（`plot-grid` 直接 rc=2
  `unsupported transform`），故不做端到端断言，改在
  `tests_r61.rs::an_overflowing_rotate_centre_is_kept_verbatim` 直接测
  `rewrite_rotate_transform`。

## 8. 变异（实跑，随后还原）

| 变异 | 单元红名 | 证据红 |
|---|---|---|
| M1 不判乘积（`scaled_finite` 直接返回乘积） | `normalize::tests::an_overflowing_coordinate_is_kept_verbatim_with_w7`、`transform::tests_r61::an_overflowing_displacement_scale_is_kept_verbatim` | A6/A8 各 3 条（6 failed） |
| M2 保原文改成写 `inf`（rect 溢出仍写四边） | `an_overflowing_rect_keeps_all_four_attributes`、`an_overflowing_pair_is_kept_on_both_axes`、`a_non_finite_factor_keeps_every_length_verbatim` | A1/A2/A9 共 12 条 |
| M3 W7 静音（`note_overflow` 空实现） | 9 条（rect/pair/axes/points/rotate/filter/displacement/factor/normalize） | 8 条 W7 断言 |

## 9. 回归

`cargo test --offline` 674 = 382 + 59 + 233（R60：658 = 366 + 59 + 233）；
fmt 干净；`clippy --all-targets -D warnings` 干净；`./install.sh` 后
`target/release` 与 `~/.local/bin` sha 一致
（`d6878edc…`）；`doctor` 57 pass / 0 fail / 1 skip；214 convert + 58
`--plan` 对 `/tmp/r50base`（`75def0b8…`）逐字节 0 差。

## 10. 不做

- 内容域第二条轴（verbatim body 出边不传 policy、按内容域克隆定义）⇒ R62。
- `src/transform/defs.rs`（19895 B）本轮不动。
- `1e-45` 的格式化口径不改（见 §7）。
