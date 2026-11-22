# R35：W8 硬错误行在病态输入下仍超 200 字符（D5b 的两个 off-by-one / 未截断）

## 现象

D5b 把 W8 文案压到 ≤200 后，短链确实合规（194/195/195），但**病态输入仍越界**，
且越界量恰好是 1 个字符——典型的 off-by-one：

1. **长单名 ⇒ 201 字符**。`--label-fontfamily` 传 81..86 个 `X` 时整行 201 字符；
   80 个 `X` 恰好 200（阈值就在 80/81 之间）。真实族名
   `"Noto Sans CJK JP Display Condensed Medium Oblique Extra Light Version Two Point Zero"`（84 字符）同样 201。
2. **字体库里是长族名 ⇒ 201 字符**。用 fontTools 造 nameID 1/4/6/16 全为 `L`*n 的 ttf
   （n=80/90/100）放进 `--font-dir`，`available:` 列表按整名拼接，整行 201；
   族名更长时（评审实测）到 213。

## 复现（D5b 后、D5c 前，HEAD `4d737b2`）

```sh
SRC=tests/user_tests/2026-11-20/pandamap_ALB_A.svg
# ① 长单名：X*80 ⇒ 200，X*81..86 ⇒ 201
svg_grid plot-grid --input $SRC --width 300 --ncol 1 --labels \
  --label-fontfamily "$(python3 -c 'print("X"*81)')" --output /tmp/x.svg 2>&1 | awk '{print length($0)}'
# ② 长可用族名：n=80/90/100 全部 201
svg_grid plot-grid --input $SRC --width 300 --ncol 1 --labels \
  --font-dir /tmp/d5c/long --label-fontfamily NoSuchFont_xyz --output /tmp/x.svg 2>&1 | awk '{print length($0)}'
```

## 根因

1. `src/warnings/fonts.rs` 的 `clamp_quoted` 用 `keep = budget - 4`，但包装
   `"{shortened}, …"` 实际多出 **5** 个字符（两个引号 + `, ` + `…`），
   于是返回 `budget + 1`。FINAL 兜底分支只 `format!` 拼装、**没有长度校验**，
   body 变成 171 > `MAX_ERROR_BODY`(170) ⇒ 整行 201。
2. `list_families` 按整名 `join(", ")`，FINAL 分支只 clamp 链回显、**不 clamp
   `available:` 列表**；库里的族名本身很长时，`fixed` 部分就把预算吃光。

## 修法

`src/warnings/fonts.rs`（函数体全部重写，`describe_missing_font_error` 现 52-118）：

- **`clamp_quoted` 改成 `keep = budget - 5`**，并在返回前再校验一次：
  若 `"{shortened}, …"` 仍超 `budget`，退到 `"…"`；`shortened` 为空也退到 `"…"`。
  这样它自身就保证「返回值 ≤ budget」，不再依赖调用方。
- **兜底分支也校验**：新增 `with_clamped_chain(chain, fixed) -> Option<String>`，
  它先算 `budget = MAX_ERROR_BODY - PREFIX - MIDDLE - fixed`，`budget < MIN_ECHO(3)`
  直接返回 `None` 让调用方继续降级；拼好后仍用 `fits()` 复核。
  最终兜底 `with_clamped_chain(&chain, "; Fix: --list-fonts.")` 之后还有一层
  `.or_else(|| Some(format!("{PREFIX}\"…\"{MIDDLE}; Fix: --list-fonts.")))`，
  保证**任何输入**都能返回一条 ≤170 body 的行。
- **`available:` 列表也能截断**：新增 `list_families_within(available, shown, budget)`，
  逐个族名按剩余预算截断（保留可辨识前缀 + `…`），放不下就停；
  调用前用 `CHAIN_ECHO = 24` 给链回显预留位置，避免「长库把链挤成 `"…"`」。
- **降级阶梯重排**：先试「完整链 + 完整库（6→3→1 个族）」，再试「clamp 链 + 库
  （1→3→6 个族，短库优先给链让位）」，最后试「clamp 链 + 截断族名」。
  每一档都保留 `is not in the font library`（doctor 的 W8 码词）、`available:`、
  `--list-fonts`（可操作性）。

## 验证数字（`target/release/svg_grid`，`--width 300 --ncol 1 --labels`）

| 输入 | D5b | D5c |
|---|---|---|
| `X`*78 / 80 | 200 | 200 |
| `X`*81 / 86 / 200 | **201** | **200** |
| 长族名库 L80 / L90 / L100 | **201** | **198** |
| 6 个 100 字符族名的库 | — | 198 |
| 200 个名字的链 | — | 198 |
| `NoSuchFont_xyz` | 194 | 194（不变长） |
| `Helvetica, Liberation Sans, DejaVu Sans` | 195 | 195 |
| `A,B,C,D,E,F,G,H` | 195 | 195 |

长单名的回显保留可辨识前缀（`"XXXXXXXXXXXXXXXXX, …"`，≥20 个 X）；
长族名库的链回显完整（`"NoSuchFont_xyz"`），族名截为 `LLLL…`。

回退链语义未回退：`Helvetica, Arial, sans-serif` / `Liberation Sans, Arial` /
`serif, NoSuch` / `Arial, Liberation Sans, sans-serif` / `Arial, NoSuch` /
`NoSuch, Arial` / `arial` / ` Arial , sans-serif ` / `""` / `,,` /
`"Times New Roman"` 全部 rc=0、stderr 0 字节。失败一律 rc=3 且不写产物。

## 测试

- `src/warnings/rules_tests.rs:141 a_pathological_font_chain_still_fits_the_error_cap`：
  5 组病态输入（X*200、X*81、长库、200 名链、长库+长链）断言 `prefix + len <= MAX_LINE`
  且仍含三个可操作片段；另断言长单名回显 ≥20 个 X、长族名被 `…` 截断。
- `tests/selftest/fonts/mod.rs:325 a_pathologically_long_chain_still_fits_the_error_cap`：
  端到端跑 X*81 / X*200 / 200 名链，断言 rc=3、每行 ≤200、不写产物。
- **变异测试**：把 `keep = budget - 5` 改回 `-4` ⇒ 单元测试红（`rules_tests.rs:184`）；
  把 `list_families_within` 直接 `return None` ⇒ 单元测试红（`rules_tests.rs:171`）。
  两处改回后复绿。

## 验证

根 crate 82 lib + 59 main + 97 selftest = 238；convert 88 lib + 12 cli = 100；
`cargo fmt --all --check` 与 `cargo clippy --all-targets -- -D warnings` 两 crate 干净；
`./install.sh` rc=0；`svg_grid doctor` 57 pass / 0 fail / 1 skip；
R8/R9/R10/R13/R14/R28–R34 证据全 rc=0；R35 证据 53 PASS。

## 遗留（非阻断）

1. `docs/general-figure-guide/references/02a_svg_grid_assembly.md` 已 19972 B（R8 上限 20000），
   要把「回退链」写进指南需先拆分该文件。
2. W5 仍对每个内移字母各发一条（R33 遗留）。
3. 空行网格错误行 244 字符（R31 遗留，任务书明确不动）。
4. W4a（`--labels` 条数与输入数不符）仍是警告 rc=0，按评审裁决未升 rc=2。
