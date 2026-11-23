# svg_grid 阻塞点 — 两个已带字母的组合图直接组合，产物重复字母且 rc=0（R44）

来源报告：`docs/PITFALLS/blocker-svg_grid-duplicate-letters-across-composites.md`
（`727f45d` 上仍可复现）。base = `727f45d`，修复后 `code_after` 见账本。

## 现象

`--labels` 的 W3 只在"**本次调用**给非空字母、而输入已带字母"时硬错误。两个输入**各自**带字母
（`ab.svg` = A,B，`c.svg` = A），本次不给 `--labels`，产物里同时出现两个
`data-svg-grid-label="0"`，渲染为 **A、B、A**，但退出码 **0**、stderr **0 字节**（`--verbose`
下也没有字母相关告警）。这正是 `docs/general-figure-guide/references/02a_svg_grid_assembly.md:63`
承诺"不再静默重号"的另一半。

## 复现（`docs/PITFALLS/round-data/R44/test.py` 是常驻版本）

```bash
mkdir -p /tmp/dup && cd /tmp/dup && cp p.svg p2.svg; cp p.svg p3.svg
svg_grid plot-grid --input p.svg --input p2.svg --width 600 --ncol 2 --labels --output ab.svg
svg_grid plot-grid --input p3.svg --width 600 --ncol 1 --labels --output c.svg
svg_grid plot-grid --input ab.svg --input c.svg --width 800 --ncol 2 --output abc.svg
```

| | 修前（`727f45d`） | 修后 |
| --- | --- | --- |
| rc | 0 | **2** |
| stderr | 0 字节 | `svg_grid: invalid input: duplicate panel letters: "A" at #0 (cell 0) and #1 (cell 1). Fix: …`（152 字符） |
| 产物 | `data-svg-grid-label` = 0,1,0；可见 A,B,A | **不写**；既有文件 9 字节原样未截断 |
| `--plan` | rc=0 且打印计划 | **rc=2、stdout 空、同文案** |

同一输入内部重号：`"A" 2 times inside #0 (cell 0)`，rc=2。
真语料：`tests/fig2_mixed/out/tagged/A_heatmap.svg`+`B_A.svg`（A,B）与 `B_B.svg`（A）组合 ⇒ rc=2。

## 根因

字母的来源有两层：输入自带（`data-svg-grid-label`，随输入子树原样进产物）与本调用 `--labels`
分配的。旧代码只校验第二层会不会覆盖第一层（W3），从没把"最终产物里出现的字母集合"当成一个
整体查重——于是"各自已加字母"的两块组合图拼起来，每个字母各出现两次也没人管。

修复后判定：收集**最终出现的字母**（每个输入的子树里所有 marker 的字母文本 + 本调用给非空
字母的 cell），按字母分组；同一字母出现 ≥2 次且**至少一处来自输入**⇒ 硬错误。只有全部来自
显式 `--labels` 时维持 W4b 警告（用户自己写重，rc=0）——两者语义不同，未顺手升级。

## 修法

- `src/label_conflicts.rs`：`validate_label_conflicts` 改为接收 `Option<&LabelSpec>`（无
  `--labels` 也要查），W3 原样保留在前；新增 `collisions()`（收集 + 分组 + 只保留"至少一处
  继承"的冲突）、`collect_letters()`（递归收集 marker 的字母文本）。
- `src/warnings/labels.rs`：新增 `LetterSite` 与 `describe_duplicate_panel_letters()`，文案
  点名字母、每个 input# 与 cell，并给修法（"letter only at the outer layer, or pass unique
  --labels here"）；`assemble_within` 折叠保证 ≤200 字符（实测 152）。
- 调用点：`src/lib.rs:plot_grid_with_diagnostics` 提前无条件调用；`src/plan.rs:plan_grid_checked`
  去掉 `if let Some(labels)` 包裹 ⇒ `--plan` 与真实渲染同 rc、同文案、同样不写任何东西。
- 错误在打开输出路径**之前**抛出，既有产物不会被截断。

## 变异验证

把 `collisions()` 的过滤条件改成 `false`（即"永不报重号"）：单元测试
`separately_lettered_composites_are_rejected`、`duplicates_inside_one_input_are_rejected`、
`an_explicit_letter_that_collides_with_an_inherited_one_is_rejected` 立刻变红；集成测试
`cli_rejects_duplicate_letters_across_composites` 的 A1/A4 段与 `--plan` 段变红。fail-closed
段在 `727f45d` 二进制上复现 rc=0 + 空 stderr + 0,1,0（R44 脚本 F1）。

## 验证

- `cargo test --offline`：228 lib + 59 main + 128 selftest = **415**，0 fail（R43 406，+9）。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` rc=0；`svg_grid doctor` = **57 pass / 0 fail / 1 skip**（doctor 的 fixture 不带
  字母，未被误伤）。
- 行为等价：220 `tests/**/*.svg` 的 `convert` 产物与 base 逐字节相同（179 成功 / 41 双方失败 /
  changed 0）；58 `tests/fixtures/**` 的 `plot-grid --width 600` 产物 0 字节差、rc 0 差、
  `--plan` 0 差；`convert --report` 在相同路径下 stdout/stderr/rc 逐字节相同。
- 误伤面（R44 脚本 C1–C6 + 单测）：A,B + C 合法；A,B + 无字母输入合法；单个组合图单独跑合法；
  两个无字母输入 + `--labels A,B` 走原路径；`--labels A,A` 仍 rc=0 警告（W4b）；`--labels ,C`
  放行；W3 的"already carry letters"文案不变。
- 旧证据 R8,R9,R15,R16,R17,R18,R20,R25,R28,R31,R35,R36,R37,R38,R39,R40,R41,R42,R43 顶层全
  rc=0 / 0 FAIL；`pitfalls-manager verify` rc=0。

## 遗留（未修）

- W4b（`--labels A,A`）仍是警告 rc=0；如果将来要统一成错误，需要另开一轮并处理"用户显式
  写重"与"跨层意外撞号"的文案区分。
- `--labels none` 遇到输入已带重复字母时同样 rc=2（本轮口径：最终字母集合查重，与是否显式给
  字母无关）——报告在案，未写进向导文档（该文件由 Arranger 维护）。
- `docs/general-figure-guide/references/02a_svg_grid_assembly.md:63` 的措辞可以进一步区分
  "本次给字母"与"跨输入撞号"；按要求只在报告里建议，未改文件。
