# 阻塞点 R31：显式网格允许整行全空 + 告警默认可见

## 现象

1. **整行全空的网格被接受。** 用户 INSTRUCTIONS 要求显式 `--ncol/--nrow` 满足
   `(rows-1)*cols < count <= rows*cols`。实现只检查了上界
   `rows*cols >= count`，所以 5 图 `--nrow 4`（推 `cols=2`，`3*2=6 >= 5`）
   会被接受：前 3 行画完 5 格，第 4 行整行空着，画布白白多出一行高度。
2. **W1–W9 与聚合 `note:` 默认就打 stderr。** 正常成功的 `plot-grid`/`convert`
   会输出非致命告警，与“默认只输出必要信息（报错）”的 CLI 契约不符，也不利于
   脚本消费 stderr；需要 `--verbose` 才显示诊断。

## 复现

```bash
cd tools/svg_grid
# 5 个 100x80 面板（含 data-panel-box）
svg_grid plot-grid $(for i in 0 1 2 3 4; do echo --input /tmp/d6/p$i.svg; done) \
  --nrow 4 --width 1000 --plot-margin 0 --output /tmp/d6/o.svg
```

- 修前：rc=0，画布 1000×1600，第 4 行没有任何 cell。
- 修后：rc=2，stderr 为：

  ```text
  grid leaves the last row empty: a 4 x 2 grid has 8 cells, but the 5 input slot(s)
  all fit in the first 3 row(s), so row 4 has no panel; fix: reduce --nrow to 3,
  or add 3 `--input NULL` placeholder(s) to fill all 8 cells
  ```

  `--plan` 同样 rc=2、同一句。

```bash
# 同一输入，默认与 --verbose 的 stderr 对比（2 图 --ncol 1 触发 W1）
svg_grid plot-grid --input /tmp/d6/p0.svg --input /tmp/d6/p1.svg \
  --width 1000 --plot-margin 0 --ncol 1 --output /tmp/d6/a.svg 2>/tmp/d6/a.err
wc -c /tmp/d6/a.err           # 修后：0
svg_grid ... --verbose 2>/tmp/d6/b.err
head -1 /tmp/d6/b.err         # 修后：svg_grid: warning: 2 inputs are scaled ...
```

## 根因

- `src/layout.rs` 的 `compute_grid`：容量检查只有
  `if rows * cols < spec.count { ... }`（旧 :93-97），没有下界检查；空行网格
  因此一路走到画布高度计算。
- 告警：`src/warnings/mod.rs` 的 `Warnings::emit(&self)` 只要消息非空就打印；
  `src/lib.rs` 的 `plot_grid` 只在末尾无条件调用 `warnings.emit()`；
  `convert/src/cli.rs` 的 warning 循环也无条件 emit。全仓没有 `--verbose` 解析点。

## 修法

### A. 网格下界（`src/layout.rs:93-123`）

```rust
let capacity = rows.checked_mul(cols).ok_or_else(|| InvalidInput("grid dimensions are too large"))?;
if capacity < spec.count { /* 既有“装不下”错误 */ }
let cells_above_last = capacity - cols;
if spec.count > 0 && cells_above_last >= spec.count {
    // 所有输入都放得进最后一行之前的各行 ⇒ 最后一行整行全空
}
```

错误文案包含：行列数、总格数、输入槽位数、空的是第几行、`reduce --nrow to N`
与需要补几个 `--input NULL`。count 是输入槽位数（含显式 NULL），由
`GridRequest` 的 input 长度传入，因此 NULL 补齐不会被误判。

### B. `--verbose`（默认静音）

- `src/warnings/mod.rs:189`：`pub fn emit(&self, verbose: bool)`，`!verbose` 直接返回；
  模块文档说明默认 stderr 只留报错。
- `src/lib.rs:140-147`：`plot_grid(spec)` 变为
  `plot_grid_with_diagnostics(spec, false)` 的薄包装；新函数接收 `verbose` 并在末尾
  `warnings.emit(verbose)`。`--plan` 的 stdout `note:` 契约完全不动。
- `src/cli/parse.rs:54` 注册 `--verbose`，`:113` 新增 `verbose: bool` 字段，
  `:217-218` 置真；帮助文本在 `src/cli/help.rs:80-100`。
- `src/cli/execute.rs:110-144` 取 `args.verbose` 传给库。
- `convert/src/options.rs` 增加 `verbose`；`convert/src/cli.rs:49` 注册 flag，
  `:106` 解析，`:256` 用 `if options.verbose` 包住 warning 循环，
  usage 与 `--help` 同步。
- doctor 需要继续判定 W7/W9，所以两处内部调用显式开诊断：
  `src/cli/doctor/run.rs:264` 给 stage 2 加 `--verbose`，`:476` 给 stage 1 传
  `Options { verbose: true, .. }`；doctor 的告警计数因此保持 57/0/1。

## 有意行为变更

**W1–W9 与聚合 `note:` 从“默认可见”改为“默认静音，`--verbose` 恢复”。**

- 报错（非法网格、重复 labels、缺字体、转换失败）不依赖 `--verbose`，rc 与文案不变。
- `--plan` 的 stdout `note:` 仍是计划契约，与 `--verbose` 无关。
- 告警文案本身未改，测试锁定的字符串原样保留。

## 测试

新增：

- `tests/selftest/layout/mod.rs:395 an_all_empty_last_row_is_rejected_by_run_and_plan`
  —— 4 个必触发案例，真跑与 `--plan` 都 rc=2、文案含行号/`--nrow`/NULL 修法；
  且不写输出。
- `tests/selftest/layout/mod.rs:458 short_last_rows_and_default_grids_are_not_mistaken_for_empty_rows`
  —— 5/`--ncol 2`、4/`--ncol 3 --nrow 2`、7/3×3、5/`--ncol 3`、5+3 NULL 4×2、
  1..9 默认网格全部 rc=0。
- `tests/selftest/warnings/mod.rs:23 diagnostics_are_silent_by_default_and_verbose_restores_them`
  —— 默认 stderr 0 字节；`--verbose` 恢复 W1+汇总；输出字节相同。
- `tests/selftest/warnings/mod.rs:82 plan_notes_stay_on_stdout_and_do_not_depend_on_verbose`
  —— `--plan` stdout 有 `note:`，两种模式 stdout 相同、stderr 均空。
- `tests/selftest/doctor/convert_cmd.rs:155/176`
  —— convert 默认静音 / `--verbose` 恢复告警。

既有告警测试全部改为显式 `--verbose`（`warnings`、`refs`、`fonts`、`compose`、
`labels`；`compose_one` 诊断 helper 直接带 flag）。未依赖旧“整行全空”几何的
快照；没有 CLI 快照需要更新。

## 验证

- R31 证据 `docs/PITFALLS/round-data/R31/test.py`：50 PASS / 0 FAIL，rc=0。
- 根 `cargo test --offline`：81 lib + 59 main + 83 selftest = 223，0 failed。
- `convert && cargo test --offline`：88 lib + 12 cli = 100，0 failed。
- `cargo fmt --check` 两 crate rc=0；`cargo clippy --offline --all-targets -- -D warnings`
  两 crate rc=0。
- `./install.sh` rc=0；`svg_grid doctor` = **57 pass / 0 fail / 1 skip**
  （matplotlib/mathtext rotated text 仍按注册 unsupported）。
- 实测：5 图 `--nrow 4` rc=2 且文案可读；5 图 `--ncol 2` rc=0；
  5+3 NULL 4×2 rc=0；默认 stderr 0 字节；`--verbose` 出声；
  `convert` 默认静音、`--verbose` 出声。

## 非阻断遗留

- doctor 现在依赖显式 `--verbose` 才能看到 W7/W9；已加注释，若以后改 doctor
  诊断策略需同步这两处。
- `docs/HANDOFF.md`/`ARCHITECTURE` 中“告警总是打印”的说明由 Arranger 统一更新，
  本次不改（任务约定）。
