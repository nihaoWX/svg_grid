# R40 阻断点：非法 `patternContentUnits` 被当成 bbox + `<pattern viewBox>` 内容被二次缩放

- 轮次：R40（R39 复核 FAIL 后的修复轮）
- 基线：`96dfeab`（R39 入账、报告已删，git 干净）
- 上游裁决：`/tmp/r39_eval/evidence/README.md`，含 `repro_invalid_units.sh`、`repro_viewbox.sh`
- 唯一出货渲染器：usvg 0.47（resvg），证据渲染器 `/tmp/d1_eval/render/target/release/d1render`

## 现象 1（B1，R39 引入）：大小写/空白/未知值被当成 `objectBoundingBox`

判据写成"任何显式非 `userSpaceOnUse` 的值 = 按 bbox 处理"：

```rust
matches!(element.attr("patternContentUnits"), Some(units) if units.trim() != "userSpaceOnUse")
```

usvg 是**类型化**解析：`convert_units(node, name, def)` =
`resolve_attr(node, name)` + `node.attribute(name).unwrap_or(def)`
（`usvg-0.47.0/src/parser/paint_server.rs:230-233`）——区分大小写、不 trim、解析失败即取默认
`UserSpaceOnUse`。因此 `ObjectBoundingBox`、`objectBoundingBox `、` objectBoundingBox`、`foo`、
空串在 resvg 下**都按 user 空间缩放**，而 R40 前的代码把内容原样留下 ⇒ 产物少了一次 ±s 缩放，
且同一份文档在 base `75d59fb` 上是正确的。旧注释"leaving the body verbatim can never *add* a
wrong scaling"与本条实测相反，已随该判据一起删除。

复现（`bash /tmp/r39_eval/evidence/repro_invalid_units.sh`，s=2，panel 120×80，`d1render 2`）：

| `patternContentUnits` | 修前 new xor / ink | base75 xor / ink | 源图 ink | 修后 new xor / ink |
| --- | --- | --- | --- | --- |
| `ObjectBoundingBox` | 448 / 96 | 0 / 384 | 384 | 0 / 384 |
| `objectBoundingBox ` | 448 / 96 | 0 / 384 | 384 | 0 / 384 |
| ` objectBoundingBox` | 448 / 96 | 0 / 384 | 384 | 0 / 384 |
| `foo` | 448 / 96 | 0 / 384 | 384 | 0 / 384 |
| `""` | 448 / 96 | 0 / 384 | 384 | 0 / 384 |

body 是 `<rect x="2" y="3" width="6" height="4">`：修前原样留在 `2 3 6 4`（正确应为 `4 6 12 8`），
修后五个值全部写出 `<rect x="4" y="6" width="12" height="8" fill="#000000"/>`。

## 现象 2（P1-B，R38 起就有）：`<pattern viewBox>` 的内容坐标被乘 (sx,sy)

usvg 先按 `viewBox` 建立 viewBox→tile 变换（`paint_server.rs:1017-1043`：
`if let Some(view_box) = patt.view_box { push_pattern_transform(&mut root, view_box.to_transform(rect.size())) }`），
只有 `view_box.is_none()` 时才走 bbox 缩放的推入路径。tile 尺寸随 s 缩放，viewBox 映射自动跟上，
**内容数值在 viewBox 单位里，不能再乘 (sx,sy)**。R38/R39 只看 `patternContentUnits`，漏了这条。

复现（`bash /tmp/r39_eval/evidence/repro_viewbox.sh`，s=2，body `<rect x="2" y="2" width="6" height="6">`）：

| 用例 | 修前 | 修后 |
| --- | --- | --- |
| `vb_user`（仅 viewBox） | xor 1792 / md 255 / ink 1344（body 写成 `4 4 12 12`） | xor 0 / md 0 / ink 1728，body 逐字 `2 2 6 6` |
| `vb_obb`（viewBox + obb） | xor 0（巧合正确） | xor 0 |
| R38 `2a6a1ed` 同用例 | xor 1792 / ink 1344 | — |
| R37 同用例 | xor 1728 / ink 0（内容整段丢墨） | — |

源图 ink 1728。

## 现象 3：`patternContentUnits`/`viewBox` 沿 `href` 继承，此前没有解析

`resolve_pattern_attr`（`paint_server.rs:498-518`）沿 `href_iter()` 走 `xlink:href`/`href` 链，
返回**第一个显式设置了该属性**的 `<pattern>`（只看属性存在，不看取值是否合法），遇到非
`<pattern>` 目标即停。因此 `<pattern id="b" href="#a">` 可以继承 `a` 的 `patternContentUnits`
或 `viewBox`。R40 前完全不看 href，继承来的单位语义一律按 user 空间缩放。

## 根因

1. 判据用"非 userSpaceOnUse"近似"是 objectBoundingBox"，与 usvg 类型化解析不等价；
2. 只看 `patternContentUnits`，忽略 `viewBox` 会**改写内容的坐标单位**这一更强的事实；
3. 两个属性都可能来自 `href` 链，而合成器没有链解析。

## 修法

新增 `src/transform/patterns.rs`（`mod.rs` 已 16 KB，按 ARCHITECTURE 的 20 KB/文件规则单独成模块）：

- `PatternIndex::collect(root)`：一次遍历收集所有 `id` 与 `<pattern>` 的 `href`/`content_units`/`view_box`
  （重复 id 取第一个，与 usvg `element_by_id` 语义一致）。
- `pattern_body(element, index) -> PatternBody{Verbatim, UserSpace}`：从元素自身出发沿 href 链取
  第一个显式值，带 `visited` 防环；遇到非 `<pattern>` 目标停止；**链不可解析（id 不存在、
  外部 URL）时判 `Verbatim`**（fail-safe：宁可原样，也绝不额外加一次缩放；这是取舍，已在
  meta.toml 记明）。
- `valid_view_box(value)`：重写 svgtypes 的列表语法（`svgtypes-0.16.2/src/viewbox.rs:54-77`）——
  空格或单个逗号分隔、支持符号/小数/指数（`e` 后跟 `m`/`x` 时不是指数）、要求 4 个数且
  w>0、h>0、有限，第 4 个数之后的垃圾忽略。
- 判据：`view_box.is_some_and(valid_view_box) || content_units == Some("objectBoundingBox")`
  —— 精确 token、不 trim、区分大小写。
- `transform_element`/`rewrite_primitive` 各加一个 `patterns: &PatternIndex` 参数：
  pattern 分支与继承长度（`stroke-width`）的早返回都用 `pattern_body_is_verbatim`；
  userSpaceOnUse 分支仍走 `ScalePolicy::Scale`（R38/R39 行为逐字保留）。
- 旧 helper `pattern_content_is_object_bounding_box` 及其错误注释删除。

## 变异验证（每条新测试都有牙）

- M1 判据改回宽松（`u.trim() != "userSpaceOnUse"`）⇒ 只有
  `invalid_pattern_content_units_scales_like_user_space` 红（`left: <rect x="2" y="3" width="6" height="4"/>`
  vs `right: x="4" y="6" width="12" height="8"`）。
- M2 判据恒 `false` ⇒ 6 条红：两个 obb 逐字用例、继承长度、`view_box_pattern_body_stays_verbatim`、
  `pattern_view_box_is_inherited_through_href`、`pattern_content_units_is_inherited_through_href`。
- 脚本级（fail-closed）：`docs/PITFALLS/round-data/R40/test.py` 在
  `SVG_GRID_OLD_BIN=/tmp/r40_old_target/debug/svg_grid`（干净的 `96dfeab` 构建，worktree `/tmp/r40_old`）下
  E1 必须红 ⇒ 实测 6 条 B1/viewBox 全红，脚本 PASS；不给旧二进制则打 SKIP。
- 回归证据链：`docs/PITFALLS/round-data/R39/test.py` 追加 F 族（类型化 token + viewBox）后，
  在 `96dfeab` 二进制上 `== 7 条不成立 ==`（F1×5、F2、F3），在当前树上 `== 全部成立 ==`。
  放进 R39 而不是 R38 的理由：R38 证据只谈相位/平移，单位语义是 R39 才写进代码的；R38 证据
  保持原样即可，勿混入新语义。
- `convert` 侧用例 `object_bounding_box_pattern_body_keeps_its_fractions` **不设合成层防护**
  （评审实测：合成器回归时它不会红），本轮不改 convert crate，也不把它与本轮新单测并列。

## 验证

- 单测：root `cargo test` 270 = 102 lib + 59 main + 109 selftest（0 fail，R39 为 259）；
  convert `cargo test` 121 = 107 lib + 14 CLI（0 fail，未改 convert）。
- `cargo fmt --all --check` OK；两个 crate `cargo clippy --offline --all-targets -- -D warnings` OK；
  `./install.sh` OK；`svg_grid doctor` **57 pass / 0 fail / 1 skip**（matplotlib/mathtext 旋转祖先下的 text）。
- 220 个 `tests/**/*.svg` 的 convert 产物对 base `75d59fb`：文件 220、双方失败各 41、new-only 0、
  base-only 0、字节变化 0、幂等问题 0、悬空引用 0。
- 58 个 `tests/fixtures/**` 的组合（`--width 600`）：失败 1（`mathtext.svg` 双方 rc=2）、
  字节差 **0**（`bars_grouped.svg` 本轮也逐字节相同，md5 `717a14646cf2e7b1621723a0b39fdc09`）。
- R38 证据（bars_grouped / S36 的 resvg 对照）不回退，本轮实测：`bars_grouped` xor 442 / maxdiff 7、
  ink 15878 = 15878（base75：xor 5589 / maxdiff 255、ink 15878 vs 13334）；`S36_panelA_matrix`
  xor 357 / maxdiff 2、ink 28223 = 28223（base75：xor 448 / maxdiff 132、ink 28223 vs 28193）。
  R38 脚本整体 rc=0、无 SKIP。
- 旧证据 R8/R9/R16/R25/R28/R31/R35/R36/R37/R38/R39 顶层复跑全部 rc=0（日志 `/tmp/r40_evidence/`；
  耗时 6s / 15s / 283s / 981s / 0s / 0s / 0s / 0s / 14s / 16s / 2s；R25 内含 R4–R17 表驱动嵌套，
  R39 仅 E1 SKIP，原因已在脚本里写明）。

## 遗留（未修，本轮确认未恶化）

1. 嵌套 `<pattern>`（xor 11808）；
2. `patternTransform` 长度量（xor 1561）——`patternTransform` 里的平移不随 tile 缩放，仍有量级差；
3. pattern 之外的 `clip-path`（xor 20736）；
4. pattern 内 `<text>`（源图即 ink=0，两家渲染器都不出墨）；
5. `patternUnits="objectBoundingBox"` 的内容语义在两渲染器分歧（以 resvg 为准）；
6. **同类隐患**：`maskContentUnits`（`units.trim() != "userSpaceOnUse"`）与
   `patternUnits`/`maskUnits`/`filterUnits` 的 `user_space_units` 仍是 trim 宽松比较，
   usvg 侧同为类型化解析。本轮无授权、且会扩大变更面（mask/filter 行为一起动），未改；
   下一轮若动，请照本轮方式先查 usvg 语义再改，并补 fail-closed 证据。
