# pattern 的 href 语义与单元继承（R40 方向写反 + 顺带修 tile 继承）

- 轮次：R41（code_before = `715ce48`，即 R40 入账后的 HEAD）
- 触发：R40 复核 FAIL：2 个代码阻断（B-1 断链判 Verbatim、B-2 重复 id 首个胜）+ 1 个账本阻断（B-3 R40 行号膨胀）
- 评审证据：`/tmp/r40_eval/evidence/`（`probe_href_external.py`、`probe_dup_id.py`、`probe_units_inherit.py`、`check_ledger_lines.py`）
- 参考渲染器：`/tmp/d1_eval/render/target/release/d1render`（resvg/usvg 0.47）；对照基准 `75d59fb`、`96dfeab`、`715ce48` 的二进制

## 现象

### 现象 1（B-1，R40 引入）：不可解析的 `href` 被当成「内容原样」

`src/transform/patterns.rs` 的 `pattern_body` 在 `internal_target()` 为 None（外部 URL `other.svg#q`、非 `#id`、
空串、裸 `url(#nope)`）或 id 不存在时直接 `return PatternBody::Verbatim`，注释写「play safe and add no scaling」。

方向是反的。usvg `HrefIter`（`parser/svgtree/mod.rs:485-493`，`impl Iterator` 见 `:619-648`）在链断掉时**停止继承**，
`convert_units`（`parser/paint_server.rs:230-233`）随后落到 `node.attribute(name).unwrap_or(default)`，`patternContentUnits`
的默认值是 `UserSpaceOnUse` ⇒ body **必须**乘 (sx,sy)。静默原样 = 每像素 1/s 的误差。

复现（面板 100×80，`--plot-margin 24`，s=2；`~/.venv/bin/python /tmp/r40_eval/evidence/probe_href_external.py`）：

| href | 版本 | xor | maxdiff | ink_src→out |
| --- | --- | --- | --- | --- |
| `#missing` | R40 `715ce48` | 448 | 255 | 384→96 |
| `#missing` | 修复后 | 0 | 0 | 384→384 |
| `other.svg#q` | R40 `715ce48` | 448 | 255 | 384→96 |
| `other.svg#q` | 修复后 | 0 | 0 | 384→384 |

`other.svg#q` 是合法 SVG：convert rc=0、compose rc=0、无告警 ⇒ 静默错语义。

### 现象 2（B-2，R40 引入）：重复 id「首个胜」

`PatternIndex::collect` 注释「Duplicate ids keep the first occurrence, like the renderer's id map」是事实错误。usvg
`parser/svgtree/parse.rs:117-121` 按文档顺序 `links.insert(id, node.id)`，后者覆盖前者 ⇒ **最后**定义胜。

`probe_dup_id.py`（两个同名 `<pattern id="q">`，首个 `patternContentUnits="objectBoundingBox"`，次个 `userSpaceOnUse`，
引用方 `href="#q"`）：R40 `xor 448 / ink 96`；修复后 `xor 0 / md 0 / ink 384 = 384`（与 R39、与参考一致）。

### 现象 3：`patternUnits` 未沿 href 继承 + tile 缺失属性被写成 `0`

`patternUnits` 同样走 `resolve_pattern_attr`（`paint_server.rs:498-518`），默认 `objectBoundingBox`；此前只读元素自身属性。
另外 `rewrite_rect` 会把缺失的 `x/y/width/height` 当 `0` 写死，覆盖 href 继承来的 tile。

- 自身声明 tile、继承 units（`/tmp/u_tile.svg`：`base patternUnits="userSpaceOnUse"`，`p href="#base" x="12" y="14" width="16" height="18"`，body `2 3 6 4`）：
  R40 `xor 1992 / md 255 / ink 512→1800`；修复后 `xor 0 / md 0 / ink 512 = 512`。
- 自身声明部分 tile、继承其余（`/tmp/u_partial.svg`：`base x="2" y="3" width="20" height="18"`，`p href="#base" patternUnits="userSpaceOnUse" x="10"`）：
  R40 输出 `<pattern ... x="20" y="0" width="0" height="0"/>` ⇒ `xor 192 / ink_out 0`；修复后 `<pattern ... x="20"/>`（其余继承）
  ⇒ `xor 0 / md 0 / ink 192 = 192`。

### 关于评审给的 `units_inherit` xor 896

该数字**不能**作为「patternUnits 未继承」的判据：probe 用 `--width 288 --height 208 --plot-margin 24` 合成 100×80 面板，
即 sx=2.4、sy=2.0，而参考图按 scale 2.0 渲染（各向异性），新旧三版（new / r39 / r38）实测**都**是 896。
用等向合成（200×160，sx=sy=2）复核：tile 完全继承的用例（`p href="#base"`，p 不写 tile）三版都是 `xor 0 / ink 384 = 384`，
本来就没坏；真正坏的是「自身声明 tile + 继承 units」那一类（上表 1992→0）。缺陷是真实的，但评审那条复现的归属不对。

## 根因

| 语义 | usvg 0.47 行为 | R40 的做法 |
| --- | --- | --- |
| 链上无法解析的 href | `HrefIter` 停止继承 ⇒ 属性取默认值（内容单位 `userSpaceOnUse`） | 直接 Verbatim（不缩放） |
| 重复 id | `links.insert` 顺序覆盖 ⇒ 后者胜 | 保留首个 |
| `patternUnits` | 沿链继承，默认 `objectBoundingBox` | 只读自身属性 |
| tile 的 `x/y/width/height` | 逐个 `resolve_number` 沿链继承 | 缺失即写 `0` |

R40 的错法有共同点：对「不知道渲染器怎么处理」的分支拍脑袋选了「看起来安全」的一边，没有先读 usvg 源码 + 用 d1render 实证。
这是同一处 pattern 语义在 R38（退化用例）、R39（宽松判据）、R40（方向写反）之后的第三次翻车。

## 修法

- `src/transform/patterns.rs`（284 行，重写）：
  - `PatternIndex::collect`（`:65-69`）走 `walk`（`:71-90`），重复 id 用 `insert` 覆盖（后者胜）；`PatternSource { href, content_units, view_box, pattern_units }`。
  - `fn resolve`（`:119-150`）沿 `href`/`xlink:href` 链解析：`visited` 防环；`internal_target` 为 None（外部/裸 URL/空）或链指向非 `<pattern>`/悬空 id ⇒ **break**（不回退 Verbatim），
    让 `content_units`/`view_box`/`pattern_units` 留在默认语义 ⇒ user space。
  - `pattern_body_is_verbatim`（`:154-158`）= 严格 token `objectBoundingBox` 或可解析 `viewBox`；`pattern_units_is_user_space`（`:163-165`）用链解析结果。
  - `valid_view_box`（`:181-192`）、`next_number`（`:196-243`）保持 R40 的 svgtypes 语法（数字语法、w/h>0、尾部垃圾忽略）。
- `src/transform/primitives.rs`：新增 `rewrite_pattern_tile`（`:114-148`）——四个属性齐全才走 `rewrite_rect`，否则只重写元素自身声明的属性，
  缺失的留给 href 继承（usvg `resolve_number` 语义）。
- `src/transform/mod.rs`：`transform_element`（`:69-188`）用链解析结果分流 pattern children；`rewrite_primitive`（`:212-441`）的
  `pattern` 分支改用 `pattern_units_is_user_space` + `rewrite_pattern_tile`（`transform_svg` `:50-67` 构建 `PatternIndex`）。
- 测试：`src/transform/tests.rs` 新增/改写 `unresolvable_pattern_href_falls_back_to_user_space`（`:305-320`）、
  `duplicate_pattern_ids_resolve_to_the_last_definition`（`:325-336`）、`pattern_units_is_inherited_through_href`（`:341-370`）；
  `tests/selftest/transform/pattern.rs` 新增 `pattern_href_chain_follows_the_renderers_inheritance`（`:361-444`）。
- 证据脚本：新增 `docs/PITFALLS/round-data/R41/test.py`（388 行，族 A 四种断链 / 族 B 重复 id 双向 / 族 C patternUnits 继承 /
  族 D 严格 obb 与非法 contentUnits / 族 E fail-closed）；`docs/PITFALLS/round-data/R40/test.py`（421 行）的 C4/C5 从「只断言 markup」
  改成 resvg 渲染对照并断言缩放。

## 变异验证

| 变异 | 结果 |
| --- | --- |
| `pattern_body_is_verbatim` 改为 `content_units != Some("userSpaceOnUse")`（即「没写就 verbatim」，R40 的反向） | **7 条红**：`invalid_view_box_falls_back_to_user_space`、`invalid_pattern_content_units_scales_like_user_space`、`pattern_href_cycle_stops_instead_of_panicking`、`pattern_units_is_inherited_through_href`、`object_bounding_box_pattern_body_is_verbatim_under_scale`、`pattern_href_to_non_pattern_ends_the_chain`、`unresolvable_pattern_href_falls_back_to_user_space` |
| 重复 id 改回「首个胜」（`entry().or_insert_with`） | 1 条红：`duplicate_pattern_ids_resolve_to_the_last_definition` |
| `pattern_units_is_user_space` 只看元素自身属性 | 1 条红：`pattern_units_is_inherited_through_href` |

## 验证

- 单测：root `cargo test` = **104 lib + 59 main + 110 selftest = 273**（0 fail；R40 为 270）；convert = **107 lib + 14 CLI = 121**（0 fail）。
- `cargo fmt --all --check` rc=0；`cargo clippy --offline --all-targets -- -D warnings` 两个 crate 均 rc=0；`./install.sh` rc=0（fixtures 58、fonts 8、skill up to date）。
- `svg_grid doctor`：**57 pass / 0 fail / 1 skip**（matplotlib/mathtext：`<text>` 在旋转祖先下，转换器拒收，既有 skip）。
- 220 fixture（`tests/**/*.svg`）convert：对 `75d59fb` / `96dfeab` / 本轮逐字节，byte-changed **0**，失败集合三方同为 41，幂等 0，悬空引用 0。
- 58 组合 fixture（`--width 600`）：0 字节差；`mathtext.svg` 三方 rc=2；`bars_grouped.svg` md5 `717a14646cf2e7b1621723a0b39fdc09` 三方一致。
- resvg 对照（`c_claims39.py`，margin 24、width 288）：`bars_grouped` 新/基 `ink 15878 = 15878`，`xor 442 / md 7`；`S36_panelA_matrix` `ink 28223 = 28223`，`xor 357 / md 2` —— 442/357 量级未回退。
- 证据脚本：`docs/PITFALLS/round-data/R41/test.py` ⇒ `== 全部成立（另有 1 条 SKIP E1）==` rc=0；设 `SVG_GRID_OLD_BIN=/tmp/r41_old_target/debug/svg_grid`（715ce48）⇒ E1 PASS，`== 全部成立 ==`。
  `docs/PITFALLS/round-data/R40/test.py` ⇒ 绿；其 E1 在 715ce48 上红（href×2 + dup + units），在 96dfeab 上红（非法 units×5 + viewBox），两版都满足 fail-closed。
- 旧证据（顶层逐个跑，日志 `/tmp/r41_evidence/`）：R8、R9、R16、R25、R28、R31、R35、R36、R37、R38、R39、R40 全 rc=0。

## 遗留（本轮未修，确认未恶化）

- `maskContentUnits` / `patternUnits` / `maskUnits` / `filterUnits` 仍是 `trim()` 宽松比较（同类隐患）；mask 路径另有独立问题、基线也错，无授权不动。
- 嵌套 `<pattern>`（xor 11808）、`patternTransform` 长度量（xor 1561）、pattern 外 `clip-path`（xor 20736）、pattern 内 `<text>`（源图即 ink=0）、
  `patternUnits="objectBoundingBox"` 内容两渲染器语义分歧 —— 均与 R40 相同，未恶化。
