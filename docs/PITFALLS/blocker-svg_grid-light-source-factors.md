# R60 — 光源长度语义（z 用 RMS、位置平移、obb 不下门控）与四条产物污染

**状态：P1-1..P1-4、P2-1..P2-4、P2-7 已修；P2-5 已补账本事件；P2-6 已改本轮写法。
残余只登记不修（见 §8）。**

本报告是本轮账本的来源文件（`pitfalls-manager record --from-json` 的 `reports`），
可跑证据在 `docs/PITFALLS/round-data/R60/{test.py,meta.toml}`。每条都先 d1render
实测再改码；"修前"是 R59 修后的二进制 `/tmp/r59_eval/pin/target/release/svg_grid`
（评审用的同一份，= `3f1de81` 的构建），"修后"是本轮 `target/release/svg_grid`。

## 1. 现象（R59 对抗评审）

1. **P1-1 光源的 z/pointsAtZ 用错因子。** 4×5 各向异性下 `feSpotLight` diffuse
   修前 **xor 37011 / maxdiff 206**；把 `z` 手改成 **45.2769** ⇒ **2/2**；sweep
   44.721→37011/206、45→18450/205、45.2769→(2,2)、46→46688/207。`fePointLight`
   同一处（R58 就继承下来的）：修前 **95008/2**，z=45.2769 ⇒ **(2,1)**。
2. **P1-2 光源的 x/y 只乘因子、不平移。** 两个 100×80 面板并排时第二个 cell 的
   光源仍是 `x=20`；cell 1 对 oracle **5277/238**（spot）、**7563/248**（point）；
   手改成 `x=120 pointsAtX=150` ⇒ **0/0**。
3. **P1-3 obb 下整块 lighting 被跳过。** `primitiveUnits="objectBoundingBox"`
   修前属性不动、**100170/242**；改成 `x=80 y=100 z=45.2769 pointsAtX/Y=200` ⇒ **2/2**。
4. **P1-4 口径被证伪。** `round-data/R59/meta.toml` 与模块文档把 37011/206 写成
   "内在残余（法线随非等比缩放变化）"，实测是 RMS 因子没实现。
5. **P2-1 `feDisplacementMap scale` 的 doc 与代码不符。** doc 说"退化成几何平均"，
   `data-scale="x"` 下代码写 `sx*s`（10→40）；实测渲染器是**平方**关系。
6. **P2-2 图元子区域 x/y/width/height 不动。** `feGaussianBlur stdDeviation=2
   x=10 y=10 width=80 height=60` ⇒ stdDeviation 变 4 但盒子不动 ⇒ **16984/255**。
7. **P2-3 R59 证据冻住了错的语义**（`z == "44.721"` + `maxdiff ≤ 210`）：正确的
   RMS 修复反而会让它红（"红正确实现"）。
8. **P2-4 非有限值被写成 `inf`/`NaN`。** `stdDeviation="1e999"`（R58 起）、
   `stroke-width="1e999"`（R50 起）都被改写成 `"inf"` —— 非法 SVG 数值，且 W7
   文案说 "kept verbatim" 而字节变了。`""` / `" "` / `"2 3 4"` 保原文但静默无 W7。
9. **P2-5** 账本缺 R19/R57 计数从 633→643 的两条 `modify`。
10. **P2-7** `src/transform/tests_r58.rs` 已 19990 B，不能再加测试。

## 2. 复现

```bash
cd /home/nihao/bioagentforge/tools/svg_grid && cargo build --release --offline

# 修前 = R59 修后，评审已构建：
/tmp/r59_eval/pin/target/release/svg_grid
# 或自建：git archive 3f1de81 | tar -x -C /tmp/r60before && cp -r fontlib /tmp/r60before/
#         (cd /tmp/r60before && cargo build --release --offline)

# 全绿（75 checks）
python3 docs/PITFALLS/round-data/R60/test.py
# 有牙：被测指回 R59 ⇒ 52 passed / 23 failed，rc=1
SVG_GRID_BIN=/tmp/r59_eval/pin/target/release/svg_grid python3 docs/PITFALLS/round-data/R60/test.py
# 缺二进制/渲染器 ⇒ rc=1（--allow-skip 才 skip）
SVG_GRID_BIN=/nope python3 docs/PITFALLS/round-data/R60/test.py
```

单条手工版：`target/release/svg_grid plot-grid --input src.svg --output out.svg
--width 400 --height 400 --ncol 1 --plot-margin 0`，`D1_FONT_DIR=$PWD/fontlib
/tmp/d1_eval/render/target/release/d1render out.svg got.png 1`（SCALE 是位置参数）。
oracle：等向 = 源图按 s 渲染；各向异性 = 源图内容包进 `<g transform="scale(sx sy)">`
在 400×400 画布上渲染。

## 3. 四列表（修前 / 修后 / oracle / d1 ±cairo）

| case | 修前 xor/md | 修后 xor/md | oracle | d1 ±cairo |
|---|---|---|---|---|
| F1-spot-rms（P1-1） | 37011/206 | **0/0** | ink 142217 | d1 ok / cairo ok |
| F1-point-rms（P1-1） | 95008/2 | **0/0** | ink 86809 | d1 ok / cairo ok |
| F2-spot-paz（P1-1） | 35728/193 | **0/0** | ink 144302 | d1 ok / cairo ok |
| F3-two-cell（P1-2） | 5277/238 | **0/0** | ink 14784 | d1 ok / cairo ok |
| F4-obb（P1-3） | 100170/242 | **0/0** | ink 142217 | d1 ok / cairo ok |
| F5-x-bbox（P2-1） | left edge **48** | left edge **42** | left edge 42 | d1 ok |
| F6-subregion（P2-2） | 16984/255 | **0/0** | ink 19132 | d1 ok / cairo ok |
| F7-blur / F7-stroke（P2-4） | `inf` | 逐字节保原文 + W7 | attribute | n/a |
| F8 `""` / `" "` / `"2 3 4"`（P2-4） | silent | 保原文 + W7 | attribute | n/a |

属性字符串（修后，`git show <fix-commit>:src/transform/filters.rs` 生成）：

```xml
<feSpotLight x="80" y="100" z="45.276928" pointsAtX="200" pointsAtY="200" pointsAtZ="22.638464" limitingConeAngle="30"/>
<fePointLight x="80" y="100" z="45.276928"/>
<!-- 第二个 cell -->
<feSpotLight x="120" y="20" z="10" pointsAtX="150" pointsAtY="40" pointsAtZ="0" limitingConeAngle="30"/>
<!-- obb 的 filter -->
<feSpotLight x="80" y="100" z="45.276928" pointsAtX="200" pointsAtY="200"/>  <feGaussianBlur stdDeviation="2"/>
<!-- 位移（x 策略、sx=4、原 scale="1"） -->
<feDisplacementMap ... scale="2"/>
<!-- 子区域 -->
<feGaussianBlur stdDeviation="4" x="20" y="20" width="160" height="120"/>
```

## 4. 根因与修法（行号取自本轮修复 commit）

### P1-1 z/pointsAtZ 是 RMS，不是几何平均

`resvg-0.47.0/src/filter/mod.rs::transform_light_source` 对两种光源都算
`let sz = (ts.sx*ts.sx + ts.sy*ts.sy).sqrt() / SQRT_2;` 并 `z *= sz`、`points_at_z *= sz`。
`src/transform/lengths.rs:186` 新增 `fn rms(sx, sy)`，**逐字照抄该式**（`45.277` 会翻 11
像素，因为渲染器直接消费 f32）。`LengthAxis` 增 `LightZ`（`lengths.rs:21`）：
`Xy/Scale → rms(sx, sy)`、`X/ScaleX → rms(sx, 1)`、`Y/ScaleY → rms(1, sy)`、
`Position/None → None`（分派在 `lengths.rs:126-176`，LightZ 的行是 130/136/143/166/172）。
两种光源都走 `lights.rs:45` 的 `rewrite_light`（z 与 spot 的 pointsAtZ 都取 LightZ，
位置另由 `rewrite_positions` 处理），写回走 `filters.rs:280` 的 `scale_axis_precise`
（最短往返，不用 3 位小数的 `format_number`）。

为什么等向看不出来：`rms(s,s) = s`，几何平均也是 `s`；只有各向异性（4×5）才分叉。

### P1-2 光源位置要平移

`resvg` 对光源的 x/y 做 `ts.map_point(&mut point)` 再减 `region`，即**先缩放再平移**。
`lights.rs:92` 的 `rewrite_positions`：present 的属性交给 `rewrite_axes`
（内部 `map_point`，与 `feImage` 的 x/y 同一机制）；**缺省的属性**用 `map_point(0,0)`
得到纯平移量，非 0 才补写——否则 `x` 省缺的光源会留在原 cell。z/pointsAtZ 只缩放
（不进 `map_point`），因为它们没有水平位置。

### P1-3 obb 不再挡住光源

usvg 的 `convert_light_source` 把六个位置读成**普通 f32**，与 `primitiveUnits`
无关。`walk.rs:250` 现在对每个 `<filter>` 都调 `rewrite_primitives`；门控下沉到
`rewrite_one` 开头（`filters.rs:135` 起）：`!user_space && !is_light_source && !is_lighting`
才整体跳过。所以 obb 下光源仍被缩放/平移，而真正的 obb 分数属性（`stdDeviation`、
`radius`、`baseFrequency`）照旧不动（F4 里 `stdDeviation="2"` 保持 `"2"`）。

### P2-1 单值 scale 被渲染器平方 ⇒ 取轴因子的平方根

`resvg` 的 `apply_displacement_map` 用 `scale_coordinates(fe.scale(), fe.scale(), ts)`，
而 `displacement_map.rs` 里 offset = `(c-0.5) * sx * fe.scale()` ⇒ **`scale` 被乘两次**。
常量 map（R=255,G=128）+ 黑块 bbox 实测：`data-scale="x"`、sx=4、原 `scale="2"` 时
x 位移 4（=0.5·2²·2），故**精确值**是 `scale·sqrt(sx)`：原 `1` ⇒ `2`（黑块左边缘 40→42）。
旧的线性值 `sx*s` 会过位移 4×（左边缘 48，xor 5472/255）。两条轴同时各向异性时单值
无解（x 要 sqrt(8)、y 要 sqrt(10)），回退为两个精确值的几何平均 `(4·5)^(1/4)=2.115`
（旧 Uniform 写 4.472，过 4.5×）。`LengthAxis::Displacement`（`lengths.rs:131,137,147,158,167,173`），
模块 doc 第 46-67 行同步写清"平方"与这个契约。

**oracle 的坑（写进 meta，供下一轮）**：位移**不能**用 `<g transform>` 包源图做 oracle
——resvg 在局部分辨率（100 宽）栅格化滤镜再放大，0.5px 的位移被舍掉，参照图不动
（假残余 1352/255）。F5 改用几何已知的黑块左边缘（10·4=40）作 oracle。

### P2-2 图元子区域是 primitiveUnits 长度

usvg `resolve_primitive_region` 的 userSpace 分支：
`NonZeroRect::from_xywh(x.unwrap_or(region.x()), ..., width.unwrap_or(region.width()), ...)`
——所有非光源图元（含 `feFlood`/`feImage`）的 x/y/w/h 都是 user-space 长度，obb 下则是
bbox 分数（不能动）。`filters.rs:231` 在 match 之后对 `user_space && !light` 的图元补
`x/y`（`rewrite_axes`，平移+缩放）与 `width/height`（按轴缩放）；`feImage` 的专用 arm
因此删掉（它这四个数就是同一个子区域）。

### P2-4 非有限值一律保原文；畸形值必须报 W7

- `filters.rs:320` `parsed_or_kept`：`Ok(value) if value.is_finite()` 才返回 `Some`；
  非有限走 `note_non_finite` 并返回 `None`（保原文）。W7 文案里的 `kept verbatim` 与字节一致。
- `lengths.rs:77` `rewrite_scaled_length`：同样只在有限时写回（`stroke-width="1e999"` 自
  R50 起被写成 `inf` 的那条）。
- `primitives.rs:35,74,112,247,309`：`rewrite_pair`/`rewrite_axes`/`rewrite_rect`/
  `rewrite_points`/`rewrite_rotate_transform` 全部"任一坐标非有限就不写这个属性"。
- `style.rs:127,134`：`scale_token` 非有限（或乘完非有限）返回 `None`，token 保原文。
- `filters.rs:353` `scale_pair`：`tokens.is_empty() || len > 2` 不再静默 return，
  记 `malformed <number-optional-number> "<raw>": expected one or two numbers`。
  W7 单行 200 字符上限（`MAX_BODY=181`）：带原因时 `"2 3 4"` 是 172 字，所以文案不重复
  "3 numbers"（会被截掉）。
- 口径依据（usvg `parser/filter.rs::convert_std_dev_attr`）：空值 / 三个数 / 非有限都被
  当成**整个属性忽略**（模糊为 0），不是回默认值——所以保原文是对的，但必须让用户看见。

### P2-3 / P1-4 R59 的证据与口径改成 RMS

`round-data/R59/test.py` 的 E1 断言 `z == "45.276928"`、`maxdiff ≤ 5`，注释改为
"RMS，不是几何平均 44.721"；R59 的 meta 与 `src/transform/filters.rs` 模块 doc 里
"lighting 残余是内在的（法线变化）/所有候选 ≈1.5e5"整段删掉，改成"GM 写 44.721 ⇒
37011/206；RMS 写 45.276928 ⇒ 0/0"。

### 结构：光源代码搬进 `lights.rs`

修完 P1 后 `src/transform/filters.rs` 涨到 22065 B，撞破"单文件 ≤ 20000 B"（R8 的
"所有源文件 ≤ 20000 B"检查会红，R7/R9/R13..R19 随之级联）。于是把光源整块搬进新的
`src/transform/lights.rs`（4502 B：`is_light_source`、`is_lighting`、`rewrite_light`、
`rewrite_positions`），`filters.rs` 只留
`"fePointLight" | "feSpotLight" => super::lights::rewrite_light(...)`；`filters.rs` 现 18883 B，
`scale_axis_precise` 改 `pub(super)` 供复用。拆分后 658 测试与 R60 证据 75 条全绿，五条变异重跑结果不变。

### 旧快照 R6 的期望值一并改正

`round-data/R6/test.py:70-72` 冻结的是旧的污染行为（`cx="nan"` 被写成 `cx="NaN"`，而 W7 文案
说 kept verbatim）。P2-4 之后正确行为是字节原样，故该断言改成 `cx="nan"` + 文本中无 `NaN`，
rc=0 与 W7 不变——这是"证据随之改口径"的 `modify` 事件，与 R59 的 E1 同类。

## 5. 变异（每条实跑，红后还原复绿；脚本 `/tmp/r60_mutate.sh`）

| 变异 | 红名 | 还原 |
|---|---|---|
| M1 `LightZ` 改回几何平均 | `F1 spot: feSpotLight@z == 45.276928`（44.72136）、`F1 spot: xor 0 / md 0`（36986/206）、`F1 point: fePointLight@z`（44.72136）、`F1 point: xor 0 / md 0`（94939/2）、`F2: z/pointsAtZ are 45.276928/22.638464`、`F2: xor 0 / md 0`（35705/193）、`F4: obb light scaled like user space`、`F4: xor 0 / md 0`（36986/206）⇒ 67/8 | 75 green |
| M2 光源只缩不平移 | `F3 after: second cell light at x=120/pointsAtX=150`（x=20/50）、`F3: xor 0 / md 0 against the translated oracle`（5277/238）⇒ 73/2 | 75 green |
| M3 给光源加回 obb 门控 | `F4: obb light scaled like user space`（x=20, z=10）、`F4: xor 0 / md 0`（100170/242）⇒ 73/2 | 75 green |
| M4 非有限值改回写 `inf` | `F7-blur: stdDeviation kept byte for byte`（inf）、`F7-blur: no inf/NaN in the output` ⇒ 73/2 | 75 green |
| M5 位移 scale 改回轴因子 | `F5 x after: scale == 2 (sqrt(4))`（4）、`F5 x after: the rect moves +2 px`（48）、`F5 xy: scale == 2.115`（4.472）、`F5 y: scale == 2.236`（5）⇒ 71/4 | 75 green |

## 6. 测试计数（新增 / 删除 / 改写）

- **新增 16**：`src/transform/tests_r60.rs`（spot/point 的 z 与 pointsAtZ RMS、等向仍等于
  因子、双 cell 平移、缺省位置也平移、obb 光源仍缩、位移三种轴策略、子区域 userSpace 与
  obb、非有限四处保原文、畸形 number-optional-number）。
- **删除 1**：`tests_r58.rs` 的 `displacement_scale_falls_back_to_the_geometric_mean`
  （它冻结的 44.721 是错的；语义由 tests_r60 的三条取代，文件因此从 19990 B 降到 19680 B，
  P2-7 解决，本轮不再动它）。
- **改写 3 条既有断言**：`tests_r59.rs` 的 spot `z` 44.721→45.276928；
  `tests/selftest/compose/lenient.rs` 的非有限期望 `NaN`→原文 `nan`（P2-4 后旧期望就是错的）；
  `round-data/R59/test.py` 的 E1 属性与像素断言（P2-3）。
- 改名 0。总数 **658 = 366 + 59 + 233**（旧 643 = 351 + 59 + 233），净增 15。
  已同轮更新 `round-data/R19/test.py` 的 `BASELINE`/`COUNTS` 与 `round-data/R57/test.py`
  第 6/200 行写死的总数（改快照就是签名）。

## 7. P2-5 / P2-6

- P2-5：`pitfalls-manager modify` 补记 R19 与 R57 的计数快照在 R59 从 633→643 的变更
  （本轮又改成 658，另记一条）。事件日志，不改历史。
- P2-6：本报告 §6 已按"新增 N、删除/改名 M、净增 K"写，不再说"函数名集合只增"。

## 8. 登记（只登记、本轮不修）

- `LightZ` 是**渲染器语义的镜像**：resvg 若改 z 的公式，`lengths.rs:186` 必须同步，
  `round-data/R60` 的 F1 会红（这是有意的）。
- 位移的双轴单值无精确解（2.115 是折中），阈值留在证据里；若将来 SVG 允许
  `scale` 分轴，应改成精确双值。
- `R49` 之前的旧证据里 `x="inf"` 之类输入仍保原文（本轮之后工具不再重写它们，字节不变）。

## 9. 验收（实测）

- `cargo test --offline`：**366 + 59 + 233 = 658** 全绿，0 failed。
- `cargo fmt --all --check` 干净；`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` 后 `target/release/svg_grid` 与 `~/.local/bin/svg_grid`
  sha256 均为 `540281e1575a52b1962c219238d668602dc06c1423781837ddd0c4f7823c11aa`。
- `doctor`：**57 pass / 0 fail / 1 skip**（matplotlib 旋转轴标签，登记为不支持）。
- 逐字节回归：214 个 `tests/**/*.svg` 的 `convert --report`（输出 + 报告）与 58 个
  `tests/fixtures/*.svg` 的 `plot-grid --width 600 --ncol 1 --plan`，对 `/tmp/r50base`
  （@e0c6033，sha 75def0b8…）**0 差异**（214/0、58/0）。
- 旧证据 R3..R60 顶层全绿（`round-data/R49/shim/ran.txt` 追加 `R60 OK`），见运行输出。
