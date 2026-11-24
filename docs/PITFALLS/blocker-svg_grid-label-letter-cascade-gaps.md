# R54 — 组合图字母查重的"误伤 / 漏判"收口

- 轮次：R54（R51 对抗评审 P1）
- 修前：`03b2d59`（本轮工作树父提交为 `904ea0d`，仅动 HARNESS）
- 修后：本报告所在提交
- 证据：`docs/PITFALLS/round-data/R54/{test.py,meta.toml}`（35 例 × 修前 rc / 修后 rc / d1 ink / cairo ink）
- 裁决依据：resvg 0.47（`d1render`）为主，cairosvg 2.9.1 只作对照列；两台分歧按 R51 先例一律跟 resvg

## 0. 本轮裁决（新语义）

1. **一个 marker 的字母 = 它整棵子树里渲染器真正画出的文本拼接**，嵌套 marker 的文本也算在内。
2. **外层只在"自己没有贡献任何字符"时丢弃**：若整棵文本恰好等于全部内层 marker 文本的拼接，则只保留内层
   （`<g label="0"><text label="1">A</text></g>` 只算一个 `"A"`）。
3. **文本只在文本上下文里被画**：`<text>` 及其 `tspan/textPath/a` 后代才算；`<g>` 里的裸字符数据渲染器不画
   （实测 `<g>A<text label="1">B</text>C</g>` 与只画 `B` 的图逐像素相同，diff=0），所以这类裸字符不进字母。
4. **画不出的不算字母**：`url(#…)` 解析不到且无回退色、渐变/pattern 所有 stop 全透明、继承来的
   `fill-opacity`/`stroke-opacity` 为 0、`stroke-width<=0`，两台渲染器 ink 都为 0。
5. **style 级联**：先剥 `/* … */` 注释；同一 `style` 内后声明覆盖前声明；带 `!important` 的声明压过不带的
   （同带 `!important` 仍后声明胜）；`style` 里没有该属性才回落到属性。
6. **`<symbol>` 上的 marker 经 `<use>` 实例化仍是一个字母**（symbol 是唯一"自身非渲染、但可被 use 画出"的容器；
   `defs/clipPath/mask/pattern/marker/filter` 经 use 都不画，实测 ink 0）。

## 1. A 类：ink=0 却判重（误伤）⇒ 修成 rc=0

| 用例 | BODY 关键属性 | 修前 rc | 修后 rc | d1 dark/nonwhite | cairo dark/nonwhite |
|---|---|---|---|---|---|
| A1 | `<text … fill="url(#missing)">A` | 2 | 0 | 0/0 | 0/0 |
| A2 | `<text … style="fill:url(#missing)">A` | 2 | 0 | 0/0 | 0/0 |
| A3 | `<g fill="url(#missing)"><text label=0>A` | 2 | 0 | 0/0 | 0/0 |
| A4 | `<text … fill="none" stroke="url(#missing)">A` | 2 | 0 | 0/0 | 0/0 |
| A5 | `<text … fill-opacity="0">A`（无 fill） | 2 | 0 | 0/0 | 0/0 |
| A6 | `<g fill-opacity="0"><text label=0>A` | 2 | 0 | 0/0 | 0/0 |
| A7 | `<g stroke-opacity="0">` + `fill="none" stroke="black"` | 2 | 0 | 0/0 | 0/0 |
| A8 | `<g stroke-width="0">` + `fill="none" stroke="black"` | 2 | 0 | 0/0 | 0/0 |

根因两条，都已复现：

- `paint_prop` 只认字面 `none`/`transparent`，从不解 `url(#id)`。实测真值：`url(#missing)` 0/0；
  `url(#missing) black` 两支都画（回退色生效）；`url(#r)` 指向 `<rect>`（不是 paint server）0/0；
  `url(#g)` 指向合法黑渐变画；空渐变 / 全 `transparent` stop / `stop-opacity="0"` 全 0/0。
- `Paint::inherit` 只继承 fill/stroke 两个布尔，有效 `fill-opacity` / `stroke-opacity` / `stroke-width` 只看
  当前元素。实测 `fill-opacity="0"` + `stroke="black"` ink 61（描边不受 fill-opacity 影响），
  `<g stroke-opacity="0"><text stroke-opacity="1">` ink 61（后代可恢复）。

修法：新增 `src/label_paint.rs`（`Paint` 五字段 + `any()` 同时判有效透明度与线宽；`paints()` 解析 `url()`、
解析不到时看回退色、能解析时按 `linearGradient/radialGradient` 的 stop 与 `pattern` 的子元素判是否出墨，
`href` 链最多 4 层）。`src/label_letters.rs` 不再自带 `paint_prop`。

## 2. B 类：style 级联

| 用例 | style | 修前 rc | 修后 rc | d1 dark/nonwhite | cairo dark/nonwhite |
|---|---|---|---|---|---|
| B1 | `display:none /* c */` | 2 | 0 | 0/0 | **14/53** |
| B2 | `display:none;display:block` | 0 | **2** | 23/40 | 14/53 |
| B3 | `display:block;display:none` | 2 | 0 | 0/0 | 0/0 |
| B4 | `display:block;display:none !important` | 2 | 0 | 0/0 | 0/0 |
| B5 | `display:none !important;display:block` | 0 | 0 | 0/0 | 0/0 |
| B5b | `display:none;display:block !important` | 0 | **2** | 23/40 | 14/53 |
| B6 | `visibility:hidden;visibility:visible` | 0 | **2** | 23/40 | 14/53 |

- 真值用两台渲染器钉住：B2/B5b/B6 都是有墨（23/40 与 14/53），所以**它们本来就该被算成字母并判重**；
  原实现取"第一个同名声明"导致 B2/B5b/B6 漏判（rc=0），B3/B4 因为先声明是 `none` 而误判不可见（rc=2）。
- **B5b 是"后声明胜"和"!important 胜"必须分开实现**的判据：只做"后声明胜"它仍然漏判，只做"!important 胜"
  B2 仍然漏判。
- **B1 是分歧点**：cairo 不剥注释（保留 `none /* c */`? 实际按无效值处理 ⇒ 可见，ink 14/53），resvg 剥掉注释
  后是 `none`（ink 0/0）。按 R51 先例跟 resvg ⇒ rc=0。
- `style_prop` 同时被 `src/ink/text.rs:77,85` 的 `text-anchor` / `dominant-baseline` 使用；改成返回
  `Option<Cow<'a, str>>` 后那两处只加 `.as_deref()`，行为不变（全量测试无退化）。

## 3. C 类：`<symbol>` 上的 marker 经 `<use>` 丢失（漏判）

| 用例 | BODY | 修前 rc | 修后 rc | d1 dark/nonwhite | cairo dark/nonwhite |
|---|---|---|---|---|---|
| C1 | `<defs><symbol id="s" label=0><text>A</text></symbol></defs><use href="#s"/>` | **0** | 2 | 23/40 | 14/53 |
| C2 | 同上但 `<g id="s" label=0>` | 2 | 2 | 23/40 | 14/53 |

`expand_use` 原先在 `skip_marked` 且目标自身带 marker 时直接 return；`<symbol>` 又走 NON_RENDERED 分支只展开
子元素，marker 因此丢失。现在：实例化时 `NON_RENDERED` 里**只放行 `symbol`**（`defs/clipPath/mask/pattern/marker/
filter` 经 use 实测 ink 0/0，全部仍不展开），且不再对"目标自身带 marker"提前返回。

## 4. D 类：嵌套 marker

| 用例 | 组合 | 修前 rc | 修后 rc | d1 dark/nonwhite | cairo dark/nonwhite |
|---|---|---|---|---|---|
| D1 | 外层 `A`/`B`/`C` 三个 `<text>`（label 0 外层，label 1 中层） vs `<text label=0>ABC` | 0 | **2** | 71/122 | 48/162 |
| D1b | `<text label=0>ABC` 两片 | 2 | 2 | 71/121 | 48/159 |
| D2 | D1 的外层 vs `<text label=0>AC` | **2** | 0 | 71/122 | 48/162 |
| D3 | `<g label=0>A<text label=1>B</text>C</g>` vs `<text label=0>AC` | **2** | 0 | 26/40 | 24/58 |
| D3b | D3 的 fixture vs `<text label=0>B` | 2 | 2 | 26/40 | 24/58 |

- D1 证明**外层现在收整棵子树的文本**（"ABC"），所以与 `ABC` 判重、与 `AC` 不判重（方向都和旧实现相反）。
- D3 是派单原 fixture：`<g>` 里的裸 `A`/`C` 两台渲染器都不画（该 fixture 与只画 `B` 的图 **diff=0**），
  因此外层不贡献字符 ⇒ 外层被丢弃，只剩内层 `"B"`（D3b 与 `B` 判重、D3 与 `AC` 不判重）。
- 丢弃条件实现为"外层整棵文本 == 内层字母拼接"（`subtree` 返回值同时带整棵文本与内层字母表），
  不是"外层文本为空"：`<g label=0>A…B…C</g>` 外层文本是 `"ABC"` ≠ 内层 `"B"`，保留。

## 5. E 类：不许改坏的对照

| 用例 | 修前 rc | 修后 rc | d1 dark/nonwhite | cairo dark/nonwhite |
|---|---|---|---|---|
| E1 `fill="url(#missing)" stroke="black"` | 2 | 2 | 41/61 | 41/65 |
| E2 合法 linearGradient 引用 | 2 | 2 | 23/40 | 14/53 |
| E3 `<g fill-opacity="0"><text fill-opacity="1">` | 2 | 2 | 23/40 | 14/53 |
| E4 `<g stroke-width="0"><text stroke-width="1" …>` | 2 | 2 | 41/61 | 41/65 |
| E5 `<g fill-opacity="0.5">` | 2 | 2 | 1/40 | 0/53 |
| E6 `fill="none"` | 0 | 0 | 0/0 | 0/0 |
| E7 `stroke="none"` | 2 | 2 | 23/40 | 14/53 |
| E8 `<g display="NONE">` | 2 | 2 | 23/40 | 14/53 |
| E9 `<g style="DISPLAY:none">` | 2 | 2 | 23/40 | **0/0** |
| E10 `<g visibility="HIDDEN">` | 2 | 2 | 23/40 | 14/53 |
| E11–E14 `!important` 四种拼法 | 0 | 0 | 0/0 | 0/0 |

E9 是第二处分歧：cairo 把 `DISPLAY:none` 当 `display:none`（0/0），resvg 关键字大小写敏感（可见，23/40）。
R51 已裁决"跟渲染器大小写敏感"，本轮维持 rc=2，不翻案。

## 6. 测试

- `cargo test --offline`：**560** = 294 lib + 59 main + 207 selftest（基线 528 = 284 + 59 + 185；+32：lib +10，selftest +22）。
- 新增 `tests/selftest/labels/r54.rs`（22 条，`mod r54;` 注册）：A1–A8 各一条、B1–B6 + B5b、C1/C2、D1/D2/D3、
  E1–E14 全覆盖；断言 rc、stderr 片段、`!out.exists()`（拒收不落盘）与逐像素 PNG 对照
  （`--output-png` 走 resvg 0.47，等价参考渲染器）。
- 旧 R51 嵌套语义的两条按本轮裁决改写：`cli_reads_the_whole_subtree_into_the_outer_marker` 现在期望 rc=2
  （外层 `"AB"` 与另一片 `"AB"` 撞），`cli_counts_a_nested_glyph_once_inside_a_covering_outer_marker` 期望
  "覆盖外层不额外产字母"。
- **修前红、修后绿**：`git worktree add /tmp/r54before 03b2d59`，只拷贝三份测试文件，
  `cargo test --offline --test selftest labels::` = **47 passed / 12 failed**；12 条正是本轮 11 个新断言点
  + nested 1 条。全部在修复后转绿。

## 7. 变异（每条实跑，列出变红测试）

| 变异 | 变红的测试 |
|---|---|
| M1 `url()` 恒当可解析 | `labels::r54::cli_ignores_a_letter_painted_with_an_unresolved_reference` |
| M2 fill opacity 不继承 | `labels::r54::cli_ignores_a_letter_with_zero_opacity` |
| M2b stroke-opacity / stroke-width 不继承 | `labels::r54::cli_ignores_a_stroke_with_zero_opacity_or_width` |
| M3 `style_prop` 退回首声明胜 | `r54::cli_lets_an_important_block_win_over_a_plain_none`、`cli_lets_an_important_display_declaration_win`、`cli_lets_the_later_display_declaration_win`、`cli_lets_the_later_visibility_declaration_win`、`cli_still_honours_every_important_spelling`、`visibility::important_is_stripped_and_uppercase_keywords_stay_visible` |
| M4 不剥注释 | `r54::cli_ignores_display_none_wrapped_in_a_css_comment` |
| M5 `!important` 不参与优先级 | `r54::cli_lets_an_important_block_win_over_a_plain_none`、`cli_lets_an_important_display_declaration_win` |
| M6 `expand_use` 对 symbol 仍 return | `r54::cli_counts_a_marker_on_a_symbol_instanced_by_use` |
| M7 外层不递归嵌套文本 | `nested::cli_reads_the_whole_subtree_into_the_outer_marker`、`r54::cli_does_not_collide_an_outer_marker_with_its_trimmed_text`、`cli_reads_the_whole_drawn_subtree_into_an_outer_marker` |
| M8 外层永不丢弃 | `nested::cli_counts_a_nested_glyph_once_inside_a_covering_outer_marker`、`cli_counts_a_nested_marker_once`、`cli_counts_only_the_innermost_marker_of_three_levels`、`r54::cli_collects_only_the_rendered_text_of_a_mixed_marker` |

## 8. 全量回归

- `cargo fmt --all --check`：干净。
- `cargo clippy --offline --all-targets -- -D warnings`：干净。
- `./install.sh` 后 `sha256sum ~/.local/bin/svg_grid target/release/svg_grid` = 两处均
  `717e36201238e3738d42226670f38eb68a9cb610beaf4dd76e4f7b159918c9e6`。
- `svg_grid doctor`：`summary: 57 pass, 0 fail, 1 skip`。
- 语料对 base `/tmp/r50base/target/release/svg_grid`（`e0c6033`）：**0 差异**——
  220 个 `tests/**/*.svg` 的 `convert` 产物 + 220 个 `--report` stdout（base 有 214，HEAD 已增到 220，
  多出的 6 个也一并逐字节比了），58 个 `tests/fixtures/**/*.svg` 的 `plot-grid --width 600 --ncol 1` 产物 +
  58 个 `--plan` stdout。
- 旧证据：`docs/PITFALLS/round-data/R49/shim/` 严格 shim，`ran.txt` 表驱动 47 轮（R3–R11、R13–R18、R20–R26、
  R28–R53，R19 已 retire）**47/47 rc=0**。
- 文件体积：全部 ≤ 20000 B（`src/label_letters.rs` 18748、`src/label_paint.rs` 11124、`src/lib.rs` 18508；
  `src/dom.rs` 19027 未动）。

## 9. 偏离派单的决定（都有实测依据）

1. **B2/B5/B6 的方向按渲染器纠正**：派单写 B2=rc0、B5=rc2、B6=rc0；实测 B2/B5b/B6 有墨（23/40、14/53）⇒
   应当判重（rc=2），B5 无墨 ⇒ rc=0（原实现已对）。派单在收到我的复现后已确认采纳。
2. **D 类前提纠正**：派单说 `<g label=0>A<text label=1>B</text>C</g>` "渲染出 ABC"；实测这台 fixture 与
   只画 `B` 的图 diff=0，裸字符数据不渲染。真值 fixture 改为三个 `<text>` 兄弟（ink 71），原 fixture 单独钉
   "只算 B"。派单已确认。
3. **E 类多列两处渲染器分歧**（B1 注释、E9 `DISPLAY:none`），按 R51 先例跟 resvg，报告点名。
4. **R51 旧证据无需改写**：派单担心 R51 A2/A3 与本轮裁决冲突；实跑 R51 的 fixture 里内层文本也带 marker，
   外层恰好"没有自己的字符"而被丢弃，结果与 R54 一致（`R51/test.py` 47/47 仍绿，A2 rc=0、A3 rc=2 均未动）。
5. **`patternTransform` / `gradientTransform` / 滤镜图元长度 / 定义体 font-size 一律未碰**（父 agent 留到 R55），
   `src/ink/walk.rs` 里那段 `patternTransform` 注释与逻辑保持原样。
6. 语料规模从派单的 214 改为 220：HEAD 在 R52/R53 后新增了 6 个 `tests/**/*.svg`，逐个都比了，比"只比 214"
   更严。

## 10. 复现

```bash
cd tools/svg_grid && cargo build --release --offline
python3 docs/PITFALLS/round-data/R54/test.py      # 170 checks，0 fail
# 有牙：指向修前二进制，A/B/C/D 的"修后 rc"断言必须红
SVG_GRID_BIN=/tmp/r54bin/svg_grid_before python3 docs/PITFALLS/round-data/R54/test.py
# 渲染器缺失必须红（除非 --allow-skip）
SVG_GRID_RENDER_BIN=/nope python3 docs/PITFALLS/round-data/R54/test.py
```
