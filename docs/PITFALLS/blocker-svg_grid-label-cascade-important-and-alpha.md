# 阻塞点：字母判据的 `!important` 级联、零 alpha 颜色与 `<symbol>` 实例化（R54 对抗评审收尾）

## 现象

R54 把字母查重的可见性判据按渲染器实测重写后，对抗评审用 d1render（resvg 0.47）+ cairosvg 又测出四类仍错判。
本轮全部按渲染器真值收口，并把 R54 自己留下的两处工程缺陷（证据脚本 fail-open、账本行号过期）一并修掉。

### P1（阻断）：同属性两条 `!important` 时取"后者"，与 resvg 相反

`src/ink/walk.rs` 的 `style_prop` 把 `important` 槽位直接赋值，于是**最后一个** `!important` 胜。实测 resvg 0.47
在同等 `!important` 中取**第一个**（cairosvg 取最后一个）。两片同输入、`<g style="…"><text data-svg-grid-label="0">A</text></g>`：

| style | d1 可见 | 修前 rc | 修后 rc |
|---|---|---|---|
| `display:block !important;display:none !important` | 是（23） | 0（漏判） | **2** |
| `display:none !important;display:block !important` | 否（0） | 2（误报） | **0** |
| `display:none !important;display:block;display:block !important` | 否（0） | 2 | **0** |
| `display:block !important;display:none !IMPORTANT` | 是（23） | 0 | **2** |
| `visibility:visible !important;visibility:hidden !important` | 是（23） | 0 | **2** |

第三条证明要取的是"第一个 `!important`"而不是"第一个声明"；第四条证明 `!important` 的词法大小写不敏感
（这条**没动**，R51 已裁决）。原注释 `:134-139` 写着"两台渲染器在最后两条规则上一致"——是假的，已删。

### P2.2：零 alpha 颜色仍被当成出墨

`fill="#00000000"`、`fill="rgba(0,0,0,0)"`、`stop-color="#00000000"`、`stop-color="rgba(0,0,0,0)"` 两台渲染器
ink 都是 0，修前 rc=2。`#00000080`（半透明）仍算出墨（d1 dark 1），是"别改坏"的对照。

### P2.2：`<symbol>` 实例化的两个方向

- `<symbol style="display:none">` 经 `<use>`：resvg **照画**（d1 dark 23），修前 rc=0（漏判）⇒ 修后 rc=2。
  `display` 不作用于 `<symbol>` 自身（SVG 1.1），但作用于 `<g>` 实例（对照仍隐藏）。
- `<symbol viewBox="0 0 6 6">` 把 `y=20` 的文本裁掉：d1/cairo ink 都是 0，修前 rc=2（R54 新引入的误报）⇒ 修后 rc=0。
  `viewBox="0 0 20 20"` 时文本仍在框内（d1 dark 91）⇒ 仍 rc=2。

### P2.3 / P2.4：R54 自己的两处缺陷

- `docs/PITFALLS/round-data/R54/test.py` 的两处早退写成 `return check(False, …)`，`check` 返回 `False`，
  `sys.exit(False)` ⇒ **退出码 0**（实测 `SVG_GRID_BIN=/nonexistent` 打印 FAIL 却 exit 0，fail-open）。
- 账本 R54-F5/F6/F8 的 `lines_before` 写的是 `197-236` / `158-191` / `190-208`，base `3532759` 的实际值是
  `221-260` / `182-215` / `266-284`（即 R51 时代的 `lines_after`）。

### P2.1（行为变更披露）：同一面板内两个 marker 都标同一个字母不再判重

单片 `<g data-svg-grid-label="0"><text>A</text><text data-svg-grid-label="1">A</text></g>`：
R54 前 rc=2，R54 起 rc=0。渲染器确实画两个 A（d1 dark 46 / nonwhite 80，cairo 28/106）。

按 R54 的整棵子树口径，外壳读 "AA"、内层读 "A"，两者不同 ⇒ 不判重，**语义自洽**（面板画的是 "AA"，
不是两个独立的 "A" 字母）。但 R54 直接换掉了旧断言 `cli_rejects_a_real_duplicate_between_an_outer_and_a_nested_marker`
却没在报告里披露。本轮：① 用渲染器钉住新口径（`tests/selftest/labels/r56.rs::cli_counts_one_letter_per_drawn_glyph_under_a_covering_marker`
做 PNG 逐字节对照 + 单片 rc=0 + 双片 rc=2 且报文同时给出 `"AA"` 与 `"A" at #0 (cell 1)`）；② 在此明确披露。

## 复现

```bash
cd tools/svg_grid && cargo build --release --offline
python3 docs/PITFALLS/round-data/R56/test.py            # 122 passed, 0 failed
SVG_GRID_BIN=/tmp/r56bin/svg_grid_before \
  python3 docs/PITFALLS/round-data/R56/test.py          # 修前二进制：107 passed, 15 failed
SVG_GRID_RENDER_BIN=/nope python3 docs/PITFALLS/round-data/R56/test.py   # 渲染器缺失：exit 1
```

`/tmp/r56bin/svg_grid_before` 由 `git worktree add /tmp/r56base 1db03ac && cargo build --release --offline` 得到。

## 期望语义（渲染器实证，不是选择）

- 同一 `style` 内：普通声明**后者胜**；`!important` 声明**第一个胜**；`!important` 压普通；`/* … */` 剥掉。
- `!important` 的词法大小写不敏感（`!IMPORTANT`/`!Important`/`! important` 都算）；`!` 后面跟垃圾
  （`display:none !`、`display:none ! foo`）也算 important 标记——resvg 把 `!` 之后的一切丢掉。
- 颜色的 alpha 通道为 0 ⇒ 不出墨（`#rrggbbaa`、`rgba()`，`stop-color` 同路）。
- gradient 的 `href` 链没有固定深度上限，环要能停。
- `display` 不作用于 `<symbol>` 自身；`<symbol viewBox>` 把内容裁掉时实例不出墨。
- 两台渲染器分歧处一律跟 resvg（R51 先例）：`!important` 取第一个、`#rrggbbaa` 的 alpha、`display:none` 的 symbol、
  `display:none /* c */` 的注释剥离、`DISPLAY:none` 的大小写。cairosvg 只作对照列。

## 四列实测（`round-data/R56/test.py` 打印的同一张表）

"修前" = `/tmp/r56bin/svg_grid_before`（`1db03ac`），"修后" = 本轮二进制。ink 为 100×40 白底面板的像素计数
（`dark` = 三通道都 <128，`nonwhite` = 任一通道 <255）。

| 用例 | 修前 rc | 修后 rc | d1 dark | d1 nonwhite | cairo dark | cairo nonwhite |
|---|---|---|---|---|---|---|
| P1a 第一个 important block 胜 | 0 | **2** | 23 | 40 | 0 | 0 |
| P1b 第一个 important none 胜 | 2 | **0** | 0 | 0 | 14 | 53 |
| P1c 取第一个 important（非第一个声明） | 2 | **0** | 0 | 0 | 14 | 53 |
| P1d `!IMPORTANT` 也算 important | 0 | **2** | 23 | 40 | 0 | 0 |
| P1e visibility 同规则（可见） | 0 | **2** | 23 | 40 | 0 | 0 |
| P1f visibility 同规则（隐藏） | 2 | **0** | 0 | 0 | 14 | 53 |
| P1g 普通声明后者胜（对照） | 2 | 2 | 23 | 40 | 14 | 53 |
| P1h important none 压普通 block（对照） | 0 | 0 | 0 | 0 | 0 | 0 |
| P1i important block 压普通 none（对照） | 2 | 2 | 23 | 40 | 14 | 53 |
| P1j 普通声明后者 none（对照） | 0 | 0 | 0 | 0 | 0 | 0 |
| P1k 孤立 `!` 算 important | 2 | **0** | 0 | 0 | 14 | 53 |
| P1l `! foo` 也算 important | 2 | **0** | 0 | 0 | 14 | 53 |
| P2a `#00000000` | 2 | **0** | 0 | 0 | 14 | 53 |
| P2b `rgba(0,0,0,0)` | 2 | **0** | 0 | 0 | 0 | 0 |
| P2c gradient stop `rgba(0,0,0,0)` | 2 | **0** | 0 | 0 | 0 | 0 |
| P2d gradient stop `#00000000` | 2 | **0** | 0 | 0 | 14 | 53 |
| P2e `#00000080` 半透明仍出墨 | 2 | 2 | 1 | 40 | 14 | 53 |
| P2f `display:none` 的 symbol 经 use 照画 | 0 | **2** | 23 | 40 | 0 | 0 |
| P2g 10 层 gradient href 链 | 0 | **2** | 23 | 40 | 14 | 53 |
| P2h 环形 href 不出墨 | 0 | 0 | 0 | 0 | 0 | 0 |
| P2i 覆盖 marker 计两个字形（同字母） | 2 | 2 | 46 | 80 | 28 | 106 |
| P2k 覆盖 marker 计两个字形（不同字母） | 2 | 2 | 49 | 80 | 38 | 111 |
| P3a `fill-opacity=" 0 "`（登记） | 0 | 0 | 23 | 40 | 0 | 0 |
| P3b `stroke-width="1e-9"`（登记） | 2 | 2 | 0 | 0 | 0 | 0 |
| P3c `fill-opacity="1e-4"`（登记） | 2 | 2 | 0 | 0 | 0 | 0 |
| P3d obb 零尺寸 pattern（登记） | 2 | 2 | 0 | 0 | 0 | 0 |
| P3e symbol viewBox 裁掉文本 | 2 | **0** | 0 | 0 | 0 | 0 |
| P3f symbol viewBox 保留文本 | 2 | 2 | 91 | 122 | 74 | 167 |

## P3 逐条处置（修了 / 登记了）

| 项 | 处置 | 依据 |
|---|---|---|
| `fill-opacity=" 0 "` 等属性尾随空格 | **登记** | resvg 的属性解析器把尾随空格当非法值 ⇒ 用初始值 ⇒ 照画（d1 dark 23）；cairosvg 接受（0）。我们 `parse_number` 两端 trim ⇒ 隐藏。要跟 resvg 得写严格属性解析器，且"隐藏"是查重场景的 fail-closed 方向，故只登记。`style="fill-opacity:0 "` 两台都隐藏（CSS 解析器会 trim），不受影响。 |
| `<symbol style="display:none">` 经 `<use>` | **修了** | d1 dark 23（照画）⇒ 修后 rc=2。 |
| gradient `href` 链深度 4 | **修了** | 10 层链 d1 dark 23 ⇒ 修后 rc=2；环形链仍 0/0。 |
| `stroke-width="1e-9"` / `fill-opacity="1e-4"` | **登记** | 无像素但计出墨（rc=2），fail-closed 方向，可接受。 |
| obb 零尺寸 pattern | **登记** | 需 bbox 计算，超出本轮；d1/cairo 都 0。 |
| `<symbol viewBox>` 裁剪 | **修了** | 6×6 裁掉 ⇒ rc=0；20×20 保留 ⇒ rc=2。 |
| `<symbol viewBox="10 10 6 6">` 里文本仍被 resvg 画出 | **登记** | rect 在 (10,10) 正确裁剪（ink 1600）、(0,0) 裁掉（0），但文本在框外仍画（423）——resvg 自身怪癖，不建模。 |
| 报告"只有两处渲染器分歧" | **改正** | 实际分歧至少还有：双 `!important`、`rgba`/hex8 stop、`<symbol style="display:none">`、`fill-opacity=" 0 "`、`style=";display:none"`。 |
| "旧证据 47/47" | **改正** | 列表实为 48 轮（R3..R55，R19 已 retire）；本轮跑的是 52 轮（R3..R56）。规范路径下全绿。 |

## 变异（实跑，`/tmp/r56mut/run_mutations.py`，每条都列出变红的测试名）

| # | 变异 | 变红的测试 |
|---|---|---|
| M1 | `important.get_or_insert` → 覆盖（回到"最后一个 important 胜"） | `ink::walk::tests::style_prop_keeps_the_first_important_declaration`、`labels::r56::cli_keeps_the_important_keyword_case_insensitive`、`cli_lets_the_first_important_display_declaration_win`、`cli_lets_the_first_important_visibility_declaration_win`、`labels::visibility::markers_in_non_rendered_subtrees_do_not_collide` |
| M2 | `important.or(plain)` → `plain.or(important)`（important 不再压普通） | `ink::walk::tests::style_prop_keeps_the_first_important_declaration`、`labels::r54::cli_lets_an_important_block_win_over_a_plain_none`、`cli_lets_an_important_display_declaration_win`、`labels::r56::cli_keeps_the_plain_declaration_cascade`、`cli_lets_the_first_important_display_declaration_win` |
| M3 | `strip_comments` 直接返回原串 | `ink::walk::tests::style_prop_strips_comments_and_a_stray_bang`、`labels::r54::cli_ignores_display_none_wrapped_in_a_css_comment` |
| M4 | `split_important` 只认 `important` 尾巴（孤立 `!` 不剥） | `ink::walk::tests::style_prop_strips_comments_and_a_stray_bang`、`labels::r56::cli_strips_a_stray_bang_from_a_declaration` |
| M5 | `colour_alpha` 恒返回 `None`（忽略 alpha） | `label_paint::tests::a_zero_alpha_colour_paints_nothing`、`labels::r56::cli_ignores_a_zero_alpha_colour`、`cli_ignores_a_zero_alpha_gradient_stop` |
| M6 | `display_applies` 恒 true（symbol 的 display 仍生效） | `labels::r56::cli_draws_a_marker_on_a_display_none_symbol` |
| M7 | `gradient_paints` 恢复 4 层上限 | `label_paint::tests::a_gradient_href_chain_has_no_fixed_depth_and_stops_on_a_cycle`、`labels::r56::cli_follows_a_gradient_href_chain_without_a_depth_limit` |
| M8 | 覆盖 marker 永不丢弃（回到"外层总计入"） | `label_letters::tests::a_nested_marker_is_collected_once_by_its_own_element`、`bare_text_inside_a_group_is_not_drawn`、`labels::nested::cli_counts_a_nested_glyph_once_inside_a_covering_outer_marker`、`cli_counts_a_nested_marker_once`、`cli_counts_only_the_innermost_marker_of_three_levels`、`labels::r54::cli_collects_only_the_rendered_text_of_a_mixed_marker` |
| M9 | `instance_draws` 恒 true（忽略 symbol viewBox 裁剪） | `labels::r56::cli_ignores_a_symbol_whose_view_box_clips_the_text_away` |

## 回归

- `cargo test --offline`：**306 lib + 59 main + 233 selftest = 598**（基线 579 = 298 + 59 + 222；lib +8、selftest +11）。
- `cargo fmt --all --check` 干净；`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `bash install.sh` 后 `sha256sum ~/.local/bin/svg_grid target/release/svg_grid` 两侧同为
  `aad3b2f44a822a140fe01187071097078174bb290df812d4726f17bcc163e2ae`。
- `svg_grid doctor`：`summary: 57 pass, 0 fail, 1 skip`。
- 语料：220 个 `tests/**/*.svg` 的 `convert` 产物 + 220 个 `--report` stdout + 58 个 fixture 的
  `plot-grid --width 600 --ncol 1` 产物 + 58 个 `--plan` stdout，对 `/tmp/r50base/target/release/svg_grid`
  （`e0c6033`，sha `75def0b8…`）**逐字节 0 差异**。
- 旧证据：`docs/PITFALLS/round-data/R49/run_old_evidence.sh` 在规范路径下 **52/52 轮全绿**（R3..R56，R19 已 retire）。
- 单文件体积全部 < 20000 B：`src/label_letters.rs` 19376、`src/label_paint.rs` 14307、`src/label_clip.rs` 4827、
  `src/ink/walk.rs` 10647、`tests/selftest/labels/r56.rs` 12862、`docs/PITFALLS/round-data/R56/test.py` 16999。
