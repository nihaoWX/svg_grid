# svg_grid 阻塞点 — 未解析实体的诊断带"假 range"，账本与元数据把坐标记错

场景：`tools/svg_grid` HEAD `582f6f1`（2026-11-24，R51 之后）。R50（`1fbc3c3`）修好了 XML 1.0
非法字符被解码进产物，评审 PASS，但留下 2 条 P2 + 3 条 P3；本轮 R52 收口其中 5 条。

## 现象

### 1（P3，代码）未解析实体的文案里有"假 range"

`svg_grid convert` 对 `<text>a&nbsp;b</text>` 输出：

```
svg_grid convert: XML error: reference &nbsp; cannot be resolved: at 1..5: unrecognized entity `nbsp`
```

`at 1..5` 是 `quick_xml` 对我们**合成串** `&{reference};`（`src/dom.rs:126` 的
`let literal = format!("&{reference};")`）的字节偏移，不是文档坐标：同一个 `&nbsp;` 放在第 12 列
仍然报 `at 1..5`，长文档里完全误导。属性路径同病（`data-x="a&foo;b"` 报 `at 1..4`）。

还有第二种拼法，只有属性路径能触发（未终止的引用，`data-x="a&nbsp"`）：

```
svg_grid convert: XML error: attribute `data-x`: Error while escaping character at range 1..6: Cannot find ';' after '&'
```

`at range 1..6` 是 `EscapeError::UnterminatedEntity(Range<usize>)` 的 Display
（`quick-xml-0.39.4/src/escape.rs:64`），同样是合成偏移，且**不匹配** `at \d+\.\.\d+` 这条正则，
所以只按正则收口会漏掉它。

### 2（P2，账本）R50 的 `modify R47` 用错了坐标系

`modify` 是 append-only 事件日志。R50 记了 5 处"行号更正"，实测只有 R47-F7
（`docs/PITFALLS/round-data/R47/test.py` `1-413 → 1-442`）为真；其余 4 处把 **R50 提交树
（`1fbc3c3`）的坐标**写成了 R47 的坐标。R47 自己的锚点是 `8aca0d1`，在它上面实测：

| finding | R50 记的"更正" | `8aca0d1` 实测 |
| --- | --- | --- |
| R47-F1 `src/dom.rs:parse`（`Event::GeneralRef` 分支） | 174-186 | **80-96** |
| R47-F2 `src/convert/dom.rs:parse` | 95-105 | **82-93** |
| R47-F4 `src/dom.rs:tests` | 280-459 | **178-245**（`#[cfg(test)]` 178、`fn` 179、文件共 245 行） |
| R47-F5 `src/convert/dom.rs:tests` | 207-321 | **186-233**（文件共 233 行） |
| R47-F7 `docs/PITFALLS/round-data/R47/test.py` | 1-442 | 1-442（这条对，保留） |

### 3（P2，元数据）`docs/PITFALLS/round-data/R50/meta.toml` 记错语料与 base

- 写"220 个 `tests/**/*.svg`" ⇒ 实测 **214**（`git ls-files '*.svg'` 全仓 217，另 3 个在 R28 的
  data 里）。220 的来源是 `pathlib.rglob` 把 6 个**被 gitignore 的**生成物也算进去了
  （`tests/mpl_dialect/{composed,panel,panel.tagged}.svg`、`tests/rdkit_dialect/{mol.grid,mol,mol.tagged}.svg`）。
- base 写成 `/tmp/r46base @196da47`（sha `99fd8cb5…`）⇒ 本次实际用的是 `/tmp/r50base @e0c6033`
  （sha `75def0b8…`）。

### 4（P3，账本）R50 自己的 findings 行号越界 1–2 行

在 `1fbc3c3` 上实测：F1 `is_xml_char` 记 54-60 ⇒ 实为 **54-59**；F2 `decode_reference` 记 78-98 ⇒
实为 **78-96**；F4 `element_from_start` 记 217-247 ⇒ 实为 **217-245**；同轮报告里
`first_invalid_xml_char` 记 62-65 ⇒ 实为 **62-64**、`numeric_reference_codepoint` 记 100-114 ⇒
实为 **100-110**。其余 8 条评审确认精确，不动。

### 5（P3，体积）`src/lib.rs` 距 20000 B 红线只剩 413 B

19587 B / 上限 20000 B（R8/R9 门禁），任何一次小改都可能越线。

## 复现步骤

```bash
cd /tmp && rm -rf r52probe && mkdir r52probe && cd r52probe
cat > t.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><text x="1" y="10">a&nbsp;b</text></svg>
SVG
cat > a.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><text x="1" y="10" data-x="a&foo;b">x</text></svg>
SVG
cat > u.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><text x="1" y="10" data-x="a&nbsp">x</text></svg>
SVG
svg_grid convert --input t.svg --output o.svg   # 修前：... cannot be resolved: at 1..5: unrecognized entity `nbsp`
svg_grid convert --input a.svg --output o.svg   # 修前：attribute `data-x`: ... at 1..4: ...
svg_grid convert --input u.svg --output o.svg   # 修前：attribute `data-x`: Error while escaping character at range 1..6: ...
```

账本侧：

```bash
cd <repo>
git show 8aca0d1:src/dom.rs | nl -ba | sed -n '80,96p'          # R47-F1 真实范围
git show 8aca0d1:src/convert/dom.rs | nl -ba | sed -n '82,93p'  # R47-F2
git show 8aca0d1:src/dom.rs | wc -l                             # 245（不是 459）
git show 1fbc3c3:src/dom.rs | nl -ba | sed -n '54,59p'          # R50-F1 真实范围
git ls-files 'tests/**/*.svg' | wc -l                           # 214
```

## 修法

1. **一份"剥 range"逻辑，两条路径共用**（`src/dom.rs:71-116`）：
   `pub(crate) fn strip_synthetic_range(message: &str) -> String` 同时处理两种拼法——
   前缀 `at N..M: `（`range_prefix`，`src/dom.rs:97-102`）与中缀 ` at range N..M`（`range_len`，
   `src/dom.rs:105-116`，保留其后的 `:`）。数字/`..`/`:` 任一不匹配就原样返回，所以
   `invalid character reference: unexpected number sign` 这类**本来没有 range** 的文案逐字不变。
   调用点：`src/dom.rs:137`（`decode_reference` 的 `Err` 分支，文本路径与属性路径的
   `invalid_reference_in_raw` 都走这里）、`src/dom.rs:285` 与 `src/convert/dom.rs:144`
   （两个 DOM 的 `decode_and_unescape_value` map_err，覆盖未终止引用）。
   判据只有一份，`src/convert/dom.rs:10` 从 `crate::dom` 导入——沿用 R47 建立的
   "`src/dom.rs` 判据被 `src/convert/dom.rs` 复用"的防漂移做法。
2. **账本 append-only**：不改 R47/R50 的任何字段，R52 追加 5 条 finding（R52-F1..F5）用
   `related` 挂到 R47-F1/F2/F4/F5/F7，`symptom`/`reason` 写明"R50 的 `modify` 误用了 `1fbc3c3`
   树的坐标，本条按 R47 自己的锚点 `8aca0d1` 更正"；再追加 5 条（R52-F6..F10）挂到
   R50-F1/F2/F4 与报告里的两个函数，写明"R50 记录越界 1–2 行，按 `1fbc3c3` 实测定准"。
3. **元数据**：`R50/meta.toml:63-64` 改成 `214 个 tests/**/*.svg（全仓 217，另 3 个在 R28 data 里）`
   与 `/tmp/r50base/target/release/svg_grid @e0c6033，sha256 75def0b8…`；并在其后追加一段 R52
   复核结论（只有 R47-F7 成立）。`R50/test.py` 里没有硬编码 220/r46base（grep 为空），未动，
   断言一条也没放宽。
4. **体积**：`normalize_label`（函数体 + 上面那段长 doc comment）整体搬到
   `src/label_letters.rs:25-47`，`src/lib.rs:28` 用 `pub use label_letters::normalize_label;`
   保持外部路径 `svg_grid::normalize_label` 不变（`src/cli/execute.rs:184` 用的就是这条路）。
   纯机械搬迁，零语义变化。

## 三列数字

`base` = `/tmp/r50base/target/release/svg_grid`（@`e0c6033`，sha256 `75def0b8…`，R50 修前）；
`修前` = HEAD `582f6f1` 的二进制（`/tmp/r52old`，sha256 `70958cf8…`）；`修后` = 本轮。

| 用例 | base | 修前 | 修后 |
| --- | --- | --- | --- |
| 文本 `a&nbsp;b` | `XML error: at 1..5: unrecognized entity \`nbsp\`` | `... reference &nbsp; cannot be resolved: at 1..5: ...` | `... reference &nbsp; cannot be resolved: unrecognized entity \`nbsp\`` |
| 文本 `a&bogus;b` | `at 1..6` | `at 1..6` | 无 range |
| 属性 `a&foo;b` | `at 2..5` | `at 1..4` | 无 range |
| 属性 `a&nbsp`（未终止） | `at range 1..6` | `at range 1..6` | 无 range |
| 文本 `a&#1;b` | rc=0（漏判） | `character reference &#1; decodes to U+0001, ...` | 逐字不变 |
| 属性 `a&#1;b` | rc=0（漏判） | `attribute \`data-x\`: character reference &#1; ...` | 逐字不变 |
| 文本 `a&#0;b` | `invalid character reference: 0x0 ...` | `character reference &#0; (U+0000) is not a legal XML 1.0 character: ...` | 逐字不变 |
| 文本 `a&#-1;b` | `invalid character reference: unexpected number sign` | `reference &#-1; cannot be resolved: invalid character reference: unexpected number sign` | 逐字不变 |

`at \d+\.\.\d+` 与 `at range ` 在修后二进制上**零命中**（证据脚本 A/B/C 族逐条断言）。

体积：

| 文件 | 修前 | 修后 |
| --- | --- | --- |
| `src/lib.rs` | 19587 B | **18491 B**（−1096） |
| `src/label_letters.rs` | 12612 B | **13748 B**（+1136） |
| `src/dom.rs` | 16969 B | 19027 B |
| `src/convert/dom.rs` | 11821 B | 13180 B |
| `tests/selftest/textref/mod.rs` | 15828 B | 16853 B |
| `tests/selftest/textref/undefined.rs` | —（新） | 4032 B |

## 影响

- 诊断：用户看到的偏移是**假坐标**，长文档里会把人引到错误位置；两种拼法都收口后，文案只留
  实体名与原因，可 grep、可断言。
- 账本：错误的 `modify` 会让下一个 agent 在 `8aca0d1` 上按 174-186 去找 R47-F1，找不到；
  更糟的是它把"R50 的树"当成"R47 的树"，等于把历史坐标系统悄悄换掉。
- 元数据：220 vs 214 会让下一轮把 6 个被 ignore 的生成物当成语料，base 写错则逐字节对照
  直接对错基线。

## 不做

- `docs/PITFALLS/blocker-svg_grid-classify-first-match-masking.md` 那条（`classify` 取表序第一个
  命中码）是既有已知问题，本轮不动。
- R50 评审的 P3-3..P3-8（旧证据只在主仓路径可复现、install sha 不可复核、HARNESS 余量、
  PROGRESS 陈旧、一条 labels 测试偶发、modify 事件缺证据）不在本轮派单范围。
