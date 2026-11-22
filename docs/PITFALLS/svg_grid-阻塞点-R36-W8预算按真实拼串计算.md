# svg_grid 阻塞点 R36 — W8 硬错误行的预算必须量真实拼串

## 现象

D5c（R35）把 W8 硬错误压到 ≤200 后，评审用两组长名输入复现出**两个新阻断**，同一根因：预算用手算常量而不是量真实拼串。

### 阻断 1 — `available:` 整段丢失（99 字符）

```bash
svg_grid plot-grid --input /tmp/d5c_eval/panel.svg --output /tmp/x.svg --width 300 --ncol 1 \
  --labels --font-dir /tmp/d5c_eval/repro_lib --label-fontfamily X
```

- R35（`5482421`）：**99 字符**、无 `available:`、rc=3
- 期望：含 ` (available: `、≤200

触发面：链回显长度 = 3 **且** 首个族名 ≥ 67 字符。

### 阻断 2 — 链回显被压成 `"…"`

```bash
svg_grid plot-grid --input /tmp/d5c_eval/panel.svg --output /tmp/x.svg --width 300 --ncol 1 \
  --labels --font-dir /tmp/d5c_eval/fonts_noto7 --label-fontfamily Noto
```

- R35：`--label-fontfamily "…" is not in the font library (available: Noto Sans CJK HK Display Condensed …, …)`（199 字符）
- 期望：回显 `"Noto"`、≤200

## 复现步骤

1. `cd tools/svg_grid && cargo build`
2. 上面两条命令（`repro_lib` = 7 个 100 字符族名；`fonts_noto7` = 7 个真实 Noto CJK 长名）。
3. 或直接 `python3 docs/PITFALLS/round-data/R36/test.py`（① ② 两段就是这两条）。

## 根因

`src/warnings/fonts.rs` 的降级阶梯里，第三段的预算写成：

```rust
let overhead = PREFIX.len() + MIDDLE.len() + "(available: ".len() + suffix.chars().count();
```

但 `fixed` 里真实的前缀是 `" (available: "`（**13** 字符，含前导空格），代码按 **12** 算 ⇒ `overhead` 少 1 ⇒ 列表预算多 1 ⇒ `list_families_within` 返回的列表长 1 ⇒ `with_clamped_chain` 的 `budget = chain_room - 1 = 2 < MIN_ECHO(3)` ⇒ 返回 `None` ⇒ 整条阶梯落空 ⇒ 落到「最短兜底」（不含 `available:`）。

阻断 2 是同一处：链回显长度 4..24 时，上面多出的 1 字符同样把 budget 压到 `MIN_ECHO` 以下，于是 `clamp_quoted` 只能给出 `"…"`；`CHAIN_ECHO = 24` 的预留因此形同虚设。

第二个缺陷是结构性的：`clamp_quoted` 末尾的复核分支在算术正确时**不可达**，所以 D5c 的测试没能杀死它，评审删掉它也全绿。

## 修法

1. **不再手算常量**：`" (available: "` 提为 `const AVAILABLE_OPEN`（带注释说明前导空格正是上次丢列表的原因），并让每一级的预算都用 `format!` 拼出的 `fixed.chars().count()` 反推：

   ```rust
   let room = super::MAX_ERROR_BODY
       .saturating_sub(head + AVAILABLE_OPEN.len() + suffix.chars().count());
   ```

2. **链回显优先于库列表**：第三段先给回显 `min(链长, CHAIN_ECHO)`，只有放不下才退 `MIN_ECHO`；库列表按剩余预算截断单个族名（`list_families_within`），保留可辨识前缀 + `…`。

3. **兜底也保留 `available:`**，并在最后无条件过一道 `truncate_body`（超了截到 `MAX_ERROR_BODY - 1` + `…`），保证「任何输入都不超」是构造性质，而不是算术巧合。

4. **把不可达的复核换成可杀的守卫**：`clamp_quoted` 的尾部复核改成 `debug_assert!`，其不变量由 `every_clamped_echo_fits_its_budget`（5 链 × budget 3..=120）钉住；真正对外的守卫是 `with_clamped_chain` 的 `fits().then_some(...)`，现在删除它会让 4 个测试变红。另外补 `clamp_quoted_never_shrinks_below_the_ellipsis`，把 budget < 3 时「返回 `"…"`（比预算宽）由调用方拒绝」这条唯一例外也锁住。

## 验证

| 判据 | 父 `4d737b2` | R35 `5482421` | 本轮 |
| --- | --- | --- | --- |
| 矩阵 over200（8 链 × 4 库） | 10 | 0 | **0** |
| missing_available | 0 | 1 | **0** |
| echo_collapsed | 5 | 10 | **0** |

- 阻断 1：**200 字符**、含 ` (available: `、回显 `"X"`、rc=3。
- 阻断 2：**200 字符**、回显 `"Noto"`（不是 `"…"`）。
- 矩阵各格：`repro_lib` 全 200、`fonts_noto7` 全 200、`/tmp/d5c/long` 全 197、默认库 181/182/183/184/184/182/194（`Arial, NoSuch` 在该库 rc=0）。逐格 for 循环里 `echo_collapsed` 为 0，即每格都回显了用户链。
- 长度回归：`X`×78/80/81/86/200 全 **200** 字符；短链 `NoSuchFont_xyz` **194**、`Helvetica, Liberation Sans, DejaVu Sans` **195**、`A,B,C,D,E,F,G,H` **195**（不得变长）。
- 语义不回退：11 条可用链全 rc=0、stderr 0 字节（含 `""`、`,,`、`"Times New Roman"`、` Arial , sans-serif `、`arial`）。
- 逐字节：7 个 fixture 不给 `--labels`、`--width 800 --ncol 2`，与 `4d737b2` 相同。

## 变异测试（改完立即复原）

| 变异 | 结果 |
| --- | --- |
| `with_clamped_chain` 末尾 `fits().then_some` → `Some(body)` | 4 个测试红 |
| `keep = budget - 5` → `budget - 4` | 3 个测试红（含 `every_clamped_echo_fits_its_budget`） |
| `AVAILABLE_OPEN` 改回 `"(available: "`（去前导空格） | `a_single_character_chain_keeps_the_available_list` 红 |
| `truncate_body` 的 `if fits(body)` → `if true` | `the_last_resort_cap_truncates_only_when_it_must` 红 |

## 新测试（`src/warnings/rules_tests.rs`）

- `a_single_character_chain_keeps_the_available_list`（:288）——单字符链 + 7×100 字符族名，必须含 ` (available: `
- `a_short_echo_is_never_collapsed_by_a_long_library`（:316）——Noto 长库 × 链 `Noto/AB/ABC/ABCD`，回显必须完整
- `with_clamped_chain_rejects_a_fixed_part_that_leaves_no_room`（:346）
- `every_clamped_echo_fits_its_budget`（:357）——5 链 × budget 3..=120
- `clamp_quoted_never_shrinks_below_the_ellipsis`（:382）——budget 0..3 的唯一例外
- `the_last_resort_cap_truncates_only_when_it_must`（:399）
- `every_chain_and_library_shape_stays_within_the_cap`（:417）——6 链 × 6 库 fuzz

## 账本锚点修正

R35 的 `lines_after` 不准（记历史，不追改）：

| finding | 账本记 | 真实（`e4660e8`） |
| --- | --- | --- |
| R35-F1 / R35-F3 `describe_missing_font_error` | 52-112 | **52-110** |
| R35-F2 `list_families_within` | 161-201 | **161-197** |

本轮在 R36 的 finding 里写明修正值，并用 `pitfalls-manager modify` 追加事件，旧记录保持原样。

## 遗留（非阻断）

- `list_families`（整段拼接版）仍被阶梯第一级使用；当库里有超长族名时第一级永不命中，实际走第三级，属预期降级而非缺陷。
- `truncate_body` 在现有输入下不可达（兜底那样本已按真实拼串预算），作为构造性保证保留，已由单测直接覆盖。
- 空行网格错误行仍 244 字符（R31 遗留，父已裁定不改）。
