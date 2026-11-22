# R33：`--labels` 的 NULL 规则 + W3/W8 升硬错 + `--plot-margin 0` 字母被裁

## 现象

1. **NULL 槽不画显式字母**：`A,B,NULL,C,D` 传 `--labels A,B,X,,E` 时，槽 2 是 NULL，
   旧实现先在 `src/lib.rs` 的循环里 `continue`，`X` 永远不会被画出来。
2. **重复加字母只是警告**：已带 `data-svg-grid-label` 的组合图再传 `--labels C`，
   旧实现 rc=0、照写输出，只在 stderr 警告 W3；`--plan` 根本不校验。
3. **缺字体族只警告**：`--label-fontfamily NoSuchFont_xyz` 在 SVG 输出上 rc=0，
   `<text>` 仍写着不存在的族名，靠渲染器静默回退；PNG/AVIF 才可能失败。
4. **`--plot-margin 0` 首行字母被裁**：默认 `vjust=-0.25` 把 baseline 放到 cell 顶上方
   0.25em；margin 0 时 `y=-6.757`（或 5 图 `-3.378`）落在画布外，字母被裁掉。

## 复现（D5 前）

```sh
# ① NULL 槽：X 不出现
svg_grid plot-grid --input A.svg --input B.svg --input NULL --input C.svg --input D.svg \
  --ncol 2 --labels A,B,X,,E --output /tmp/x.svg

# ② 已带字母的 row 再标一次：rc=0，只警告
svg_grid plot-grid --input row.svg --ncol 1 --labels C --output /tmp/y.svg

# ③ 缺族：rc=0，SVG 照写
svg_grid plot-grid --input A.svg --labels --label-fontfamily NoSuchFont_xyz --output /tmp/z.svg

# ④ margin 0：字母 y 为负
svg_grid plot-grid --input A.svg --labels --plot-margin 0 --output /tmp/m.svg
grep 'data-svg-grid-label' /tmp/m.svg   # y="-6.757"
```

## 根因

- `src/cli/execute.rs:151 parse_labels` 的 AUTO 分支本身已跳过 spacer；但
  `src/lib.rs` 的 `prepared` 循环对 `None` 先 `continue`，所以**显式**列表里给 NULL 槽的
  非空字母没有机会走 `labels::build`。
- W3 在 `collect_warnings` 里、输出写完之后才 `warnings.add`；
  W8 同理走 `describe_missing_font` 警告，二者都不影响 rc。
  `--plan` 分支（`src/cli/execute.rs:77`）只调 `plan_grid_with_diagnostics`，
  没有 `LabelSpec`，因此 W3 无法在计划阶段触发。
- `src/labels.rs:25` 的 baseline 直接等于 `cell.y + (1-y)*h + vjust*size`，没有画布夹取；
  `label_ink_box` 用同一公式，因此 W5 只能报告“可能被裁”，不能避免。

## 修法

- `src/labels.rs:45 raw_baseline`（旧公式）与 `:57 baseline`：
  `raw.max(LABEL_CAP_HEIGHT_RATIO * size)`（0.75em）。`build` 与 `label_ink_box` 都用
  `baseline`，所以字母 ink 至少落在画布内；`--plot-margin` 仍被尊重，只是放不下时内移。
- `src/lib.rs:171` 在 `prepare` 之后、打开输出之前调用
  `validate_label_conflicts(&prepared, labels)`；新模块 `src/label_conflicts.rs:13`
  收集 `(index, letter)`（非空字母 + 输入已含 `data-svg-grid-label`），
  返回 `SvgGridError::InvalidInput`（rc=2）。
- 同一处 `src/lib.rs:175`：`spec.font_families` 存在且任一字幕非空时，
  `describe_missing_font_error` 返回 `SvgGridError::Fonts`（rc=3）。
- `src/lib.rs:273`：spacer 分支不再直接 `continue`；若显式列表给该槽非空字母，
  仍用 `labels::build(index, cell, spec.width, labels)` 画在空格子左上角。
- `src/warnings/labels.rs:19 describe_existing_labels` 改为接收 `(usize, String)`，
  文案点名 `input #0 (letter "B")` 并给修法；`:44 describe_moved_label` 生成 W5 内移说明。
- `src/warnings/fonts.rs:44 describe_missing_font_error` 列出最多 6 个可用族，
  点名缺族并给 `--list-fonts` / `fontlib/` 修法。
- `src/plan.rs:230 plan_grid_with_labels` + `:238 plan_grid_checked`：
  `--plan` 先跑 W3 校验，失败不打印计划；`src/cli/execute.rs:50` 把 `LabelSpec`
  构造提前到 plan 分支之前，`:89` 的 plan 调用带 `label_spec.as_ref()`。
- W3/W8 从 `collect_warnings` 移除；W5 若发生内移，先发 `describe_moved_label`，
  再按实际（已夹取）ink 检查是否仍有裁切。

## 验收数字

**A. NULL 槽（5 槽：0,1 真图，2=NULL，3,4 真图；1000×800，ncol 2）**

| 调用 | slot 0 | 1 | 2 (NULL) | 3 | 4 |
|---|---|---|---|---|---|
| 裸 `--labels` | A | B | — | C | D |
| `--labels A,B,X,,E` | A | B | **X** | — | E |

AUTO 不重号、不跳号；显式 `""` 不画字母。字母坐标仍由 cell 左上角与现有
`x/y/hjust/vjust` 决定（例：1000×800、ncol 2 时槽 0 的 baseline 20.622）。

**B. W3**

```text
$ svg_grid plot-grid --input row.svg --ncol 1 --labels C --output /tmp/retag.svg
rc=2, no /tmp/retag.svg
svg_grid: invalid input: panels in input #0 (letter "C") already carry letters
(data-svg-grid-label); lettering them again would duplicate them.
Fix: use --labels only where these panels are direct inputs.

$ svg_grid plot-grid --input row.svg --plan --labels C
rc=2, stdout=0 字节
```

`--labels ,C` 对同一 row 仍 rc=0（空串槽不冲突）。`--plan` 与真跑同一错误文案。

**C. W8**

| 输出 | rc | 产物 |
|---|---|---|
| `--output x.svg` + `NoSuchFont_xyz` | 3 | 不写 |
| `--output-png x.png` + 同族 | 3 | 不写 |
| `--plan` + 同族 | 0 | 正常打印计划（不读字体） |

文案：`--label-fontfamily "NoSuchFont_xyz" is not in the font library
(available: Arial, Times New Roman). Fix: use a family from --list-fonts, ...`。

**D. `--plot-margin 0`（5 图 ncol5，500×160）**

旧：5 个字母 y=-3.378，全部在画布外。
新：5 个字母 y=10.135（=0.75×13.514），x=0/100/200/300/400，全部可见；
`--verbose` 对每个字母发一条
`panel letter "A" (cell 0) was moved inside the canvas: its baseline -3.378 is above
the canvas, so it was clamped to 10.135. Fix: omit --plot-margin ...`。

## 有意行为变更

1. **W3 重复加字母：警告 rc=0 → 硬错误 rc=2、不写输出**；`--plan` 同样失败。
2. **W8 缺标签字体族：SVG 静默回退 → 所有输出 rc=3、不写输出**；`--plan` 不读字体，保持成功。
3. **AUTO 继续跳过 NULL**（不是新变更，但本轮把它钉进测试）；显式列表可给 NULL 槽画字母。
4. **`--plot-margin 0` 字母内移**：不再被裁；W5 文案改为“moved inside the canvas”。

## 体积回退（D5 连带面）

D5 改动让 `src/lib.rs` 达到 20182 B、`tests/selftest/warnings/mod.rs` 达到 20781 B，
违反 R8 的 20000 B 底线。处理：

- 新增 `src/label_conflicts.rs`（W3 校验与 `contains_data_svg_grid_label`）；
  `src/lib.rs` → 18914 B。
- 新增 `tests/selftest/warnings/ink.rs`（W6/W5 三条 ink 测试），
  `tests/selftest/warnings/mod.rs` → 14156 B；`ink.rs` 6801 B。
- 全仓源码最大 `src/layout.rs` 19069 B；R8 rc=0（144 个文件）。

## 测试

新增/改写：

- `tests/selftest/labels/mod.rs:291 auto_labels_skip_null_and_explicit_labels_can_fill_the_empty_cell`
- `tests/selftest/labels/mod.rs:383 margin_zero_moves_every_letter_inside_the_canvas`
- `tests/selftest/fonts/mod.rs:125 a_missing_label_family_is_a_hard_font_error`
- `tests/selftest/warnings/mod.rs:246 warns_when_relettering_an_already_lettered_composite`
  （改为 rc2 + plan + 空串槽放行）
- 单元：`src/warnings/tests.rs:existing_label_error_names_the_input_and_letter`；
  `src/warnings/rules_tests.rs` 的 W3 输入改 `(index, "A")`。

计数：根 crate **80 lib + 59 main + 94 selftest = 233**；convert **88 lib + 12 cli = 100**。
旧证据：R8/R9/R10/R13/R14 全 **rc=0**，R28–R32 全 PASS；R33 **19 PASS, 0 failure**。
`cargo fmt --all --check` 两 crate rc=0；clippy `-D warnings` 两 crate rc=0；
`./install.sh` rc=0；`svg_grid doctor` **57 pass / 0 fail / 1 skip**。

## 遗留（非阻断）

- 任务书写“显式 `--labels` 长度不符仍 rc=2（现有 fail-closed）”，但 HEAD 的实际行为是
  W4a 非致命警告 rc=0（`describe_label_count`）。本轮按“沿用现有”未改；若要求升为 rc=2，
  需要再动 W4a 及其证据 R9，属另一条行为变更。
- `docs/general-figure-guide/` 中“分步组图时在中间层加字母”的段落未改（按任务只报告）；
  需要 Arranger 同步为“字母只在拥有面板的那一层加”。
- W5 现在对每个内移字母各发一条；若希望按 cell 聚合为一条，可后续统一。
