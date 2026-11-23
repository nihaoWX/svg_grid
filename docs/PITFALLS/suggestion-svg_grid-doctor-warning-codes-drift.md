# svg_grid 建议 — doctor 的 `WARNING_CODES` 有两条码词永远匹配不上，且注释承诺的守护测试不存在

场景：`tools/svg_grid` HEAD `b55669e`（2026-11-23，D5b 之后）。`src/cli/doctor/run.rs:394-409`。

## 现象

`WARNING_CODES` 用"文案片段"把 stderr 行映射成告警码（`classify`，`src/cli/doctor/run.rs:415-432`），
`--json` 的 `warning_codes` 字段就是它的输出。其中两条片段与真实文案**对不上**：

| 码 | 表里的片段 | 真实文案（实测） |
|---|---|---|
| W4a | `` `--labels` has `` （带反引号） | `--labels has 3 entries but the grid has 2 inputs: …`（无反引号，`src/warnings/labels.rs:71`） |
| W5 | `may be clipped by the canvas` | D5 起主路径是 `was moved inside the canvas`（`src/warnings/labels.rs:49`）；`may be clipped` 只在字母 ink 仍越界时出现（`src/warnings/labels.rs:128`） |

后果：`classify` 对这两条**静默漏判**——`warnings=N` 计数仍对（它只数 `svg_grid: warning:` 行），
但 `warning_codes` 里缺项。今天还看不到错，因为 doctor 的 plot 阶段从不传 `--labels`
（`src/cli/doctor/run.rs:248-265` 的参数表里没有它），所以 W4a/W5 根本不会出现在 doctor 的 stderr 里；
一旦将来给 doctor 加一个带字母的 fixture，这两条就会被静默吞掉。

`src/cli/doctor/run.rs:393` 的注释写着"a reworded message is caught by
`every_warning_code_still_matches_its_message` in the unit tests"——**这个测试在仓库里不存在**
（`grep -rn every_warning_code` 只命中这行注释本身；`git log -S` 也只命中 `4133c83` 引入注释的那次提交）。
注释是空头承诺，正是这两条漂移没被发现的原因。

## 复现步骤

```bash
cd /home/nihao/bioagentforge/tools/svg_grid

# 1. 两条片段确实匹配不上真实文案
grep -n 'W4a\|W5' src/cli/doctor/run.rs | sed -n '1,4p'
grep -n 'entries but the grid has' src/warnings/labels.rs
grep -n 'was moved inside the canvas' src/warnings/labels.rs

# 2. 注释承诺的测试不存在
grep -rn every_warning_code --include=*.rs .    # 只有 run.rs:393 的注释

# 3. doctor 从不传 --labels，所以今天看不到错
sed -n '248,265p' src/cli/doctor/run.rs
```

## 建议

希望 `WARNING_CODES` 的每条片段都由一个**真实构造的告警文案**来守护（表里每条码至少有一条测试断言
"用真实 `describe_*` 产出的文案能被 `classify` 认出来"），这样文案一改就会红；顺带把 W4a/W5 两条片段
改成与现行文案一致，并让注释指向真实存在的测试名。

## 追加实测（2026-11-24，R44 之后，HEAD `42184d5`）

用真实 `describe_*` 产出的文案逐条比对 `WARNING_CODES`（脚本见下），**11 条里有 5 条永远匹配不上，不是两条**：

| 码 | 表里的片段 | 真实文案（实测） | 后果 |
|---|---|---|---|
| W4a | `` `--labels` has `` | `--labels has 3 entries but the grid has 2 inputs: …` | `--json` 缺项（doctor 从不传 `--labels`，暂不可见） |
| W5 | `may be clipped by the canvas` | 主路径 `was moved inside the canvas`（`src/warnings/labels.rs:49`）；`may be clipped` 只在 `src/warnings/labels.rs:128` 的极端分支 | 同上 |
| **W7** | `could not be rewritten and were left exactly as written` | `1 attribute could not be rewritten, kept verbatim: input #0 <rect x>: non-finite number "inf".`（`src/warnings/passthrough.rs:110`） | **不只是 `--json` 缺项**：`judge()`（`src/cli/doctor/run.rs:364-369`）用 `codes.contains(&"W7")` 把 W7 判成 FAIL，而 `classify` 永远产不出 `W7` ⇒ **doctor 的 W7 门禁是死代码，fail-open** |
| **W3** | `already carry letters` | 片段本身对，但真实行是 `svg_grid: invalid input: panels in input #0 … already carry letters`（`src/warnings/labels.rs:19`）——**不是 `svg_grid: warning:` 行**，`is_warning_line`（`src/cli/doctor/run.rs:435-438`）直接跳过 | `--json` 永远没有 `W3`（D5 起 W3 是硬错误 rc=2，doctor 靠 rc 判 FAIL，所以不 fail-open，但表里这一行是死的） |
| **W8** | `is not in the font library` | 同上：真实行是 `svg_grid: font library error: …`（`src/warnings/fonts.rs:77`），不是 warning 行 | 同上（W8 是 rc=3 硬错误） |

即：`classify` 的 `is_warning_line` 只认 `svg_grid: warning:` / `svg_grid convert: warning:` 前缀，而 W3/W8 是**硬错误行**、W4a/W5/W7 是**片段写错**。11 条里真正能命中的只有 W1/W2/W4b/W6/W9/V1 六条。

复现（W7 真实可达）：

```bash
cd /home/nihao/bioagentforge/tools/svg_grid
cat > /tmp/w7.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80"><rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/><g data-scale="xy"><rect x="inf" y="0" width="10" height="10" fill="#000"/></g></svg>
SVG
~/.local/bin/svg_grid plot-grid --input /tmp/w7.svg --width 400 --ncol 1 --verbose --output /tmp/w7o.svg 2>&1 | grep 'kept verbatim'
# svg_grid: warning: 1 attribute could not be rewritten, kept verbatim: input #0 <rect x>: non-finite number "inf". …
```

逐条比对脚本（把 `WARNING_CODES` 的片段与真实文案对照，输出 MISS 即漂移）：

```bash
python3 - <<'PY'
import re, pathlib
run = pathlib.Path('src/cli/doctor/run.rs').read_text()
pairs = re.findall(r'\("([^"]+)",\s*"((?:[^"\\]|\\.)*)"\)',
                   re.search(r'WARNING_CODES: &\[\(&str, &str\)\] = &\[(.*?)\];', run, re.S).group(1))
real = {
 'W1': 'scaled by the layout', 'W2': 'stretched: ', 'W3': 'already carry letters',
 'W4a': '--labels has 3 entries but the grid has 2 inputs',
 'W4b': 'repeats some letters',
 'W5': 'was moved inside the canvas',
 'W6': 'draws ink outside its own canvas box',
 'W7': 'could not be rewritten, kept verbatim',
 'W8': 'is not in the font library',
 'W9': 'point at an id that nothing in the output defines',
 'V1': '<style> CSS block',
}
for code, phrase in pairs:
    phrase = phrase.encode().decode('unicode_escape')
    print(f"{code:4} {'OK ' if phrase in real.get(code, '') else 'MISS'} {phrase!r}")
PY
```

## 建议（追加）

1. 三条片段都改成与现行文案一致：W4a → `--labels has `、W5 → `was moved inside the canvas`（并保留 `may be clipped by the canvas` 作为**第二条**片段，或让 `classify` 支持一码多片段）、W7 → `could not be rewritten, kept verbatim`。
   另：W3/W8 要么从 `WARNING_CODES` 移出（它们是硬错误，靠 rc 判），要么让 `classify` 也认 `svg_grid: invalid input:` / `svg_grid: font library error:` 前缀——现状是"表里有、永远产不出"，最容易被误读成"已覆盖"。
2. 补上注释承诺的守护测试（`every_warning_code_still_matches_its_message`）：对每个码用**真实 `describe_*` 产出的文案**断言 `classify` 能认出来——这样文案一改就红，而不是靠人肉比对。
3. W7 这条尤其要测：它是 `judge()` 的 FAIL 判据，漂移的后果是 doctor 静默放过本该 FAIL 的 fixture（fail-open），比 `--json` 缺项严重。
