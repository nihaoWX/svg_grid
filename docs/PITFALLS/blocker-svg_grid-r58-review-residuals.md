# R59 — R58 评审残余：`feSpotLight` 长度、单轴双值、非法长度回归、obb `clipPath` 字号

**状态：已修（R59-F1..F4），两条口径/注释更正（F5/F6），残余只登记不修（见 §7）。**

本报告是本轮账本的来源文件（`pitfalls-manager record --from-json` 的 `reports`），
可跑证据在 `docs/PITFALLS/round-data/R59/{test.py,meta.toml}`。每条都先 d1render
实测再改码；表中"修前"用 `git archive cad0bef` 构建的 `/tmp/r59before`，"修后"是
本轮 `target/release/svg_grid`。

## 1. 现象（R58 评审的四条残余）

1. **E1（P1）`feSpotLight` 的放置量被当角度整块跳过。** 等向 2×：
   `x=20 y=20 z=10 pointsAtX=50 pointsAtY=40 pointsAtZ=0 limitingConeAngle=30`
   修前 **xor 18003 / maxdiff 205**（墨 31392，oracle 28950）；把六个位置量手改成
   `x=40 y=40 z=20 pointsAtX=100 pointsAtY=80 pointsAtZ=0` ⇒ **0/0**。各向异性
   （4×5）修前 **100170/242**。`feDistantLight` 的 `azimuth/elevation` 是角度，
   修前修后都 0/0，不应动。
2. **E2（P1）单轴策略下双值标量属性完全没改写。** `data-scale="x"` 时
   `stdDeviation="2"` 修前 **6024/81**，`y` 时 **9244/87**；双值源 `"3 4"` 在 x 下
   **9804/75**、y 下 **18856/80**；`radius` x **756/255**、y **1328/255**；
   `baseFrequency` x **31875/131**。手改成 `"8 2"` / `"2 10"` / `"12 4"` /
   `"3 20"` / `"0.0125 0.05"` 后全部 0/0。
3. **E3（P2 回归）非法长度让整次运行失败。** `stdDeviation="2px"`/`"abc"`、
   `feOffset dx="2px"`、`baseFrequency="abc"`、`radius="2px"` 在 cad0bef 上 **rc=2
   `invalid input: invalid numeric SVG attribute`**；R58 之前是 rc=0、属性原样保留。
4. **E4（P3）obb `clipPath` 体内 `font-size` 被缩。**
   `<clipPath clipPathUnits="objectBoundingBox">` 里 `font-size="0.2"` 修前写成
   `0.400`（mask/pattern 都已门控，只有 clipPath 漏了）。今天没有像素差：obb
   clipPath 的坐标缩放是 pre-existing 缺陷，会把 clip 清空（评审 fixture 两侧墨量
   都是 0），见 §7。

## 2. 复现

```bash
cd /home/nihao/bioagentforge/tools/svg_grid
# 修前二进制（不是 /tmp/r51head，也不是 /tmp/r50base）
rm -rf /tmp/r59before && mkdir -p /tmp/r59before
git archive cad0bef | tar -x -C /tmp/r59before
cp -r fontlib /tmp/r59before/
(cd /tmp/r59before && cargo build --release --offline)

# 全绿（96 checks）
python3 docs/PITFALLS/round-data/R59/test.py
# 有牙：被测指回修前 ⇒ 53 passed / 43 failed，rc=1
SVG_GRID_BIN=/tmp/r59before/target/release/svg_grid python3 docs/PITFALLS/round-data/R59/test.py
# 缺渲染器必须 rc=1（--allow-skip 才允许 skip）
SVG_GRID_RENDER_BIN=/nope python3 docs/PITFALLS/round-data/R59/test.py
```

单条手工版：`target/release/svg_grid plot-grid --input src.svg --output out.svg
--width 400 --ncol 1 --plot-margin 0`，`D1_FONT_DIR=$PWD/fontlib
/tmp/d1_eval/render/target/release/d1render out.svg got.png 1`（SCALE 是位置参数）。
oracle：等向 = 源图按 s 渲染；各向异性 = 源图内容包进 `<g transform="scale(sx sy)">`
在 400×400 画布上渲染。

## 3. 根因与修法（行号取自本轮修复 commit）

### E1 `feSpotLight` 是长度表，不是角度

`src/transform/filters.rs:179` 新增 `"feSpotLight"` arm：`x`→`LengthAxis::X`、
`y`→Y、`z`→Uniform、`pointsAtX`→X、`pointsAtY`→Y、`pointsAtZ`→Uniform。
只有 `specularExponent`/`limitingConeAngle` 无量纲，故意不列。Uniform 用几何平均
（`10 → 44.721` @4×5），与 `fePointLight z` 同口径。模块文档
`src/transform/filters.rs:25` 已把 spot 从"角度"改成"长度"。
`feDistantLight` 仍不动（`azimuth=45`、`elevation=20` 原样，xor 0/0）。

### E2 缺的因子按 1.0，标量必须写成双值

`src/transform/filters.rs:342-348`：分别取 X/Y 因子，只有**两者都缺**
（position/none）才 `return`；缺的一侧 `unwrap_or(1.0)`。之后
`src/transform/filters.rs:362` 的 `tokens.len() == 1 && fx == fy` 只在等向时保留
单值，否则写 `"x y"`。这和 `src/transform/lengths.rs:130-141` 的契约一致
（x 策略只缩 x 长度、y 保持），契约文字无需改。

### E3 解析失败是数据，不是结构错误

新增 `src/transform/filters.rs:287 parsed_or_kept()`：`parse_f32` 失败时
`passthroughs.record(index, element, attr, "unparsable number {raw:?}: {error}")`
并返回 `None`，调用方（`scale_axis` / `scale_pair`）保留原值；`?` 不再把错误上抛。
pair 只解析出第一个数时整条保留（`dx="2px"` 保留，`dy="5"` 仍缩成 `10`）。
模型是 `src/transform/transforms.rs` 的 `patternTransform` 路径。rc 仍为 0，
`--verbose` 打出 `1 attribute could not be rewritten, kept verbatim: ... "2px"`。

### E4 只门控 obb `clipPath` 的 body 字号

`src/transform/walk.rs:273-278`：`clipPathUnits == objectBoundingBox` 时
`child_body = false`。`child_body` 只喂 `text/tspan` 的 `font-size` 规则，坐标
`policy` 不变，因此没有碰 pre-existing 的 obb 坐标问题（§7）。

### E5 注释改成可复现说法

`tests/selftest/defs/mod.rs:189-193` 原注释称 "radius 2 => xor 840"；该 fixture 的
滤镜区域正好等于 rect，radius 2 与 4 实际都渲染 0/0（膨胀被裁掉），840 是 R58 A3
几何的数字。现注释写明：两者都是 0/0，断言针对属性（`radius="4"`）而不是像素，
旧期望把"图元不缩放"写死。

## 4. 四列证据（xor / maxdiff）

| 用例 | 修前 | 修后 | oracle 墨 | d1 |
|---|---|---|---|---|
| E1 spot 等向 2× | 18003 / 205 | **0 / 0** | 28950 | 28638 |
| E1 spot 各向异性 4×5 | 100170 / 242 | 37011 / 206（残余：法线） | 142217 | 141255 |
| E1 point 等向 | 0 / 0 | 0 / 0 | 18683 | 18471 |
| E1 distant（角度） | 0 / 0 | 0 / 0 | 558 | 522 |
| E2 x blur `2`→`8 2` | 6024 / 81 | 0 / 0 | — | — |
| E2 y blur `2`→`2 10` | 9244 / 87 | 0 / 0 | — | — |
| E2 x pair `3 4`→`12 4` | 9804 / 75 | 0 / 0 | — | — |
| E2 y pair `3 4`→`3 20` | 18856 / 80 | 0 / 0 | — | — |
| E2 x morph `2`→`8 2` | 756 / 255 | 0 / 0 | — | — |
| E2 y morph | 1328 / 255 | 0 / 0 | — | — |
| E2 x turb `0.05`→`0.0125 0.05` | 31875 / 131 | 0 / 0 | — | — |
| E3 五种非法长度 | rc=2 | rc=0 + 原样 + W7 | — | — |
| E4 obb clip font-size | `0.400` | `0.2`（像素两侧 0） | — | — |

E1 各向异性的 37011/206 是**法线随非等比缩放变化**，不是长度漏缩：把六个位置量
按 4×5/几何平均手改后仍是 37011/206（oracle 墨 142217），与 R58 N8 lighting 同族，
只登记不声称 0。full cairo 列在 test.py 里按能力输出（滤镜不作仲裁）。

## 5. 测试

新增 `src/transform/tests_r59.rs`（10 条 lib 测试）：spot 等向/各向异性、
distant 角度、x/y 单轴标量精确串、双值按轴、morphology+turbulence、三种非法长度
（rc=0+原样+W7）、obb clipPath 字号。`src/transform/tests_r58.rs` 里写错的
"spot 是角度"测试改写成正确语义（条数不变，旧断言把 bug 写死）。
总数 **643 = 351 + 59 + 233**（旧 633 = 341 + 59 + 233），已同步
`docs/PITFALLS/round-data/R19/test.py` 的 `BASELINE`/`COUNTS` 与
`docs/PITFALLS/round-data/R57/test.py` 写死的总数（"改快照就是签名"）。

## 6. 变异（4 条实跑，红名，还原后复绿 351）

| 变异 | 红掉的测试 | 结果 |
|---|---|---|
| E1 跳过 spot（arm 改名） | `tests_r58::spot_light_positions_scale_but_distant_light_angles_do_not`、`tests_r59::spot_light_positions_scale_isotropically`、`tests_r59::spot_light_positions_take_one_factor_per_axis` | 348/3 |
| E2 缺因子返 1 改回 `return` | `tests_r59::{x_policy_writes_a_pair_for_a_scalar_blur, y_policy_writes_a_pair_for_a_scalar_blur, one_axis_policy_scales_each_source_pair_on_its_own_axis, one_axis_policy_scales_morphology_and_turbulence_too}` | 347/4 |
| E3 解析失败重新 `?` 上抛 | `tests_r59::{malformed_blur_std_deviation_is_kept_verbatim, malformed_offset_radius_and_frequency_are_kept_verbatim}` | 349/2 |
| E4 去掉 clipPath 门控 | `tests_r59::object_bounding_box_clip_path_keeps_its_fractional_font_size` | 350/1 |

## 7. 登记（只登记、本轮不修）

- **obb `clipPath` 的坐标缩放是 pre-existing 缺陷**：`clipPathUnits="objectBoundingBox"`
  的 body 数（如 `x=0.05 y=0.7`）被当 user-space 数缩放（→ `0.1/1.4`），会把 clip
  清空；评审 fixture 因此两侧墨量 0、无像素 oracle。本轮只门控 font-size，坐标不动。
- **lighting 各向异性残余 37011/206**：非等比缩放下表面法线改变，无法用长度改写
  消掉；与 R58 N8 同族。
- `feImage` 各向异性（`preserveAspectRatio` 在变换前适配）与 `feDisplacementMap`
  噪声残余沿用 R58 登记。

## 8. R58 报告的三处口径更正（写进 R59 meta，不写回已删报告）

1. R58 不是"0 改动"：`git show c4c522f --numstat -- tests/` 是
   `5 2 tests/selftest/defs/mod.rs`。
2. 变异脚本不在 R58 `meta.toml`（那里没有 mutation 段），实际只在账本 R58-F7。
3. `feSpotLight` 的 `x/y/z/pointsAt*` 是长度，不是角度；只有
   `specularExponent`/`limitingConeAngle` 无量纲。

## 9. 回归与自述偏离

- `cargo test --offline` 643 全绿；`cargo fmt --all --check` rc=0；
  `cargo clippy --offline --all-targets -- -D warnings` rc=0。
- `./install.sh` 后 `target/release/svg_grid` 与 `~/.local/bin/svg_grid` 两侧
  sha256 同为 `3275c076…`；`svg_grid doctor` 57 pass / 0 fail / 1 skip。
- 214 个 `git ls-files 'tests/**/*.svg'` 的 `convert --report`、58 个 fixture 的
  `plot-grid --width 600 --ncol 1` 与 `--plan`，对 `/tmp/r50base`
  （@e0c6033，sha `75def0b8…`）逐字节 0 差异。
- R3..R58 旧证据顶层全绿，`R49/shim/ran.txt` 追加 `R59 OK`。
- 偏离：① E1 各向异性按硬性要求点名残余，不声称 0/0；② E4 无像素 oracle，理由
  是 §7 的 pre-existing 缺陷；③ 评审报告里 x 单轴 blur 记 4184，是本轮 fixture
  （显式 `filterUnits`、100×80→400×400）测得的 6024，同一机制不同几何；
  ④ `code_before=cad0bef` 按任务给定，工作树 HEAD 此后有一个只改
  PROGRESS/HANDOFF 的 HARNESS commit（7ee197d），代码未变。
