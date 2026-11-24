# svg_grid 阻塞点 — `classify` 取表序第一个命中码，用户内容可把 W9/W7 门禁打成 PASS

场景：`tools/svg_grid` HEAD `31eab2d`（2026-11-24，R53 修复前；报告最初写于 R46 之后的
`54fa2b6`）。这是 R45 把告警短语表搬进 `src/warnings/codes.rs` 后暴露的**既有**缺陷：
`classify` 一直取**表序第一个**命中码，R45 只是把可遮蔽 W9 的短语从 5 条扩到 7 条，同时
把已经失配的 W7 短语换对（W7 判据由此复活）。

## 现象

`src/warnings/codes.rs`（修复前 `classify` 50-68）对每条告警行只取表序第一个命中的码：

```rust
if let Some((code, _)) = WARNING_CODES
    .iter()
    .find(|(_, phrases)| phrases.iter().any(|phrase| line.contains(phrase)))
```

而 W9 的行原样打印悬空 id、W7 的行原样打印属性值，两者都是**用户内容**：

```
svg_grid: warning: 1 reference(s) point at an id that nothing in the output defines: … url(#<用户 id>)
svg_grid: warning: 1 attribute(s) could not be rewritten, kept verbatim: … <用户值>
```

于是只要用户 id / 属性值里出现任何**表序更靠前**的短语（W1 `scaled by the layout`、
W2 `stretched: `、W4a `--labels has `、W4b `repeats some letters`、W5 两条、W6
`draws ink outside its own canvas box`），`classify` 就只返回那个码，W9/W7 消失。
doctor 是 `classify` 的唯一调用方，判据是"该 fixture 的命中码集合包含期望码"，所以
**码被抢走 ⇒ 该 fixture 从 FAIL 变 PASS ⇒ 门禁 fail-open**。

## 复现

三个 fixture 都必须**完整打 tag**（`data-panel-box` + `data-scale` 组），否则 convert 会拒绝而
根本走不到 composer 告警。每个 fixture 单独放一个目录（one-fixture corpus），否则 doctor 的
rc 会被同目录其它 fixture 摊平，测试就没有牙。W9 组：

```bash
cd /tmp && rm -rf r53probe && mkdir -p r53probe/fx/ggplot2 && cd r53probe
cat > fx/MANIFEST.toml <<'TOML'
[[fixture]]
id = "ggplot2/w9"
package = "ggplot2"
expect = "pass"
TOML
cat > fx/ggplot2/w9.svg <<'SVG'
<svg width="100pt" height="80pt" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#--labels has 3 entries)"/></g>
</svg>
SVG
svg_grid doctor --fixtures fx; svg_grid doctor --fixtures fx --json
```

W7 组同理：一个干净 fixture 用 `<rect x="inf">`（触发 W7），再把
`<path d="M0 0 --labels has 3"/>` 加进同一文档里，W7 就被 W4a 抢走。

## 三列

"参考"= R45 之前的 `196da47^`（自建 `/tmp/r53ref`，`git archive` + `cargo build --release`），
那里 W9 语义正确、但 W7 短语已失配（判据是死的）；"base" = 任务指定的
`/tmp/r50base/target/release/svg_grid` @`e0c6033`，它**在 R45 之后**，所以和"修前"同病；
"修前" = HEAD `31eab2d` 的二进制（`/tmp/r53old/svg_grid`，sha256 `1c483bb1…`）；"修后" = R53。

| fixture | 参考 `196da47^` | base `e0c6033` = 修前 `31eab2d` | 修后 |
| --- | --- | --- | --- |
| `w9`（悬空 id = `--labels has 3 entries`） | rc=1 FAIL `['W9']` | rc=0 PASS `['W4a']`（错） | rc=1 FAIL `['W4a','W9']` |
| `w9w5`（悬空 id = `was moved inside the canvas`） | rc=1 FAIL `['W9']` | rc=0 PASS `['W5']`（错） | rc=1 FAIL `['W5','W9']` |
| `w7`（干净 W7） | rc=0 PASS `['W6']`（W7 判据当时是死的） | rc=1 FAIL `['W6','W7']` | rc=1 FAIL `['W6','W7']` |
| `w7mask`（W7 + `--labels has 3`） | rc=0 PASS `['W6']` | rc=0 PASS `['W6','W4a']`（错） | rc=1 FAIL `['W6','W4a','W7']` |

参考列还说明了 R45 的另一面：R45 之前表里的 W7 短语与 `describe_passthrough` 的实际文案
对不上，W7 判据**根本不可能命中**；R45 换上 `could not be rewritten, kept verbatim` 才复活它。
所以 R45 不能整体回退——R53 修的是它顺带扩大的遮蔽面。每行"命中多个码"的顺序是
`classify` 按行序、行内按表序累积：W6 行先出现，W4a 与 W7 同属 W7 行且 W4a 表序在前。

## 修法

1. `classify`（`src/warnings/codes.rs:57-72`）改为**收集全部命中码**：

   ```rust
   for (code, phrases) in WARNING_CODES {
       if phrases.iter().any(|phrase| line.contains(phrase)) && !codes.contains(code) {
           codes.push(code);
       }
   }
   ```

   顺序保持稳定（行序 + 表序），`warning_codes` 因此对干净语料逐字节不变；同一行多码时
   `--json` 报全部码（doctor 只做集合包含，正是它需要的语义）。
2. `is_warning_line`（`src/warnings/codes.rs:74-80`）**决定修**：原来的
   `!line.contains("non-fatal warning(s)")` 会吞掉"正文里恰好提到这句"的真实告警行。
   现在只排除 footer 自己的正文：

   ```rust
   let body = line
       .strip_prefix("svg_grid: warning:")
       .or_else(|| line.strip_prefix("svg_grid convert: warning:"));
   body.is_some_and(|body| !body.trim_start().starts_with("non-fatal warning(s)"))
   ```

   理由：footer 是固定整句且以该串开头，正文里的 W9 行可能把该串当悬空 id 原样打印；
   按"正文以 footer 句式开头"排除既保住原意（不数 footer），又不再吞真实告警。
3. **核心不变量**（表驱动守卫，`src/warnings/codes_tests.rs`）：对 `WARNING_CODES` 的每一条
   `(码, 短语)`，把短语塞进 W9 的悬空 id（`a_phrase_inside_a_dangling_id_cannot_mask_w9`）
   与 W7 的属性值（`a_phrase_inside_a_kept_verbatim_value_cannot_mask_w7`），断言
   `classify` 仍含 `W9` / `W7`。W9 守卫先证明手工构造的行与生产
   `describe_dangling_reference` 同形（长短语会被 200 字符预算截断，所以生产路径不能直接复用），
   W7 守卫走真实 `describe_passthrough`。
4. 三个复现落成集成测试 `tests/selftest/doctor/masking.rs`：断言 doctor rc=1、
   `FAIL  <id>  check`、`failures:` 块、`stderr contains W9/W7`，以及 `--json` 里精确的
   `"warning_codes": [...]` 串。
5. 同族清理：`rules_tests.rs` 的第三份短语副本改为从 `WARNING_CODES` 派生
   （`warning_phrases()`），并断言 `phrases == table`；`every_warning_code_still_matches_its_message`
   改为**逐短语**核对真实生产文案，表里加一条死短语立刻变红（旧版只比码 id，抓不到）；
   删掉 `R17/test.py` 的死变量 `KNOWN_CODE_WORDS`（它本身是第四份已过期的短语副本）。

## 变异

| # | 变异 | 红 |
| --- | --- | --- |
| M1 | `classify` 退回 `WARNING_CODES.iter().find(...)` | 5：`warnings::codes_tests::a_phrase_inside_a_dangling_id_cannot_mask_w9`、`…kept_verbatim_value_cannot_mask_w7`，以及 3 个 `doctor::masking::*` |
| M2 | `warning_phrases()` 改回 8 条手写旧副本 | 1：`warnings::rules_tests::no_cause_note_repeats_a_warning_phrase`（断言 `phrases == table` 失败） |
| M3 | 给 W6 加一条永不出现在生产文案里的死短语 | 1：`warnings::codes_tests::every_warning_code_still_matches_its_message` |
| M4a | 删掉 footer 排除（`body.is_some()`） | 1：`warnings::codes_tests::only_the_footer_is_excluded_not_a_body_mentioning_it` |
| M4b | 退回 `!line.contains("non-fatal warning(s)")` | 同 M4a，1 红 |

## 回归

- `cargo test --offline`：**528**（284 lib + 59 main + 185 selftest；基线 522，+6，未删既有测试）。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 均通过。
- `./install.sh` 后 `sha256sum ~/.local/bin/svg_grid target/release/svg_grid` 同为
  `e111f6ed5ab559fb23b6032a9be6b936ef720963e35f2d8031218387b9ba6cb9`。
- `svg_grid doctor`：57 pass / 0 fail / 1 skip。
- 语料逐字节（base `/tmp/r50base/target/release/svg_grid` @`e0c6033`，sha256 `75def0b8…`）：
  214 个 `tests/**/*.svg` 的 `convert` 与 `--report`（428 次调用）0 差异；
  58 个 fixture 的 `plot-grid --width 600 --ncol 1` 与 `--plan`（116 次调用）0 差异；
  rc/stdout/stderr/产物字节全比。doctor `--json` 逐 fixture 0 差异（只把每条的 `"ms"` 归一，
  base 自己两次运行也会变，其余逐字节相同）。
- 旧证据 R3..R52 顶层（R19 已 retire，不计）全 rc=0。
- 单文件：`src/warnings/codes.rs` 3568 B、`codes_tests.rs` 11597 B、`rules_tests.rs` 17301 B、
  `tests/selftest/doctor/masking.rs` 5073 B，均 ≤20000 B；`src/lib.rs` 18491 B、`src/dom.rs` 19027 B 未动。

## 账本（R45 行号漂移补记）

三个位置都是 R45 造成的净漂移，R53 追加 finding 记录（append-only，不改旧记录）：

| where | 旧 finding / 锚点 | 锚点实测 | R53 提交实测 | 归因 |
| --- | --- | --- | --- | --- |
| `src/warnings/mod.rs:emit` | R31-F3 @`3311eec` | 189-202 | 202-215 | R45 +13；R48 动过 `mod.rs` 但未越过 `emit`，净 0 |
| `src/warnings/mod.rs:assemble_within` | R34-F2 @`e362c019` | 257-279 | 261-283 | R45 +4；之后未再动 |
| `src/lib.rs:plot_grid_with_diagnostics` | R34-F4 @`e362c019` | 152-325 | 157-330 | R45 → 155-328（+3）；R48 → 163（+8）、R51 → 180（+17）、R52 → 157（−23），后几轮净 +2 |

（`git show <rev>:<path>` + 花括号计数实测；`lines_before` 一律按 finding 自己的锚点核，
不用当前树坐标冒充历史坐标。）
