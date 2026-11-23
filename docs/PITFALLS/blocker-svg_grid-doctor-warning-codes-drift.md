# doctor 的 WARNING_CODES 与产品文案漂移，W7 门禁 fail-open（R45）

## 现象

`svg_grid doctor` 把 stderr 里的短语映射成 `ARCHITECTURE/warnings.md` §3 的稳定码
（`W1`…`W9`、`V1`），并用其中两个码判 FAIL（`W7`、`W9`）。R45 之前这张表
（`src/cli/doctor/run.rs:394-409`）的 11 条里有 **5 条永远匹配不上**产品真实文案：

| 码 | 表里的片段 | 产品真实文案 | 后果 |
|---|---|---|---|
| W4a | `` `--labels` has ``（带反引号） | `--labels has 3 entries but the grid has 2 inputs` | 永不命中 |
| W5 | `may be clipped by the canvas` | 主路径是 `was moved inside the canvas` | 主路径永不命中 |
| W7 | `could not be rewritten and were left exactly as written` | `1 attribute could not be rewritten, kept verbatim: …` | **门禁 fail-open** |
| W3 | `already carry letters`（片段本身对） | 行前缀是 `svg_grid: invalid input:` | 被 `is_warning_line` 跳过 |
| W8 | `is not in the library`（片段本身对） | 行前缀是 `svg_grid: font library error:` | 被 `is_warning_line` 跳过 |

只有 W1/W2/W4b/W6/W9/V1 能命中。W7 最重：`judge()`（`src/cli/doctor/run.rs:364-369`）
用 `codes.contains(&"W7")` 判 FAIL，而 W7 片段是死码 ⇒ 这条判据从未生效，
带 `x="inf"` 的 fixture 会被判 PASS。

## 复现

```
$ mkdir -p /tmp/w7cmp && cat > /tmp/w7cmp/w7.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect x="10" y="10" width="40" height="30" fill="#000000"/>
  <rect x="inf" y="10" width="20" height="20" fill="#000000"/>
</svg>
SVG
$ printf '[[fixture]]\nid = "w7"\npackage = "r45"\nexpect = "pass"\n' > /tmp/w7cmp/MANIFEST.toml
$ svg_grid doctor --fixtures /tmp/w7cmp
```

修复前（`2d76da3`）：

```
PASS  w7  both  3  warnings=2
summary: 1 pass, 0 fail, 0 skip
```

`--json` 的 `warning_codes` 是 `["W6"]` —— 产品明明打印了
`svg_grid: warning: 1 attribute could not be rewritten, kept verbatim: input #0 <rect x>: non-finite number "inf".`，
却没有被认成 W7。

修复后：

```
FAIL  w7  check  3  warnings=2
summary: 0 pass, 1 fail, 0 skip
```

`--json` 的 `warning_codes` 是 `["W6", "W7"]`，文本报告点名
`stderr contains W7 (an attribute could not be rewritten and was kept verbatim)`。

## 根因

1. **短语表与文案分居两处。** 表在 `src/cli/doctor/run.rs`，文案在
   `src/warnings/*.rs` 的 `describe_*` 里；改文案时没有任何机制提醒改表。
2. **没有用真实文案回灌的测试。** `src/cli/doctor/tests.rs:442-460` 的
   `w7_and_w9_fail_the_fixture_but_w1_does_not` 自己编了一条与表自洽、与产品无关的
   W7 字符串；`classify` 的另两个测试同样是手写字符串。它们只证明「表与测试一致」，
   不证明「表与产品一致」。
3. **注释承诺的测试不存在。** `src/cli/doctor/run.rs:393` 写着
   `every_warning_code_still_matches_its_message`，仓库里没有这个测试。
4. **同一漂移的化石。** `docs/PITFALLS/round-data/R17/test.py:44-46` 定义了
   `KNOWN_CODE_WORDS`（其中 W7 也是旧文案），但从未被使用 —— 说明这个风险早被察觉，
   只是没有落成断言。

## 修法

1. **表移到文案旁边**：新文件 `src/warnings/codes.rs`，`WARNING_CODES` 改成
   `&[(&str, &[&str])]`，一个码可以有多条短语（W5 的「被移动」与「可能被裁」两种形态
   都保留，没有为了省事删掉极端分支）。`classify` 与 `is_warning_line` 一并搬过去，
   `src/warnings/mod.rs` 重导出，`src/lib.rs` 对外暴露
   `classify` / `WARNING_CODES` / `describe_passthrough` / `Passthrough`。
   `src/cli/doctor/run.rs` 删掉本地副本（1,723 B），改用 `svg_grid::classify`。
2. **5 条死码的处置**：
   - W4a → `--labels has `（去掉反引号，与 `describe_label_count` 一致）。
   - W5 → 两条短语：`was moved inside the canvas` + `may be clipped by the canvas`。
   - W7 → `could not be rewritten, kept verbatim`（与 `describe_passthrough` 一致）。
   - W3 / W8 → **从表中移除**（选 (a)）。理由：它们是硬错误，行前缀分别是
     `svg_grid: invalid input:`（rc=2）与 `svg_grid: font library error:`（rc=3），
     `is_warning_line` 只认 `svg_grid: warning:` / `svg_grid convert: warning:`，
     所以它们**永远不可能**被 `classify` 看到；doctor 对它们本来就是靠 rc 判。
     若选 (b)（让 `classify` 也认这两个前缀），`warnings` 计数语义会从「warning 行数」
     变成「warning + 硬错误行数」，`--json` 的 `warnings` 字段与既有输出都会变，
     收益却只是把两个由 rc 已经覆盖的码再记一遍 —— 不值得。
3. **补上守护测试** `src/warnings/codes_tests.rs::every_warning_code_still_matches_its_message`：
   对每个码**调用真实的 `describe_*` 函数**产出文案（W1 `describe_scale`、W2
   `describe_stretch`、W4a `describe_label_count`、W4b `describe_label_duplicates`、
   W5 `describe_moved_label`、W6 `describe_content_outside_canvas`、W7
   `describe_passthrough`、W9 `describe_dangling_reference`、V1 由
   `crate::convert::convert` 真跑一个 `<style>` 文档取回），断言 `classify` 认得出；
   并断言表的码集合与测试覆盖的码集合**完全相等**（表里多一条没人能产出的码也会红）。
   W3/W8 在测试里显式排除并注释原因。
   另加 `a_code_with_two_shapes_recognises_both`（W5 两种形态）与
   `hard_error_lines_are_not_classified_as_warnings`（硬错误行不计入 warnings）。
4. **把自洽测试换成真实文案**：`w7_and_w9_fail_the_fixture_but_w1_does_not` 里的 W7
   字符串改为 `svg_grid::describe_passthrough(...)` 的产出。
5. **端到端证明门禁活着**：`tests/selftest/common/harness.rs` 新增 `W7_PANEL` 与
   `w7_corpus()`，`tests/selftest/doctor/corpus.rs` 新增
   `a_kept_verbatim_attribute_fails_the_fixture`（doctor rc=1、`FAIL … check`、
   文本报告点名 W7、`--json` 带 W7）。
6. **注释指向真实测试**：`src/warnings/codes.rs` 的文档注释点名
   `every_warning_code_still_matches_its_message`，该测试确实存在。

## 变异验证

备份 `/tmp/r45_codes.bak`，改完还原：

| 变异 | 结果 |
|---|---|
| W4a 片段改回 `` `--labels` has `` | `every_warning_code_still_matches_its_message` FAILED：`W4a is not recognised from its own message: "--labels has 3 entries but the grid has 2 inputs: …" -> []` |
| W7 片段改回 `could not be rewritten and were left exactly as written` | 同一测试 FAILED：`W7 is not recognised from its own message: "1 attribute could not be rewritten, kept verbatim: input #0 <rect x>: non-finite number \"inf\". …" -> []` |

两条都只让守护测试红，说明它确实盯着「表 ↔ 真实文案」这条缝。

## 验证

- `cargo test --offline`：**231 lib + 59 main + 129 selftest = 419**，0 fail（R44 是 415）。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` rc=0；`svg_grid doctor` **57 pass / 0 fail / 1 skip**（没有 fixture 意外触发 W7）。
- 产物等价（对 `2d76da3` 的 release 二进制）：220 个 `tests/**/*.svg` 的 `convert`
  产物 **0 字节差异**（两侧各 41 个失败，无单侧成功）；58 个 `tests/fixtures/**` 的
  `plot-grid --width 600` **0 字节差异、0 rc 差异、0 `--plan` 差异**；
  `convert --report` 在相同 `--output` 路径下 stdout/stderr/rc 全同。
- 旧证据 R8..R44 顶层全 rc=0（R19 已 deprecated，不计）。
- `docs/PITFALLS/round-data/R45/test.py`：21 条全成立；带
  `SVG_GRID_OLD_BIN=/tmp/r45_base_target/release/svg_grid` 时 24 条全成立
  （E 段在旧二进制上必须红：旧二进制放过 W7 fixture、`--json` 无 W7）。

## 遗留（未修，非本轮授权）

- `src/warnings/rules_tests.rs:236-266` 的 `no_cause_note_repeats_a_warning_phrase`
  里还有一份短语清单，是第二处需要同步的副本；本轮没有把它接到 `WARNING_CODES` 上。
- `doctor` 只对 W7/W9 判 FAIL，其余码（含 W1/W2/W4a/W4b/W5/W6/V1）仅记录不判失败 ——
  这是既有设计（`ARCHITECTURE/warnings.md` §3），本轮未改。
- `docs/PITFALLS/round-data/R17/test.py:44-46` 的 `KNOWN_CODE_WORDS` 仍是旧文案且未被使用；
  它是历史证据脚本，本轮不动（R17 的断言本身不依赖它）。
