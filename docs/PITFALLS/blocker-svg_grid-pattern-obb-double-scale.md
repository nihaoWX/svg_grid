# blocker: `patternContentUnits="objectBoundingBox"` 的 body 被二次缩放（R38 代码回归）；账本三处记录与事实不符

- 轮次：R39（`code_before = 75d59fb`；回归引入于 R38 `95698ca`）
- 影响 1（P0，静默）：任何「面板被缩放 + `<pattern patternContentUnits="objectBoundingBox">`」的图，
  motif 被再乘一次 `(sx, sy)`：缩小侧整块变淡/消失，放大侧 motif 缩小到 tile 左上角；**rc=0、stderr 0 字节**。
- 影响 2（P1，证据）：R38 的 A2/A6A2 用例 body `x="0.1"`（0.1×40=4 恰好落在 tile 右缘外）⇒ resvg 下
  源图本身 ink=0，三张图全空 ⇒ `diff 0` 是空洞通过；cairosvg 侧又被 `skip("obb 内容语义两渲染器不一致")` 掩盖。
- 影响 3（P1，账本）：R19 的 retire ③、R20 的 modify 文本、R38-F7 的 symptom 与仓库事实相反；
  R38 报告里的转换器计数（104 lib + 14 = 118）也过期（实测 106 + 14 = 120）。

## 现象 1 — obb body 双重缩放（P0，静默）

`patternContentUnits="objectBoundingBox"` 时，body 的分数字面量是**引用元素 bbox 的分数**；引用元素已经
随面板缩放，bbox 也随之缩放。R38 把 pattern 的**所有** children 一律送进 `ScalePolicy::Scale`（再乘
`(sx, sy)`），于是同一个 bbox 被乘了两次。

### 复现（`/tmp/r38_eval/evidence/repro_obb.sh`）

```bash
cd /tmp && mkdir r39rep && cd r39rep
cat > src.svg <<'EOF'
<svg width="120" height="80" viewBox="0 0 120 80" xmlns="http://www.w3.org/2000/svg">
<rect data-panel-box="main" x="0" y="0" width="120" height="80" fill="none" stroke="none"/>
<g data-scale="xy"><defs>
<pattern id="p" patternUnits="userSpaceOnUse" x="10" y="10" width="20" height="20"
         patternContentUnits="objectBoundingBox">
<rect x="0" y="0" width="0.1" height="0.1" fill="#000"/></pattern></defs>
<rect x="10" y="10" width="40" height="30" fill="url(#p)"/></g></svg>
EOF
SVG_GRID=/home/nihao/bioagentforge/tools/svg_grid/target/debug/svg_grid
$SVG_GRID plot-grid --input src.svg --output out.svg --width 60 --height 40 --ncol 1 --plot-margin 0
# 放大侧：--width 240 --height 160
```

实测（resvg `d1render`；源图按 composer 施加的 s 渲染后与产物逐像素比）：

| 场景 | 源图 ink | 修复前（R38/`95698ca`） | 修复后（R39） | base `756dfb2` |
|---|---|---|---|---|
| s=0.5（`--width 60`） | 16 | ink 4，xor 16，maxdiff 255 | **ink 16，xor 0，maxdiff 0** | ink 16，xor 0 |
| s=2（`--width 240`） | 192 | ink 768，xor 576，maxdiff 255 | **ink 192，xor 0，maxdiff 0** | ink 192，xor 0 |

修复前 body 字面量被乘上 s：s=0.5 写 `width="0.050"`、s=2 写 `width="0.200"`（均应为 `0.1`；
实测对应 `95698ca` 的构建）；修复后两种尺度都逐字保留 `width="0.1"`。另一个证据脚本 `/tmp/r38_eval/evidence/a_obb2.py`：

- `both_obb`（tile 与 content 都是 obb，s=2）：修复前 body `x="0.100" width="0.200"`、ink 6912、xor 8448；
  修复后 body `x="0.05" width="0.1"`、ink 3072、**xor 0**，与 base 一致。
- `units_obb_content_user`（tile 是 obb、content 默认 userSpaceOnUse）：修复后 ink 4624、**xor 0**；
  base 仍把 body 平移到 `x="24"` ⇒ ink 1536、xor 6160（**R38 的修法不能回退**）。

## 现象 2 — R38 的 A2/A6A2 是非退化失败（空洞通过）

R38 的 A2 用 tile `4×6`、body `x="0.1" y="0.2" width="2" height="3"`，引用矩形 `40×30`；`0.1×40=4` 恰好
等于 tile 右缘（tile 宽 4 从 x=0 起）⇒ 源图在 resvg 下 ink=0，三张图全空，`xor 0` 只证明两块白布相同。
cairosvg 分支则直接 `skip("obb 内容语义两渲染器不一致")`。

修法：A2 改为 tile 与引用矩形同尺寸（`40×30`，tile 在 `(10,10)`），body `0.1/0.2/0.5/0.25` 完整落在 tile 内；
同一文档组合两次（100×80 与 200×160），断言 s=1 与 s=2 都逐字保留 body；A6 判据追加
`ink(src) > 0 and ink(out) > 0`；cairosvg 分支去掉 SKIP，改为「源图 1:1 渲染后取 ink，与产物同量级
（4× 以内）」，逐像素判据以 resvg 为准（实测 cairosvg 不实现 obb contentUnits：分数被当 user unit，
body 只有亚像素，源图 1:1 ink=1）。修完 `docs/PITFALLS/round-data/R38/test.py` **rc=0、无 SKIP**。

## 现象 3 — 账本记录与事实不符

1. **R19 retire 理由 ③ 写「独立 `svg_grid_convert` 已合并/已不存在」——不成立。**
   `convert/Cargo.toml:12` 仍声明 `[[bin]] name = "svg_grid_convert"`，base 上可构建出 1,093,176 B 的独立
   二进制，`--help` 仍自称 `svg_grid_convert`；只是 `install.sh` 自 R19 起不再安装它（`stale_convert`）。
   真实失效依据是：① 顶层测试目录清单（缺 `user_tests`、多 `poppler_dialect`/`shape_paint`）
   ② 写死 `BASELINE=190`（60/57/73；base 实测 89/59/104）③ 告警前缀与 `<style>` V1 告警（D6 起默认静音）。
   base `756dfb2` 上该证据实测 **rc=1（12 FAIL）**。
2. **R20 的 modify 文本重复了同一句「已合并的独立 `svg_grid_convert`」** ⇒ 追加更正。
3. **R38-F7 的 symptom 写「旧断言反而编码了错误行为（body 随 tile 平移）」与 git 内容不符**：base 的
   期望值本来就是 `<rect x="0" y="0" width="8" height="6">` 与 `M 0 0 L 8 6`（正确语义），只是
   `margin=0` 让平移不可分辨 ⇒ 旧断言不是错，只是测不出错。R38 报告已删，只能在账本追加更正。
4. **R38 报告正文的转换器计数过期**：写成 `104 lib + 14 CLI = 118`；本轮实测 **106 lib + 14 = 120**
   （`cd convert && cargo test`）。报告已删无法改正文，在 `round-data/R39/meta.toml` 与账本记录里点明。

## 根因

1. **代码**：R38 为修 `userSpaceOnUse` 的相位，把 `ScalePolicy::Scale` **无差别**套给 pattern 的所有
   children，没有检查 `patternContentUnits`。obb 内容的参照系是「引用元素 bbox（已含 s）」，与
   userSpaceOnUse 的「tile 用户空间」只差一次 `(sx, sy)`；不区分单位模式就必有一侧错。
2. **测试**：A2 的数值恰好退化（`0.1×40 == tile 宽 - 4`），且判据只看 `diff == 0`、不看 ink，
   于是「全空」也通过；cairosvg 侧用 SKIP 掩盖。这是**判据的洞**，不是数值的洞。
3. **账本**：R19 retire 的③是没有核对 `convert/Cargo.toml` 与 `install.sh` 的差异就写的「已合并」；
   R20 的 modify 沿抄了同一句；R38-F7 的 symptom 是事后追写、没有对 base 内容取数。

## 修法（位置见 `meta.toml` / 账本 R39-F1..F6）

- `src/transform/mod.rs`
  - 新增 `pattern_content_is_object_bounding_box(element) -> bool`：`patternContentUnits` 显式存在且
    非 `userSpaceOnUse`（未写 = 默认 userSpaceOnUse）。
  - `transform_element` 的 `pattern` 分支：先判 obb ⇒ 直接 `Ok(())`（整棵 body 不递归、不改数值）；
    否则维持 R38 的 `ScalePolicy::Scale` 递归。
  - `rewrite_primitive`：在末尾 `rewrite_scaled_length("stroke-width", …)` 之前对 obb pattern 早返回，
    避免 `<pattern>` 上继承的 `stroke-width` 被二次缩放（body 的字面量本身已由递归跳过保证）。
- `src/transform/tests.rs`（新，3 单测）：s=1 逐字保留、s=2 逐字保留且 tile 仍随 s 变、继承
  `stroke-width="0.1"` 在 s=2 仍为 `0.1`。
- `tests/selftest/transform/pattern.rs`：`object_bounding_box_pattern_body_is_verbatim_at_every_scale`
  跑 identity/double 两尺度，断言 body 逐字、tile `10,10,4,6 → 20,20,8,12`。
- `convert/src/translate/mod.rs`：`object_bounding_box_pattern_body_keeps_its_fractions`——
  转换器只改语法，obb 数字必须原样（含 `points` 里的分数）。
- `docs/PITFALLS/round-data/R38/test.py`：A2/A6A2 非退化重建 + cairosvg 判据改为 resvg 仲裁 +
  ink 量级（见现象 2）。
- 账本（append-only）：`retire R19` 追记正确理由、`modify R20`、`modify R38`（symptom 更正）、
  `modify R38`（证据脚本修改）。

## 验证

- 根 `cargo test`：**92 lib + 59 main + 108 selftest = 259，0 fail**（R38 时为 89/59/108=256）。
- `cd convert && cargo test`：**107 lib + 14 CLI = 121，0 fail**（R38 报告写的 118 已过期）。
- `cargo fmt --all --check` rc=0；`cargo clippy --offline --all-targets -- -D warnings` rc=0（两个 crate）。
- `./install.sh` rc=0；`svg_grid doctor` ⇒ **57 pass / 0 fail / 1 skip**（matplotlib/mathtext：`<text>`
  在旋转祖先下，转换器按设计拒绝）。
- 220 fixture（`tests/**/*.svg`）转换：new-only 失败 **0**、base-only **0**、字节变化 **0**、
  幂等问题 **0**、悬空引用 **0**。
- 58 fixture 组合（`--width 600`）：唯一 rc=2 是 `mathtext.svg`（base 同样）；唯一字节变化是
  `bars_grouped.svg`（R38 既有，允许）；无 `data-scale="scale"` 泄漏。
- 旧证据顶层逐个复跑：R8、R9、R16、R25、R28、R31、R35、R36、R37、R38 全部 rc=0
  （日志 `/tmp/r39_evidence/<R>.log`）；R38 的 A2/A6A2 修复后仍全绿、无 SKIP。

## 遗留（既有缺陷，本轮不修，已确认未恶化）

- **P1 嵌套 pattern**（pattern body 里再引用 pattern）：仍按 tile 局部处理，累计相位问题未动。
- **P1 `patternTransform` 长度量**：原样保留（语料 0/220 使用）；要写必须**累积**而不是覆盖。
- **P2 pattern 外的 clip-path**：不变。
- **P3 pattern 内 text**：不变（cairosvg/resvg 对 obb 下 text 的分数语义仍不一致，以 resvg 为准）。
- cairosvg 不实现 `patternContentUnits="objectBoundingBox"`：这是渲染器差异，不作 composer 缺陷处理。
