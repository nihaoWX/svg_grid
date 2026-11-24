# R62 阻塞点：定义的内容域（verbatim body）与 `<ellipse>` 从不缩放

状态：已修（A、B），口径更正（C），登记不改（D）。证据：`docs/PITFALLS/round-data/R62/`
（41 checks，fail-closed，有牙）。本轮 code_before `abc0d22`。

## 1 现象与复现

### A 内容域：verbatim body 把面板策略传给了内层定义

渲染器对“body 已被自己映射”的定义有一套自己的坐标空间：pattern 带 `viewBox`
或 `patternContentUnits="objectBoundingBox"`、marker 带 `viewBox` 时，tile/坐标/body
都由渲染器再映射一次。因此这些定义**内部**引用的 pattern / gradient / marker 应当
“完全不动”。修前它们却按引用者的面板 `data-scale` 被重写了。

复现（源 100×80，`plot-grid --width 200 --height 160 --ncol 1 --plot-margin 0`，即 s=2）：

```bash
# v1：viewBox pattern 的 body 里再嵌一个 userSpaceOnUse pattern
python3 docs/PITFALLS/round-data/R62/test.py     # A 组 8 个用例
```

修前产物里内层 pattern 的 tile 是 16/16、body rect 是 8/8（各被 ×2 一次），而正确值
是 8/8 与 4/4；v2/v3/m1/n3 的梯度 `x2` 同样被乘了一次面板因子。8 个用例的像素差
（xor 像素数 / 最大通道差）修前是 12160/255、32000/125、31200/51、800/125、1376/255、
16000/125、800/125、18720/125；两个对照（普通 userSpace 嵌套、全 userSpace）本来就是
0/0，修前修后产物逐字节相同。

### B `<ellipse>` 从未被重写

`src/transform/dispatch.rs` 只有 `circle` 分支，没有 `ellipse`；`convert --check`
也把椭圆当 compatible。复现（s=8，`plot-grid --width 800 --height 640 --ncol 1
--plot-margin 0`）：

```svg
<ellipse cx="20" cy="20" rx="10" ry="5" fill="red"/>
```

修前产物原样保留 `cx=20 cy=20 rx=10 ry=5`（应在 160/160/80/40），渲染与 oracle 差
10396 像素；只写 `rx` 时更糟（差 20656 像素，因为缺失的 `ry` 按 SVG 规则等于*旧* rx）。
`rx="abc"` 这种非数字几何修前 `plot-grid` rc=0 且写出产物、`convert --check` rc=0。

## 2 根因

1. R46 给定义分策略时只建了一根轴：“谁引用它、引用者是什么 `data-scale`”
   （`src/transform/defs.rs::referrers` 的 fixpoint：`refs[D] = direct[D] ∪ ⋃ refs[owner]`）。
   fixpoint 把 owner 的策略无差别传给 target，没有“owner 的 body 是 verbatim”这个
   第二根轴，于是 verbatim 域的出边继承了面板策略。
2. `rewrite_primitive` 的 match 里没有 `"ellipse"` 臂；`convert/scan.rs::non_numeric_geometry`
   也没有 ellipse 行，所以非数字几何也不会被拦。

## 3 修法

### A 内容域第二条轴 + 按域克隆

- 新文件 `src/transform/domains.rs`：`Slot::{UserSpace(ScalePolicy), Verbatim}`，
  `Slot::rank()`（Verbatim 排最后）、`Slot::tag()`（`verbatim`）、
  `body_is_verbatim(element, patterns)`（pattern 走 `pattern_body_is_verbatim`，
  marker 走 viewBox 判据）、`marker_body_is_verbatim`。
- `src/transform/defs.rs`：`DefsPlan` 从 `policies` 改成 `slots`；`referrers` 的 fixpoint
  在 owner 的 body 是 verbatim 时把传给 target 的 slot 换成 `Slot::Verbatim`；
  `split` 按 slot 排序（`rank`）后第一条保留原 id，其余 `id--svg-grid-<tag>`
  （沿用 R46 的 `--svg-grid-` 命名与去重号）；`split_children` 的 narrow 分支把
  新 slot 一并传给子定义（传递闭包）；单域永不克隆（`wanted` 只有一条时走旧路径）。
- `src/transform/walk.rs`：`slot_of` 为 `Verbatim` 时**直接返回**（该定义整棵子树不动）；
  为 `UserSpace(p)` 时用 p；没有 slot（未引用定义）时仍回退 `data-scale`/继承，
  保证旧产物的字节不变。
- `src/transform/hoist.rs` 跟随 `Slot`。
- 复审修复：第一版把**容器**的 slot 丢了（递归时把 `inherited` 而非元素自己的
  `reference_slot` 传给子节点），`<g data-scale="x">` 里的引用会指回 xy 的副本。
  R46 的 `A4_copy` 与 R49 的 `A1–A5` 旧证据当场变红抓到；修法是把元素自己的
  slot（verbatim body 时仍是 `Verbatim`）传给子节点，并加单测
  `a_group_data_scale_reaches_its_children` 钉死。修后 R46/R49 全绿。

### B ellipse

- `src/transform/primitives.rs::rewrite_ellipse`：`cx/cy` 坐标对；`rx` 走 x 轴、`ry` 走 y 轴；
  只写一个半径时，按 SVG 的“另一个与它相等”补写缺失的那个 —— 用**它自己的轴**的因子
  （`data-scale="x"` 时 `ry` 保持源值，正好等于“与旧 rx 相等”）。`position`/`none`
  下两半径都不动，未触碰的输入保持原字节。半径只写在 inline `style` 里时整对保留：
  d1render 对两种拼写都不画，写属性等于凭空造几何（0/0 变 20324），登记不改。
- `src/transform/dispatch.rs` 加 `"ellipse"` 臂。
- `src/convert/scan.rs::non_numeric_geometry` 加 `"ellipse" => &["cx","cy","rx","ry"]`，
  `convert --check` 与 `plot-grid` 现在同一句话、同一 rc 语义。
- `ARCHITECTURE/tag-protocol.md` 支持列表补 `<ellipse>` 与单半径规则。

## 4 证据（oracle = 源图按 s 用 d1render 直接渲染）

A 组（200×160，s=2；修前 = `abc0d22` 的二进制，修后 = 本轮）：

| 用例 | 修前 rc/产物 | 修后 rc/产物 | oracle | 修前 xor/md | 修后 xor/md |
|---|---|---|---|---|---|
| v1 内层 pattern（viewBox 父） | 0；tile 16/16、body 8/8 | 0；tile 8/8、body 4/4 | 源@2 | 12160/255 | 0/0 |
| v2 内层梯度（viewBox 父） | 0；x2=32 | 0；x2=16 | 源@2 | 32000/125 | 0/0 |
| v3 内层梯度（obb 父） | 0；x2=1 | 0；x2=0.5 | 源@2 | 31200/51 | 0/0 |
| m1 marker body 里的梯度 | 0；x2=20 | 0；x2=10（markerWidth 仍 20） | 源@2 | 800/125 | 0/0 |
| n3 marker body 里的 pattern | 0；tile 8/8、body 4/4 | 0；tile 4/4、body 2/2 | 源@2 | 1376/255 | 0/0 |
| v4 梯度被面板与 viewBox body 共用 | 0；单个 x2=20 | 0；`g` x2=20 + `g--svg-grid-verbatim` x2=10 | 源@2 | 16000/125 | 0/0 |
| n4 同上、marker 版 | 0；单个 x2=20 | 0；面板 `g` 20 + body 克隆 10 | 源@2 | 800/125 | 0/0 |
| m3 三域共用梯度 | 0；单个 x2=20 | 0；`g` 20 + 一个 verbatim 克隆 10（两个 verbatim 域共用） | 源@2 | 18720/125 | 0/0 |
| c1 普通嵌套（对照） | 0；产物字节不变 | 0；与修前逐字节相同 | 源@2 | 0/0 | 0/0 |
| c2 全 userSpace（对照） | 0；产物字节不变 | 0；与修前逐字节相同 | 源@2 | 0/0 | 0/0 |

B 组（800×640，s=8；b4 的 oracle 是手写的 800×640 椭圆文档，因为单轴策略没有“源图按 s”可对）：

| 用例 | 修前 rc/产物 | 修后 rc/产物 | oracle | 修前 xor/md | 修后 xor/md |
|---|---|---|---|---|---|
| b1 双半径 | 0；cx/cy/rx/ry=20/20/10/5 | 0；160/160/80/40 | 源@8 | 10396/255 | 0/0 |
| b2 只写 rx | 0；rx=10（无 ry） | 0；rx=80、ry=80 | 源@8 | 20656/255 | 0/0 |
| b3 只写 ry | 0；ry=10（无 rx） | 0；rx=80、ry=80 | 源@8 | 20656/255 | 0/0 |
| b4 `data-scale="x"` | 0；20/20/10 | 0；160/20/80/10 | 手写文档 | 2959/255 | 0/0 |
| b5 `rx="abc"` | 0；写出产物、ink 0；`convert --check` rc=0 “compatible” | 2；不写产物；`convert --check` rc=1 “geometry attribute rx is not a plain number” | 失败即产物 | — | — |

D 组（只登记、不改；修前修后同一数字，两侧二进制一致）：obb clipPath body 引用 pattern
仍 xor 16000/255（把 tile 缩到 8 也仍是 16000/255，没有正确写法）；obb mask 引用 pattern
的两种拼写都 0/0（无判别力）；单坐标梯度、tile 的 x/y 与 width/height、filter/mask
区域各自只重写写出来的属性（单测钉住，且不允许补对）。

## 5 变异实跑（红名）

| 变异 | 单元红名 | 证据红名 |
|---|---|---|
| M1 fixpoint 不再把 verbatim 传给 target | `a_pattern_inside_a_view_box_body_stays_verbatim`、`a_gradient_inside_an_object_bounding_box_body_stays_verbatim`、`a_marker_view_box_body_keeps_its_gradient`、`a_pattern_inside_a_marker_view_box_stays_verbatim`、`a_shared_gradient_is_copied_per_domain`、`three_domains_are_two_copies_because_both_verbatim_slots_agree` | A 组 8 例 × 2 条断言（rc=1） |
| M2 walk 不再跳过 verbatim 定义 | `a_gradient_inside_an_object_bounding_box_body_stays_verbatim` | A 组 8 例 |
| M3 共用定义只留一个 slot 版本（不克隆） | `a_shared_gradient_is_copied_per_domain`、`copies_follow_the_reference_graph_and_get_unique_ids`、`three_domains_are_two_copies…` + hoist 3 条 | A/v4、A/n4、A/m3 |
| M4 不补写缺失半径 | `an_ellipse_with_one_radius_materialises_the_equal_one`、`an_ellipse_with_one_radius_under_a_single_axis_policy_keeps_the_other` | B/b2、B/b3、B/b4 |
| M5 scan 不检查 ellipse | `a_non_numeric_ellipse_radius_is_rejected` | B/b5 |
| M6 dispatch 去掉 ellipse 臂 | ellipse 5 条单测（b1/b2/b3/b4/样式半径） | B/b1…b4 |
| M7 子节点不继承容器 data-scale（复审修复前的形态） | `a_group_data_scale_reaches_its_children` | R46 `A4_copy`、R49 `A1–A5` |

## 6 未做清单（有理由）

- 非有限 points 的双 W7（`note_non_finite` + `note_overflow`，第二条被错误归到 overflow）：
  要修得把“整组保留”拆成逐属性 offender，W7 文案与大量证据期望会一起变；诊断措辞收益。
- A3 的“1 attribute / 保留 2 个”：同一个 atom-group 只记一个 offender，同上。
- R61 `test.py:258-259` 失败只打印 `err[-160:]`：失败打印便利，不影响结论。
- ellipse 半径只写在 inline `style`：d1render 两种拼写都不画，写属性会造出源图没有的几何。

## 7 验收

- `cargo test --offline`：696 = 404 lib + 59 main + 233 selftest（基线 674；R19/R57 同轮更新）。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` 后两侧 sha256 相同：`2495107bcf1e5e0cc580746bcd747abd8b2d414429fe77305ed31d5ac1ffcfb3`。
- `svg_grid doctor`：57 pass / 0 fail / 1 skip。
- 语料 214 个 `tests/**/*.svg` 的 `convert` 产物与 `/tmp/r50base/target/release/svg_grid`
  （@e0c6033）逐字节 0 差异；58 个 fixture 的 `plot-grid` 0 差异。
- 旧证据 R3–R61 逐个脚本全绿（R18/R19 按设计各跑约 20/25 分钟）。
