# R34：W8 把 `--label-fontfamily` 当单族名（回退链误判）+ 硬错误行超 200 字符

## 现象

1. **回退链被误判为缺族**：`--label-fontfamily "Helvetica, Arial, sans-serif"` 在
   `Arial` 明明在库的情况下 rc=3、不写产物。同理 `"Liberation Sans, Arial"`、
   `"serif, NoSuch"` 也 rc=3；只有**链首**在库（`"Arial, Liberation Sans, sans-serif"`）
   才 rc=0——正好说明实现只看链首。
2. **硬错误行超长**：W3 重复加字母的错误行 205 字符、W8 缺族 239 字符，
   违反 `ARCHITECTURE/warnings.md:25` 的「单条 ≤ 200 字符（含前缀）」。
3. **账本行号漂移**：R33-F4 记的 `src/lib.rs:152-236` 只覆盖
   `plot_grid_with_diagnostics` 的前半段（函数实际 152-325），它描述的 spacer 分支在 273-282。

## 复现（D5b 前，HEAD `eac0dee`）

```sh
SRC=tests/user_tests/2026-11-20/pandamap_ALB_A.svg
# ① 链首缺、链尾在库 ⇒ 旧实现 rc=3
svg_grid plot-grid --input $SRC --output /tmp/a.svg --width 600 --ncol 1 \
  --labels --label-fontfamily "Helvetica, Arial, sans-serif"; echo rc=$?
# ② 通用族在链里 ⇒ 旧实现 rc=3
svg_grid plot-grid --input $SRC --output /tmp/b.svg --width 600 --ncol 1 \
  --labels --label-fontfamily "serif, NoSuch"; echo rc=$?
# ③ 行长度
svg_grid plot-grid --input $SRC --output /tmp/c.svg --width 600 --ncol 1 \
  --labels --label-fontfamily NoSuchFont_xyz 2>&1 | awk '{print length($0)}'   # 239
```

## 根因

- `src/warnings/fonts.rs:44 describe_missing_font_error` 用
  `names.iter().position(|name| !is_generic(name))` 取**第一个非通用名**，
  再只判断这一个名字是否在库：`if has(available, missing) { return None; }`。
  CSS `font-family` 是**回退链**（`ARCHITECTURE/fonts.md:18`、`src/cli/help.rs:70-72`、
  `docs/general-figure-guide/references/01_canvas_layout.md:19` 三处契约都这么写），
  第一个**可用**者生效——所以链首缺失不该判死。
- 硬错误不走 `src/warnings/mod.rs` 的 `fit` / `assemble`（它们按
  `MAX_BODY = MAX_LINE - "svg_grid: warning: "` 预算），而是直接 `format!`；
  但硬错误打印时的前缀更长（`svg_grid: invalid input: ` / `svg_grid: font library error: `），
  于是 205 / 239 字符越界。

## 修法

- `src/warnings/fonts.rs:52 describe_missing_font_error`：先
  `if names.iter().any(|name| is_generic(name) || has(available, name)) { return None; }`
  ——链里**任一**名字可用（在库或通用族）即放行；只有整条链都不可用才报错。
  报错文案点名**整条链**、列出可用族、给 `--list-fonts` / `fontlib/` 修法，
  并按 `MAX_ERROR_BODY` 逐级收缩（先减可用族条数 `MAX_LISTED → 3 → 1`，
  再 `clamp_quoted` 把链回显截到最后一个逗号边界并加 `…`），保证单行 ≤ 200。
  新增 `:104 HEAD_FIXED`、`:107 list_families`、`:126 clamp_quoted` 三个私有助手。
- `src/warnings/mod.rs:122 MAX_ERROR_BODY = MAX_LINE - "svg_grid: font library error: ".len()`
  （=170，最长的一档前缀）；`:257 assemble_within(head, detailed, plain, tails, budget)`
  把 `assemble` 的预算参数化，`assemble` 仍用 `MAX_BODY`。
- `src/warnings/labels.rs:19 describe_existing_labels` 改用
  `assemble_within(..., super::MAX_ERROR_BODY)`，尾句压成
  ` already carry letters (data-svg-grid-label); lettering again duplicates them. Fix: --labels only at the layer that owns the panels.`
  （205 → 188 字符，8 个 offender 折叠时同样 188）。
- `src/warnings/labels.rs:45 describe_moved_label`（W5）同步压到 191 字符。
- `src/cli/doctor/run.rs:397` 的 W3 码词从 `"already contain panel letters"` 改为
  真实文案 `"already carry letters"`（旧词早已对不上，`classify` 会漏判 W3）；
  `src/warnings/rules_tests.rs:125` 的短语表同步。
- `font-family` 字符串**不重写**：`src/labels.rs:26` 仍原样写用户给的链，
  由渲染器自己做同样的回退（父 agent 的 `aa5693c` 也钉了这点）。

## 验收数字

**A. 四条回退链（`pandamap_ALB_A.svg`，`--width 600 --ncol 1 --labels`）**

| `--label-fontfamily` | D5b 前 | D5b 后 |
|---|---|---|
| `Helvetica, Arial, sans-serif` | rc=3, 235 B | **rc=0, 0 B** |
| `Liberation Sans, Arial` | rc=3, 241 B | **rc=0, 0 B** |
| `serif, NoSuch` | rc=3, 232 B | **rc=0, 0 B** |
| `Arial, Liberation Sans, sans-serif` | rc=0, 0 B | rc=0, 0 B |

`font-family="Helvetica, Arial, sans-serif"` 原样出现在输出里；把 `font-family`
归一化后与 `--label-fontfamily Arial` 的输出**逐字节一致**（几何不受链影响）。

**B. 整条链都不可用（rc=3、不写产物、单行 ≤200）**

| 链 | rc | 行长度 |
|---|---|---|
| `NoSuchFont_xyz` | 3 | 194 |
| `Helvetica, Liberation Sans, DejaVu Sans` | 3 | 195（链回显截为 `"Helvetica, Liberation Sans, …"`） |
| `A,B,C,D,E,F,G,H` | 3 | 195 |

文案（第一条，逐字）：

```text
svg_grid: font library error: --label-fontfamily "NoSuchFont_xyz" is not in the font library (available: Arial, Times New Roman); no family in the chain is usable. Fix: --list-fonts or fontlib/.
```

**C. W3 行长度**：205 → **188**（`--plan` 同样 rc=2、stdout 0 字节）：

```text
svg_grid: invalid input: panels in input #0 (letter "C") already carry letters (data-svg-grid-label); lettering again duplicates them. Fix: --labels only at the layer that owns the panels.
```

**D. W5 行长度**：217 → **191**（`--plot-margin 0` 内移说明）。

**E. 无 `--labels` 的输出不变**：7 个 fixture
（`tests/fig2_mixed/out/tagged/{A_heatmap,B_A,B_B,B_C,B_D}.svg`、
`tests/user_tests/2026-11-20/{pandamap_ALB_A,ABCDE}.svg`）与 `eac0dee` 的 release 构建
**逐字节一致**。

## 有意行为变更

1. **W8 从「链首非通用名」改为「整条链都不可用才报错」**：链首缺、链尾在库的调用
   由 rc=3 变 rc=0（这是修 bug，不是放宽判据——契约本来就写的是回退链）。
2. **W3/W8/W5 文案压缩**：语义不变，只是把「怎么修」写短，以守住 200 字符。
   W3 的 `already carry letters`、W8 的 `is not in the font library` 两个码词保留
   （doctor 的 `WARNING_CODES` 与 R13/R33 证据都靠它们）。
3. **W4a（`--labels` 条数与输入数不符）仍是警告 rc=0**：`INSTRUCTIONS.md:56-58`
   的「必要信息」只列了非法 ncol/nrow、重复字母、缺字体，长度不符不在其中。

## 测试

新增：

- `src/warnings/rules_tests.rs:81 a_font_chain_is_satisfied_by_any_usable_family`
  （可用链含空串；死链含 6 名；断言 `is not in the font library` /
  `no family in the chain is usable` / `available:` / `--list-fonts` /
  `WARNING_PREFIX.len() + body.len() <= MAX_LINE`；空库 ⇒ `available: none`）
- `tests/selftest/fonts/mod.rs:186 a_fallback_chain_wins_with_its_first_available_family`
  （`Helvetica, Arial, sans-serif` rc=0、stderr 0、`font-family` 原样；`serif, NoSuchFont_xyz` rc=0）
- `tests/selftest/fonts/mod.rs:271 a_wholly_unavailable_chain_is_a_hard_font_error`
  （rc=3、点名整条链、`available:`、`--list-fonts`、每行 ≤200、不写产物）

改写：`src/warnings/tests.rs` 的 W3 单测断言新文案；`src/cli/doctor/run.rs` 与
`src/warnings/rules_tests.rs` 的 W3 码词。

计数：根 crate **81 lib + 59 main + 96 selftest = 236**；convert **88 lib + 12 cli = 100**。
`cargo fmt --all --check` 两 crate rc=0；clippy `-D warnings` 两 crate rc=0；
`./install.sh` rc=0；`svg_grid doctor` **57 pass / 0 fail / 1 skip**。

旧证据：R8/R9/R10/R13/R14/R28/R29/R30/R31/R32/R33 全 **rc=0**；R34 **38 PASS, 0 failure**。
R16 的 `compose()` 与 R25 的 `plot()` 补 `--verbose`（D6 起告警默认静音，这两个脚本判的是告警文案），
R16 62 PASS / 0 FAIL / 3 SKIP，R25 全绿。

## 遗留（非阻断）

- `docs/general-figure-guide/references/02a_svg_grid_assembly.md` 已 19972 B（R8 上限 20000），
  本轮未改；若要把「回退链」写进指南，需先拆分该文件。
- W5 仍对每个内移字母各发一条（R33 遗留，未变）。
- 空行网格错误行 244 字符（R31 遗留，任务书明确不动）。
