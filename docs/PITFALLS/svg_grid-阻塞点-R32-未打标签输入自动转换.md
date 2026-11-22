# R32 阻塞点：`plot-grid` 不自动转换未打标签的输入（用户期望 #11）

## 现象

把 R/svglite 或 matplotlib 直接写出的 SVG（没有 `data-panel-box="main"`）交给
`svg_grid plot-grid`，命令直接失败，rc=2：

```
svg_grid: invalid input: input SVG must contain a <rect data-panel-box="main" .../> element
```

用户期望 `plot-grid` 能自己调用 `svg_grid convert` 把这些输入转成面板，且**不得改动原图**。

## 复现

```bash
cd tools/svg_grid
cat > /tmp/untagged.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="288pt" height="216pt" viewBox="0 0 288 216">
<style>.svglite circle { stroke: #000; fill: none; }</style>
<circle cx="60.82" cy="135.96" r="4" style="stroke-width: 1.5;"/>
</svg>
SVG
target/debug/svg_grid plot-grid --input /tmp/untagged.svg --output /tmp/out.svg --width 500 --ncol 1
# 旧行为：rc=2，上面的缺 panel-box 文案；新行为：rc=0，/tmp/out.svg 含 data-svg-grid-cell="0"
```

`--plan` 同样失败；旧实现里 `convert` 是纯字符串的库函数，但 `prepare` 从未调用它。

## 根因

`src/plan.rs:125 prepare()` 对每个输入无条件执行：

```rust
let panel = geom::find_panel_box(&root)?;
```

`geom::find_panel_box`（`src/geom.rs:20`）只接受已带 `data-panel-box="main"` 的 SVG，
没有“先转换”的路径；而 `convert/src/pipeline.rs:22 convert(source, force)` 已经是不写文件的
纯函数，具备在进程内转换的条件。

## 修法

1. `src/geom.rs:31` 新增 `pub(crate) fn find_panel_box_opt()`，复用原有递归查找；
   `find_panel_box` 的公开语义与错误文案保持不变（`src/geom.rs:18-27`）。
2. `src/plan.rs:155-174`：解析后先探测 panel box；没有则 `src/plan.rs:266 auto_convert()`：
   - 调用 `svg_grid_convert::convert(source, false)`（纯字符串，无临时文件、无回写）；
   - `Conversion::Converted` → 重新解析转换后的字符串继续；
   - 转换后仍无 panel box / 解析失败 ⇒ fail-closed，不循环；
   - 每条输入最多转换一次；已有 panel box 的输入完全跳过该分支（组合图、嵌套中间产物、
     doctor stage2 字节不变）。
3. 先转换、后 `--normalize-input-max-side` 归一化，再 `find_panel_box` / `find_canvas_box`。
4. 失败文案（`src/plan.rs:291 auto_conversion_error()`）点名输入、说明“无 panel box 且
   自动 convert 失败”、列前 3 条 finding（每条 ≤200 字符）、给出修复建议
   （换方言，或先手动 `svg_grid convert --check --report`）。`--plan` 与真跑共用
   `prepare`，同样 rc=2，且不写输出文件。
5. 诊断走 D6 通道：`src/plan.rs:326 collect_auto_conversion_diagnostics()` 把
   `Converted.warnings` 变成 `auto-converted <path>: <warning>`；转换器没告警时补一条
   “converted in memory (the source file was not modified)”，并加
   `NOTE_AUTO_CONVERTED`（`src/warnings/mod.rs:154`）。默认静音，
   `--verbose` 才经 `Warnings::emit(verbose)` 打印。`--plan` 的 stdout `note:` 契约未动。
6. 库 API：`plot_grid` / `plan_grid` 仍默认静音；新增
   `plot_grid_with_diagnostics`（`src/lib.rs:147`，R31 已有）与
   `plan_grid_with_diagnostics`（`src/plan.rs:222`）。CLI 的 plan 分支在
   `src/cli/execute.rs:78` 传入 `args.verbose`。
7. 截断改为优先在词边界（`src/plan.rs:367 clamp()`），避免告警以 `Fix: wr…` 这种半个单词结尾。

## 有意行为变更

- **未打标签输入：rc=2 → rc=0**，这是本任务的目标。
- 已打标签输入的输出不变：用 D4 前 HEAD `bddf2b7` 单独构建的二进制与当前二进制，
  对同一 `tests/fig2_mixed/out/tagged/A_heatmap.svg`（`--width 600 --ncol 1`）输出
  **逐字节相同**，SHA-256 都是
  `ca2f6503a18e58c52bb8991d6a8fcd97ee3d96e40e5e6c1f8b903552df51a59f`。
- 默认 stderr 仍为 0 字节；`--verbose` 才出现转换告警与 note，输出字节不变。
- 没有引入新依赖（未使用 `sha2`；证据脚本用标准库 `hashlib`，集成测试内置紧凑 SHA-256）。

## 验证数字

环境：`target/debug/svg_grid`，`--plot-margin 0`，宽 500/600/800。

| # | 输入 | 结果 |
|---|------|------|
| 1 | 未打标签 svglite（288×216） | rc=0；含 `data-svg-grid-cell="0"`；与“先 `svg_grid convert` 再 `plot-grid`”**逐字节相同** |
| 2 | 真实 matplotlib 3.11 `probe_imshow_cb.svg`（4661×809，11 个 glyph path） | rc=0；与手动 convert 产物**逐字节相同**；`cell 600×104.141` |
| 3 | 已打标签 `tagged/A_heatmap.svg` | rc=0；stderr 0 字节；55350 字节输出 SHA `ca2f6503…` = D4 前基线 |
| 4 | 未打标签 `<symbol>`（convert 判 Incompatible） | rc=2；无输出文件；stderr 含 `auto-conversion failed for`、`svg/symbol`、`svg_grid convert`；`--plan` 同样 rc=2 |
| 5 | 未打标签 + `--plan` | rc=0；stderr 0；`canvas 500x375`、`cell 500x375`、`s=1.736x1.736`；真跑输出 `width=500 height=375` |
| 6 | 源文件只读性 | 前后 bytes、SHA-256、mtime 全部相同；默认 stderr 0 字节；`--verbose` 800 字节且含 `auto-converted` + `svg_grid: note:`；两种模式输出字节相同 |
| 7 | 混合（已打标签 + 未打标签，2 列） | rc=0；两格 `data-svg-grid-cell` 为 0/1；`cell 649.654×112.759` + `150.346×112.759`，等高，宽度和 = 800 |

多子路径：`MULTI_SUBPATH` 自动转换后仍是展平多轮廓 path（≥3 个 `M` 子路径），
字形/孔洞没有被填成单轮廓；R28 的展平语义保持不变。

### 测试计数

- 根 crate：81 lib + 59 main + 92 selftest = **232**，0 failed。
- `convert`：88 lib + 12 cli = **100**，0 failed。
- `cargo fmt --all --check` 两个 crate rc=0；
  `cargo clippy --offline --all-targets -- -D warnings` 两个 crate rc=0。
- `./install.sh` rc=0；`svg_grid doctor` **57 pass / 0 fail / 1 skip**（matplotlib/mathtext）。
- 旧证据 R14/R28/R29/R30/R31 全部 rc=0；R31 的“默认静音 / `--verbose` 恢复”仍通过。
- R14 会串跑 R4–R13。D6 把告警改成默认静音后，这些更早的证据仍按“默认出声”断言，
  因此本次给 R4/R5/R6/R7/R8/R9/R13 的 `plot-grid` 调用显式补 `--verbose`（测试适配，
  不改产品行为）；R14 的“没有 panel-box ⇒ rc=2”分档改用“有 panel-box 但 canvas 宽 0”
  仍 rc=2，并新增一条“未打标签 ⇒ rc=0”的 D4 断言。R14 现为 `== 全部成立 ==`。
- 新增证据 `docs/PITFALLS/round-data/R32/test.py`：**38 PASS / 0 failure**。

新增集成测试（`tests/selftest/autoconvert/mod.rs`）：

- `untagged_svglite_matches_manual_conversion_byte_for_byte`（:119）
- `untagged_multi_subpath_panel_keeps_its_holes`（:133）
- `untagged_real_matplotlib_probe_matches_manual_conversion`（:147）
- `an_input_with_a_panel_box_is_never_auto_converted`（:161）
- `unsupported_untagged_input_fails_closed_without_output`（:198）
- `plan_on_an_untagged_input_matches_the_real_run`（:226）
- `auto_conversion_is_silent_by_default_and_leaves_the_source_untouched`（:256）
- `mixed_tagged_and_untagged_inputs_share_one_grid`（:313）
- `the_local_sha256_matches_the_standard_abc_vector`（:445，校验测试内哈希实现）

没有修改任何既有 cargo 测试/快照；仅适配了旧 round-data 证据（R4–R9/R13 补 `--verbose`、R14 更新分档）。

## 改动文件

- `src/geom.rs:31`：新增 `find_panel_box_opt`。
- `src/plan.rs:125-210`：`prepare` 自动转换分支；`:215` / `:222` plan 包装与诊断；
  `:266-320` auto-convert + 错误/ findings 描述；`:326-376` 诊断发射与截断。
- `src/lib.rs:174`：组合路径接入自动转换诊断；`:24` 导出 plan 诊断入口。
- `src/cli/execute.rs:78`：`--plan` 传 `args.verbose`。
- `src/warnings/mod.rs:154`：`NOTE_AUTO_CONVERTED`。
- `tests/selftest/main.rs`：声明 `mod autoconvert;`。
- `tests/selftest/autoconvert/mod.rs`：新增 9 条测试。
- `docs/PITFALLS/round-data/R32/`：`test.py` + `meta.toml`。
- `docs/PITFALLS/round-data/R4,R5,R6,R7,R8,R9,R13/test.py`：补 `--verbose`，适配 D6 默认静音。
- `docs/PITFALLS/round-data/R14/test.py`：结构坏的分档改用零宽 canvas；新增未打标签自动转换断言。

## 非阻断遗留

- 自动转换告警是转换器原文，超过 200 字符时按词边界截断；完整原因仍可用
  `svg_grid convert --check --report` 查看。
- 自动转换只处理“完全没有 panel box”的输入；已有 panel box 但几何不完整的输入仍按
  原有缺 box/画布错误处理，不做二次猜测（符合“已有 box 一律不转”的要求）。
- `convert` 的 `--verbose` 与 `plot-grid` 的 `--verbose` 是两套入口，自动转换路径只复用
  转换器返回的 warnings，不调用 convert CLI，因此不会产生 convert 的 stderr 前缀。
