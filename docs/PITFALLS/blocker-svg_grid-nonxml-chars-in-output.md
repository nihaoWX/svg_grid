# 产物不良构：XML 1.0 非法字符被解码进输出

## 现象

R47（`8aca0d1`）把文本节点的字符引用改成 `quick_xml::escape::unescape` 解码。`unescape`
只按「引用语法」判，不按 XML 1.0 的 `Char` 产生式判：`&#1;`、`&#8;`、`&#11;`、`&#12;`、
`&#31;`、`&#xFFFE;`、`&#xFFFF;` 都能解码成裸控制字符 / 非字符。产物因此从良构变成
**不良构**，而 `convert` 仍返回 0：

```
printf '<svg xmlns="http://www.w3.org/2000/svg" width="60" height="30"><text x="5" y="20">a&#1;b</text></svg>' > t.svg
svg_grid convert --input t.svg --output t_new.svg   # rc=0，产物含裸 0x01
/tmp/d1_eval/render/target/release/d1render t_new.svg out.png 1
# rc=3  parse failed: SVG data parsing failed cause a non-XML character '\u{1}' found at 2:234
```

属性路径（`decode_and_unescape_value`）在 R47 之前就有同样的洞，不是 R47 引入：
`data-x="a&#1;b"` ⇒ 产物 `data-x="a\001b"`、`d1render` rc=3。两条路径必须一起修，否则
又造出第二处口径差。

## 复现（修前 / 修后 / 参考）

修前二进制：`/tmp/r50base/target/release/svg_grid`（`e0c6033`，R49 末）。
修后二进制：本仓 `target/release/svg_grid`。
参考渲染器：`/tmp/d1_eval/render/target/release/d1render`（resvg 0.47.0）与
`~/.venv/bin/python -c "import cairosvg; …"`（libxml2）。

| 输入 | 修前 rc | 修前产物 | 修后 rc | 修后产物 | d1render（原图） | cairosvg（原图） |
|---|---|---|---|---|---|---|
| `&#9;` `&#10;` `&#13;` `&#32;` `&#65;` `&#x1F600;` `&#xFFFD;` `&#x10FFFF;` `&#xE000;` `&#xD7FF;`（文本/属性） | 0 | 良构 | 0 | 良构 | 0 | 0 |
| `&#1;` `&#8;` `&#11;` `&#12;` `&#31;` `&#xFFFE;` `&#xFFFF;`（文本/属性） | **0** | **含 1 个非法字符、不良构** | **1** | **不写产物** | 3 | 1 |
| `&#0;` `&#x0;` `&#55296;` `&#xDFFF;` `&#x110000;`（文本/属性） | 1（`unescape` 已拒） | 不写产物 | 1 | 不写产物 | 3 / 0 / 0 | 1 |

`d1render` 对 `&#55296;`、`&#xDFFF;`、`&#x110000;` 返回 0（resvg 对代理区/溢出宽松），
`cairosvg`/libxml2 对全部 12 个非法引用都返回 1。按 XML 1.0 `Char` 产生式，fail-closed
是正确的一边；resvg 的宽松不构成「放行」的理由。

## 根因

- 文本路径：`src/dom.rs` 的 `Event::GeneralRef` 分支把 `&{reference};` 交给
  `quick_xml::escape::unescape`。`unescape` 接受 `&#1;` 这类「语法合法、码位非法」的引用，
  返回裸 `\u{1}`，随后被 `write_element` 原样写进产物。
- 属性路径：`element_from_start` 的 `decode_and_unescape_value` 同样只做语法解码，不检查
  XML 1.0 `Char`。
- 两条路径各自实现，没有共享判据；R47 只改了文本路径的解码方式，没有补 `Char` 检查。

## 修法

1. **一份判据**：`src/dom.rs` 新增 `pub(crate) fn is_xml_char(c: char) -> bool`，严格按
   XML 1.0 `Char`：`#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`。
   `src/convert/dom.rs` 通过 `use crate::dom::{…}` 调用同一份，不复制逻辑。
2. **文本路径**：`decode_reference` 在 `unescape` 成功后检查 `first_invalid_xml_char`；
   非法时返回 `character reference &#1; decodes to U+0001, which is not a legal XML 1.0
   character`。`unescape` 自己拒绝的（`&#0;`、代理区、溢出）也补上引用原文与码位：
   `character reference &#0; (U+0000) is not a legal XML 1.0 character: …`。
3. **属性路径**：`element_from_start` 先扫原始值里的引用（`invalid_reference_in_raw`），
   再检查解码后的值；文案带属性名：`attribute \`data-x\`: …` / `attribute \`data-x\`
   contains U+0001, …`。
4. **错误类型**：`SvgGridError::Xml` 的载荷从 `quick_xml::Error` 改为 `String`，以便携带
   上述文案；`Display` 前缀 `SVG XML error: ` 与 `ConvertError::Xml` 的 `XML error: `
   保持不变，rc 仍为 1。
5. **字面控制字符**：`Event::Text` / `Event::CData` / 解码后的属性值也走同一判据，
   保证「产物可重新解析」这条不变量对字面控制字符同样成立（语料里 0 个，无回归风险）。

## 合法面 / 非法面（逐条测试）

- 合法（rc=0、产物可重新解析）：`&#9;`、`&#10;`、`&#13;`、`&#32;`、`&#65;`、`&#x1F600;`、
  `&#xFFFD;`、`&#x10FFFF;`、`&#xE000;`、`&#xD7FF;`。
- 非法（rc=1、不写产物，文本与属性各一遍）：`&#0;`、`&#x0;`、`&#1;`、`&#8;`、`&#11;`、
  `&#12;`、`&#31;`、`&#xFFFE;`、`&#xFFFF;`、`&#55296;`（`&#xD800;`）、`&#xDFFF;`、
  `&#x110000;`。
- 强不变量：`tests/selftest/textref/mod.rs::every_accepted_output_is_well_formed` 对
  13 个文本用例 + 12 个属性用例 × `convert`/`plot-grid` 逐个断言：rc=0 ⇒ 产物用
  `quick_xml::Reader` 全程 `read_event` 无 `Err` 且字节里无非法 XML 字符；rc≠0 ⇒ rc=1
  且不写产物。

## 变异表

| 变异 | 改了哪一行 | 红了哪些测试 |
|---|---|---|
| M1 判据恒 `true`（放行） | `src/dom.rs` `is_xml_char` 函数体 → `true` | lib 6：`dom::tests::the_xml_char_predicate_matches_the_xml_1_0_production`、`dom::tests::illegal_character_references_name_the_reference_and_codepoint`、`dom::tests::illegal_character_references_in_attributes_name_the_attribute`、`dom::tests::literal_control_characters_are_rejected_in_text_and_attributes`、`convert::dom::tests::illegal_character_references_name_the_reference_and_codepoint`、`convert::dom::tests::illegal_character_references_in_attributes_name_the_attribute`；selftest 5：`textref::convert_rejects_illegal_character_references_in_text`、`textref::convert_rejects_illegal_character_references_in_attributes`、`textref::plot_grid_rejects_illegal_character_references_in_text`、`textref::plot_grid_rejects_illegal_character_references_in_attributes`、`textref::every_accepted_output_is_well_formed` |
| M2 判据恒 `false`（全拒） | 同上 → `false` | lib 181 / selftest 152（判据同时守所有文本/属性值，属预期的大面积）；14 条 R50 新测试全红，代表：`dom::tests::legal_character_references_stay_accepted`、`dom::tests::character_references_decode_to_their_characters`、`dom::tests::the_five_predefined_entities_still_decode`、`dom::tests::whitespace_character_references_are_passed_through`、`convert::dom::tests::legal_character_references_stay_accepted`、`textref::legal_character_references_stay_accepted`、`textref::every_accepted_output_is_well_formed` |
| M3 只修文本路径、不修属性路径 | `src/dom.rs` / `src/convert/dom.rs` 的 `element_from_start` 回退到 R47 前形态（不扫原始引用、不检查解码值） | lib 3：`dom::tests::illegal_character_references_in_attributes_name_the_attribute`、`dom::tests::literal_control_characters_are_rejected_in_text_and_attributes`、`convert::dom::tests::illegal_character_references_in_attributes_name_the_attribute`；selftest 3：`textref::convert_rejects_illegal_character_references_in_attributes`、`textref::plot_grid_rejects_illegal_character_references_in_attributes`、`textref::every_accepted_output_is_well_formed` |

## 数字

- 回归：`cargo test --offline` **496** = 272 lib + 59 main + 165 selftest（基线 482，
  +14，未删任何既有测试）；`cargo fmt --all --check` OK；
  `cargo clippy --offline --all-targets -- -D warnings` OK。
- 安装：`./install.sh` 后 `sha256sum ~/.local/bin/svg_grid target/release/svg_grid` 同为
  `57cb41b01f804384d8b1de34b13e46bb95434fbe9d6cb54dfb7d9a8fb44314c0`。
- `svg_grid doctor`：**57 pass / 0 fail / 1 skip**。
- 语料逐字节：220 个 `tests/**/*.svg` 的 `convert` 产物 + 58 个
  `tests/fixtures/**/*.svg` 的 `plot-grid --width 600 --ncol 1` 产物，对 base
  `/tmp/r46base/target/release/svg_grid`（@`196da47`）**0 差异**（语料 0 个数字实体）。
- 旧证据：R3..R49 顶层跑 **44/45 绿**，仅 R19 既有失效（已 `retire`，不计入）。
- 单文件：`src/dom.rs` 16969 B、`src/convert/dom.rs` 11821 B、
  `tests/selftest/textref/mod.rs` 15828 B、`src/error.rs` 4221 B，均 ≤20000 B。

## 账本更正（R47）

- R47-F7 `lines_after`：`1-413` → **`1-442`**（`git show 8aca0d1:docs/PITFALLS/round-data/R47/test.py | nl -ba | tail -1` = 442）。
- R47-F1 `src/dom.rs:parse`：`80-96` → **`174-186`**（R50 在 `parse` 前新增判据/解码辅助函数）。
- R47-F2 `src/convert/dom.rs:parse`：`82-93` → **`95-105`**。
- R47-F4 `src/dom.rs:tests`：`178-245` → **`280-459`**。
- R47-F5 `src/convert/dom.rs:tests`：`186-233` → **`207-321`**。
