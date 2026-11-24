# 字母查重（`data-svg-grid-label`）的检测缺口

面板字母是"面板已经自带字母"的唯一信号：`plot-grid --labels` 只在没有字母的输入上盖字母，
所以 W3（`panels in … already carry letters`）与 W5（`duplicate panel letters`）必须和**真实渲染**
一致——画出来一个 A 就算一个字母，没画出来就不算。本文件记录这条判据与渲染语义对不齐的地方。

来源：R48 对抗评审（1 阻断 + 若干 P2/P3），R51 修复。

---

## R51（2026-11-24）字母查重的剩余缺口

修前 HEAD `cdcc726`，修后见 `git log`。参考渲染器：`/tmp/d1_eval/render/target/release/d1render`
（resvg 0.47.0）与 `~/.venv` 的 cairosvg；所有"该算/不该算"都以 ink 实测为准。

### A（阻断）嵌套 marker 被重复计数

**现象**：单输入、单字形，却被判成两个字母而拒稿（rc=2、不写产物）。

**复现**（`docs/PITFALLS/round-data/R51/test.py` A1）：

```xml
<g data-svg-grid-label="0"><text data-svg-grid-label="1" x="4" y="14"
    font-size="12" font-family="Arial">A</text></g>
```

- `1b7ef3d`（R48 前）：rc=0，写出产物，d1render 一个 A（ink 40）——正确。
- `cdcc726`（R48/R49 之后）：rc=2，无产物，
  `duplicate panel letters: "A" at #0 (cell 0) and #0 (cell 1)`——误判。

**机制**：`src/label_conflicts.rs:collect_text`（cdcc726 的 169-184 行）从外层 marker 起读**整棵子树**，
递归进了内层 marker 的 `<text>`，于是外层也拿到 `"A"`；`walk_letters`（142-164）本来就递归收内层 marker
⇒ 同一个字形产生两个 `LetterSite`。

**修法**：`collect_text` 不递归进自身带 `data-svg-grid-label` 的子元素。外层文本因此变空，被
`letter.is_empty()` 跳过；内层 marker 由 `walk_letters` 计一次。收集逻辑整体搬到新模块
`src/label_letters.rs`（`src/label_conflicts.rs` 只留策略），一个字形只有一个入口。

**覆盖**（`tests/selftest/labels/nested.rs`，每条断言 rc + 字母 + sites 文案 + 是否写产物）：

| 用例 | 期望 |
|---|---|
| A1 最小复现 | rc=0、一个字母、写出产物 |
| A2 外层 `A` + 内层 marker `B` + 第二个输入 `AB` | rc=0（修前外层会吞成 `"AB"` ⇒ rc=2）|
| A3 外层 `A` + 内层 marker `A` | 真重号，rc=2、sites `#0 (cell 0)` / `#0 (cell 1)`、不写产物 |
| A4 三层 `<g label=0><g label=1><text label=2>` | 只最内层计 `A`，rc=0；与裸 `A` 配对时 rc=2、`#0 (cell 2)` |
| A5 两个输入各一个嵌套 marker、同字母 | 仍 rc=2，sites 从 4 个降到 2 个：`#0 (cell 1)` / `#1 (cell 0)` |

### B（P2）`<defs>` + `<use>` 假阴性

**现象**：定义在 `<defs>` 里的字母被 `<use>` 实例化、**确实画出来**，却不参与查重（rc=0 放行）。

**复现**：

```xml
<defs><text id="L" data-svg-grid-label="0" x="4" y="14" font-size="12">A</text></defs>
<use href="#L"/>
```

再配一个可见的 `A`（marker `1`）：`1b7ef3d` rc=2，`cdcc726` rc=0。

**参考渲染器**：defs 单独 ink 0；`<use>` 一次 ink 40；两次同位置仍 40（重叠）；两次错开 `x=30`
ink 80 ⇒ **每个 `<use>` 实例化算一次字母**，未被引用的 defs 不算。`<symbol>` 同样：不引用 ink 0，
引用 ink 40。

**修法**：`src/ink/uses.rs:use_target` 提升为 `pub(crate)`（与 ink 模块共用一份 `href`/`xlink:href`
解析，不复制第二份），`label_letters::expand_use` 在每个 `<use>` 处把被引用子树按同一套
`collect_text`/`walk_letters` 再走一遍；`Ctx.ancestors` 用 `std::ptr::eq` 记录展开链，目标已在链上就不再展开
⇒ 自引用/互引用环只算一次；`MAX_USE_DEPTH=16` 兜底；`NON_RENDERED` 目标（如 `<symbol>`）走它的子元素。

**覆盖**（`tests/selftest/labels/uses.rs`）：一 use 一次、两 use 两次（`"A" 2 times inside #0 (cell 0)`）、
未被引用的 defs 不算（rc=0 且写产物）、自引用与互引用各只展开一次（rc=0）、`<symbol>` 只在被 use 时算。

### C（P3）NBSP 过度规范化 + 零宽字符漏判

**现象**：`A` 与 `A&#160;`（U+00A0）被判重号（rc=2），但两者的渲染 PNG **diff 0**（`1b7ef3d` rc=0）。
反向：`A` 与 `A\u200b`（U+200B）都 rc=0（漏判），渲染同样 diff 0。

**口径**：SVG 默认 `xml:space="default"` 只折叠 ASCII 空白（U+0020/U+0009/U+000A/U+000D），
U+00A0 是**字符**不是可折叠空白；零宽字符不产生墨迹也不产生步进，肉眼与像素都无法区分，应视为同一字母。

**修法**：`src/lib.rs:normalize_label` 先删除 U+200B/U+200C/U+200D/U+FEFF，再只按上述四个 ASCII
空白字符 trim + 折叠（U+00A0 保持显著）。

**覆盖**（`tests/selftest/labels/text.rs`）：`A` vs `A&#160;` ⇒ rc=0 且写产物；两个 `A&#160;` 仍 rc=2；
`A` vs `A\u200b` ⇒ rc=2、sites `#0 (cell 0)` / `#1 (cell 0)`、不写产物。

### D（P3）`!important` 与关键字大小写

**现象**：`style="display:none !important"` 的 text 渲染 ink 0，却被计入查重（rc=2）。四种合法写法
（`!important`、`!IMPORTANT`、`! important`、`none!important`）都被误伤。

**修法**：`src/ink/walk.rs:style_prop` 对 `style` 声明先剥尾部 `!important`（允许前后空白、任意大小写），
再按属性名取值。

**关键字大小写——按渲染器裁决（不改）**：`style="display:NONE"`、`style="DISPLAY:none"`、
`display="NONE"`、`visibility="HIDDEN"` 在 d1render 中都 ink 40（**确实画出来**），因此它们必须继续参与
查重（rc=2）。CSS 规范说属性名与关键字都应大小写不敏感，**resvg 与 cairosvg 都不合规**（cairosvg 只在
属性名上合规：`style="DISPLAY:none"` 它 ink 0、resvg ink 40）。本项目一贯以"画出来的结果"为准
（R38–R41 的教训），并取 fail-closed：宁可拒稿，也不放过"画了两个 A"。该裁决记为
`R51-F9`（wontfix），后续不得按 CSS 规范重开。

### E（P3）`fill="none"` 的 text 被计入

**现象**：`<text data-svg-grid-label="0" fill="none">A</text>` 无 fill 无 stroke，d1render ink 0，
却被计入（rc=2，`1b7ef3d` 同）。

**修法**：`label_letters::Paint` 沿树继承 `fill`/`stroke`/`fill-opacity`/`stroke-opacity`/`stroke-width`，
只有 `visible && paint.any()` 才收字。`fill="none"` + `stroke="black"` 仍算（ink 61）；`<g fill="none">
<text fill="black">` 可由后代恢复（ink 40），与 `visibility` 一样的继承语义。

### F 账本更正（append-only）

- `R47-F7` 把 `docs/PITFALLS/round-data/R47/test.py` 记成 `1-413`，实际 **442** 行（`R51-F6`，`related = [R47-F7]`）。
- R48 报告声称"10 条 `modify --by R48`"，账本里实际只有 **4** 条（R31/R33/R34/R44）（`R51-F7`）。

### 三列数字（`1b7ef3d` 基线 / `cdcc726` 修前 / 修后 / d1render）

| 用例 | 1b7ef3d | cdcc726 | 修后 | d1 ink（cairo）|
|---|---|---|---|---|
| A1 嵌套 marker 单输入 | 0 | **2** | 0 | 40（45）|
| A2 外层 A + 内层 B + 输入 `AB` | 0 | **2** | 0 | 57（62）|
| A3 外层 A + 内层 A（真重号）| 2 | 2 | 2 | 40（45）|
| A4 三层嵌套 | 0 | **2** | 0 | 40（45）|
| A5 两输入各一嵌套 A | 2 | 2 | 2 | 40（45）|
| B1 defs+use vs marker 1 | 2 | **0** | 2 | 40（45）|
| B2 defs+两次 use | 0 | **0** | 2 | 80（90）|
| B3 未引用的 defs | 2 | 0 | 0 | 0 |
| C1 `A` vs `A&#160;` | 0 | **2** | 0 | 40（45），xor 0 |
| C2 `A` vs `A\u200b` | 0 | **0** | 2 | 40（45），xor 0 |
| D1 四种 `!important` | 2 | 2 | 0 | 0 |
| D2 `style="display:NONE"` | 2 | 2 | 2 | 40（45）|
| D3 `style="DISPLAY:none"` | 2 | 2 | 2 | **40（0）** |
| D4 `display="NONE"` | 2 | 2 | 2 | 40（45）|
| D5 `visibility="HIDDEN"` | 2 | 2 | 2 | 40（45）|
| E1 `fill="none"` | 2 | 2 | 0 | 0 |
| E2 `fill="none" stroke="black"` | 2 | 2 | 2 | 61（65）|
| E3 `fill="none" stroke="none"` | 2 | 2 | 0 | 0 |

D3 两台渲染器分歧：resvg 画（40），cairosvg 不画（0）——见上"D 项裁决"。

### 变异（改了哪一行 → 红了哪些测试）

| 变异 | 红的测试 |
|---|---|
| M1 `collect_text` 去掉"不递归进内层 marker" | 6：`label_letters::tests::a_nested_marker_is_collected_once_by_its_own_element`、`an_outer_marker_keeps_its_own_text_next_to_a_nested_one`、`labels::nested::cli_counts_a_nested_marker_once`、`cli_counts_only_the_innermost_marker_of_three_levels`、`cli_reads_the_outer_marker_text_next_to_a_nested_one`、`cli_rejects_a_real_duplicate_between_an_outer_and_a_nested_marker` |
| M2 `if element.name == "use"` → `if false && …`（回到无条件跳 defs）| 5：`a_definition_counts_once_per_use`、`a_symbol_is_drawn_through_use`、`labels::uses::cli_counts_a_definition_once_per_use`、`cli_counts_two_use_instantiations_separately`、`cli_draws_a_symbol_only_through_use` |
| M3 `normalize_label` 换回 `split_whitespace` | 2：`labels::text::cli_keeps_a_no_break_space_significant`、`cli_strips_zero_width_characters_before_comparing` |
| M4 display/visibility 关键字改小写比较 | 2：`label_letters::tests::important_and_case_follow_the_renderer`、`labels::visibility::important_is_stripped_and_uppercase_keywords_stay_visible` |
| M5 `strip_important` 原样返回 | 2：同上（D 的两种误伤各由一条测试守住）|
| M6 `visible && paint.any()` → `visible` | 2：`label_letters::tests::text_that_paints_nothing_is_not_a_letter`、`labels::visibility::text_that_paints_nothing_is_not_a_letter` |

### 回归

`cargo test --offline` **517**（280 lib + 59 main + 178 selftest；基线 496，+21，未删既有测试）；
`cargo fmt --all --check` OK；`cargo clippy --offline --all-targets -- -D warnings` OK；
`./install.sh` 后两处 `svg_grid` sha256 一致；`svg_grid doctor` 57 pass / 0 fail / 1 skip；
220 个 `tests/**/*.svg` 的 `convert` + 58 个 `tests/fixtures/**/*.svg` 的 `plot-grid --width 600 --ncol 1`
与 base `196da47` 逐字节 0 差异；旧证据 R3..R50 45/46 绿（R19 已 retire）。

### 不做（留给下一轮）

- `tspan x`/`dy` 重排：`collect_text` 按文档序拼字，而视觉序由 `x`/`dy` 决定；重排需要视觉序模型
  （字体度量 + 逐 tspan 定位），本轮不实现。建议记入 `PROGRESS.md` 的 Known Issues，等有真实语料再动。
- `DISPLAY:none` 一类"渲染器互相矛盾"的写法：已按 resvg + fail-closed 裁决，若将来需要跟 cairosvg
  对齐，必须整条判据一起换，不许只改一处。
