# R58 — filter primitive lengths, definition-body `font-size`, and invented 0×0 regions

**状态：已修（A/B/C 三族），残余已登记。没有"全部修好"这回事：见 §7。**

本报告是本轮的账本来源文件（`pitfalls-manager record --from-json` 的 `reports`），
可跑证据在 `docs/PITFALLS/round-data/R58/{test.py,meta.toml}`。

## 1. 现象（三个独立缺陷）

1. **A：滤镜图元长度不跟随面板缩放。** `<filter>` 不写 `primitiveUnits`（默认
   `userSpaceOnUse`）时，`feGaussianBlur stdDeviation="2"` 在 2× 面板里仍是 2：
   d1render 与"源图整体 2×"的 oracle 相差 **xor 8360 / maxdiff 89**；写成 `4` 后
   **xor 0**。同一原因波及 `feOffset`、`feMorphology`、`feDropShadow`、`feTurbulence`、
   `feDiffuseLighting`/`feSpecularLighting` 的 `fePointLight`、`feImage`、
   `feDisplacementMap`。
2. **B：定义体（pattern / mask / clipPath body）里的 `font-size` 不缩。**
   pattern body `font-size="12"` 经 2× 面板后 `x`/`y` 缩了、字号仍是 12：墨量
   **1092（oracle）/ 276（产物）**，xor **1524 / maxdiff 255**；手改成 24 后
   **xor 0，墨量 1092/1092**。`style="font-size: 12px"` 与 `<tspan font-size="12">`
   两种写法同病。
3. **C：缺失的 filter/mask 区域被凭空写成 0×0，内容整块消失。**
   `<filter id="base" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="80">…`
   + `<filter id="f" href="#base"/>`：R46 之前的二进制产物 `f` 只有 `href`，d1render
   **墨量 9588**；R46 之后产物变成
   `href="#…-base" x="0" y="0" width="0" height="0"`，**墨量 0**（内容消失）。
   没有任何区域属性的 `<filter filterUnits="userSpaceOnUse">` 与 `<mask maskUnits="userSpaceOnUse">`
   是同一族的既有变体（三个二进制都是墨量 0）。

## 2. 复现（命令行，全部在本轮复跑过）

```bash
cd /home/nihao/bioagentforge/tools/svg_grid
D1=/tmp/d1_eval/render/target/release/d1render
export D1_FONT_DIR=$PWD/fontlib            # 少了它文本 ink 恒 0
# 源 100x80 -> cell 200x160（s=2）；oracle = 同一源图用 d1render 以 SCALE=2 渲染
target/release/svg_grid plot-grid --input src.svg --output out.svg --width 200 --ncol 1 --plot-margin 0
$D1 out.svg got.png 1        # SCALE 是位置参数，写 --scale 2 会报错
$D1 src.svg oracle.png 2
```

一键版：`python3 docs/PITFALLS/round-data/R58/test.py`（fail-closed：缺二进制/渲染器
默认红，`--allow-skip` 才允许跳过）。

## 3. 根因 A：`primitiveUnits` 才是图元长度的门，`filterUnits` 只管区域

- `filterUnits` 决定 `<filter>` 上 `x/y/width/height` 的坐标系（默认
  `objectBoundingBox`）；`primitiveUnits` 决定**图元内部**所有长度量的坐标系
  （默认 `userSpaceOnUse`）——两个属性互相独立、默认值相反。usvg 依据：
  `usvg-0.47.0/src/parser/filter.rs:170-171`（两个默认值），属性沿 `href` 解析：
  `usvg-0.47.0/src/parser/paint_server.rs:520-537`（`resolve_filter_attr`）。
- 旧代码把两者都挂在 `filterUnits` 上：`src/transform/walk.rs:245` 的门用的是
  `primitiveUnits`（本轮新增），而 R46 留下的滤镜分支（旧 `walk.rs:205`）直接
  `return Ok(())`、图元一个都不改；`src/transform/dispatch.rs:280-285` 的
  `filterUnits` 门只该管区域。
- 修法：新增 `src/transform/filters.rs`。
  `filters.rs:58 primitive_units_are_user_space()` 用
  `resolved_units(..., "primitiveUnits", UserSpaceOnUse)`（沿 href 继承）；
  `filters.rs:73 rewrite_primitives()` 递归图元、`filters.rs:87 rewrite_one()` 是
  完整表：`feGaussianBlur stdDeviation`、`feOffset dx/dy`、`feMorphology radius`、
  `feDropShadow dx/dy/stdDeviation`、`feTurbulence baseFrequency`（**除**）、
  `fePointLight x/y/z`、`feImage x/y/width/height`、`feDisplacementMap scale`。
  **不动**：`feConvolveMatrix`（order/kernelMatrix/divisor/bias/targetX/targetY/edgeMode
  是计数/核/偏置，不是用户长度）、`feSpotLight`/`feDistantLight`（角度）、
  `feColorMatrix`/`feComponentTransfer`/`feComposite`/`feBlend`/`feMerge`/`feFlood`/`feTile`
  （没有用户长度）。
- 调用点 `src/transform/walk.rs:245-247`：只有 `primitiveUnits=userSpaceOnUse` 才进
  `rewrite_primitives`；`objectBoundingBox` 原样（实测 `stdDeviation="0.05"` 保持、
  xor 0）。

### 3.1 各向异性：能写两个数的属性必须写双值，几何平均是错解

源 100×80 → cell 400×400（sx=4、sy=5），oracle = 把源图整棵内容包进
`<g transform="scale(4 5)">` 再渲染到 400×400。`feGaussianBlur stdDeviation="2"`：

| 产物写法 | d1 xor | maxdiff |
|---|---|---|
| `2`（原样） | 39808 | 137 |
| `8 10`（各轴乘） | **0** | 0 |
| `8`（只用 sx） | 24644 | — |
| `10`（只用 sy） | 22936 | — |
| `8.944`（几何平均） | 69016 | 139 |

⇒ `<number-optional-number>`（`stdDeviation`、`radius`、`baseFrequency`）在
`fx≠fy` 时必须写成 `"x y"`——`filters.rs:233 scale_pair()` 只在 `fx==fy` 时保留单值。
只有**单值属性**（`feDisplacementMap scale`）退到几何平均
`sqrt(sx·sy)`（`LengthAxis::Uniform`，`filters.rs:200-206`）；lighting 族的各向异性
无法精确（见 §7）。

`feTurbulence baseFrequency` 是**频率**，各轴**除**以因子：`0.05` →
`"0.0125 0.01"`，xor 2 / maxdiff 1（噪声本身不可逐位复现，**不能要求 0**）。
它也是唯一被项目"最多 3 位小数"写法坑到的量：`0.05/4` 被写成 `0.013`（偏 4%），
噪声整场错位 xor 68684；因此 `filters.rs:306 format_length(value, precise=true)`
对它用最短往返表示（`format!("{value}")`），其余仍用 `crate::format_number`。

## 4. 根因 B：定义体 `font-size`；门控是"在定义体里"，不是策略

- 旧 `src/transform/dispatch.rs` 的 `"text"` 分支只 `rewrite_pair(element,"x","y")`，
  `"tspan"` 同理；`lengths.rs:48 rewrite_scaled_length()` 本来就能同时改属性与
  `style="…"`（`src/style.rs:25 scale_length`），只是没人对 `font-size` 调用。
- 不能简单按策略开闸：mask/clipPath body 的坐标必须继续用继承的 `Xy`（带平移，
  否则 body 与已改写的区域错位），只有 pattern/marker 分支才派生
  `body_policy()`。`ARCHITECTURE/tag-protocol.md` 也写明 `font-size` 在任何策略下
  都不缩放——普通面板文本必须保持原样。
- 修法：`src/transform/walk.rs:85` 给 `transform_element` 增加 `body: bool`；
  pattern 分支 `walk.rs:172` 传 `true`，marker 分支传 `true`，通用递归用
  `walk.rs:265 let child_body = body || matches!(name, "mask" | "clipPath")`；
  `walk.rs:115-122` 用 `rewrite_scaled_length(..., LengthAxis::Uniform)` 改
  `font-size`（属性与 `style` 都走同一条路），`tspan` 一样。普通面板文本
  `body=false` ⇒ 一个字节都不动（回归测试 `panel_text_font_size_is_never_scaled`、
  `panel_text_style_font_size_is_never_scaled`）。
- 各向异性用 `Uniform`（几何平均）：源 100×80 → 400×400 时 `font-size` 12 →
  53.667，墨量 5328 vs oracle 5352（差 0.4%），但**任何单值都无法逐像素精确**
  （maxdiff 恒 255）⇒ 只登记残余（§7），不声称修好。

## 5. 根因 C：缺失的区域属性不许凭空写 0

- 机制：`src/transform/dispatch.rs:269`（mask）与 `:284`（filter）原先调用
  `primitives::rewrite_rect`（`src/transform/primitives.rs:80-118`），它对**缺失**的
  `x/y/width/height` 先 `unwrap_or("0")` 再**无条件写回**，于是把沿 href 继承来的
  区域覆盖成 `0×0`；内容被裁掉，d1render 墨量 0。
- 口径：**filter/mask 自己没有某个区域属性时，不要写它**——让渲染器按 SVG 语义取
  默认或沿 href 继承（R41-F5 对 `rewrite_pattern_tile` 已立过同样的先例）。
  新函数 `src/transform/primitives.rs:133 rewrite_region()`：用 `rewrite_axes` +
  按轴 `rewrite_scaled_length` 改写 `width`/`height`，只碰元素**自己写了**的属性；
  自带区域仍照常缩放（`dispatch.rs:269/284`）。
- 像素验证：filter href 变体删掉凭空写入的四个属性后马上回到 xor 0 / 墨量
  9532（A 未修时是 9360/89），mask 变体是**精确**的 xor 0 / 墨量 32000/32000
  （cairosvg 修前直接 `ValueError: The SVG size is undefined`，修后能渲染 716）。
- 语料面：`grep -rn '<filter[^>]*href' tests/ scripts/` = 0 命中；另扫
  `<text` 出现在 clipPath/mask/pattern/marker body 里也是 0 命中（183 个文件有
  clipPath，但 body 只有 rect），所以对 214 个语料文件应零字节变化——已实测
  （§8）。

## 6. 四列数字（d1render 仲裁；oracle = 源图整体放大到 cell 尺寸）

| 用例 | 修前 xor/maxdiff | 修后 xor/maxdiff | oracle 墨量 / 修后墨量 | cairo 前/后 |
|---|---|---|---|---|
| A1 blur `stdDeviation=2` | 8360/89 | **0/0** | 9532/9532 | 0/0（cairo 忽略滤镜） |
| A2 offset `dx3 dy5` | 1410/255 | **0/0** | 9360/9360 | 1650/0 |
| A3 morphology `radius=2` | 840/255 | **0/0** | 11049/11049 | 0/0 |
| A4 drop shadow | 2489/158 | **0/0** | 10949/10949 | 0/0 |
| A5 turbulence `baseFrequency=0.05` | 13804/150 | **0/0** | 0/0（无黑像素） | 0/0 |
| A6 diffuse + point light | 13417/250 | **0/0** | 4744/4744 | 0/0 |
| A7 specular + point light | 0/0 | 0/0（本就对） | 0/0 | 0/0 |
| A8 `feImage` 四属性 | 896/127 | **0/0** | 0/0 | 0/0 |
| A9 displacement | 3522/255 | 127/255（**残余**） | 127/0 | 0/0 |
| A10 convolve（不动） | 0/0 | 0/0 | 9600/9600 | 0/0 |
| A11 colour/composite/flood/tile | 0/0 | 0/0 | 13824/13824 | 0/0 |
| A12 blur 双值 `2 3` | 9128/78 | **0/0** | 9508/9508 | 0/0 |
| N1 blur 单值 aniso | 39808/137 | **0/0**（`8 10`） | 47708/47708 | 0/0 |
| N2 blur 双值 aniso | 45352/134 | **0/0**（`8 15`） | 47556/47556 | 0/0 |
| N3 morphology aniso | 6516/255 | **0/0**（`8 10`） | 55845/55845 | 0/0 |
| N4 drop shadow aniso | 11875/239 | **0/0** | 54778/54778 | 0/0 |
| N5 turbulence aniso | 69051/161 | **2/1**（噪声下限） | 0/0 | 0/0 |
| N6 `feImage` aniso | 4480/127 | 640/127（**残余**） | 0/0 | 0/0 |
| N7 offset aniso | 11640/255 | **0/0** | 46800/46800 | 12840/0 |
| N8 diffuse aniso | 68236/245 | 45934/2（**残余**） | 20077/21385 | 0/0 |
| B1 pattern body fs=12 | 1524/255 | **0/0** | 1092/1092 | 3036/2964（字体不同，cairo 非仲裁） |
| B2 mask body fs | 127/255 | **0/0** | 91/91 | 253/247 |
| B3 clipPath body fs | 127/255 | **0/0** | 91/91 | 148/0 |
| B4 pattern `style="font-size: 12px"` | 1524/255 | **0/0** | 1092/1092 | 3036/2964 |
| B5 pattern `<tspan font-size=12>` | 1524/255 | **0/0** | 1092/1092 | 3036/2964 |
| B6 普通文本（scale 1 回归） | 0/0 | 0/0（不动） | 87/87 | 0/0 |
| B7 viewBox pattern body（不动） | 0/0 | 0/0（`font-size` 仍 12） | 1092/1092 | 2964/2964 |
| B8 pattern text aniso | 6324/255 | 5880/255（**残余**） | 5352/5328 | 14442/14514 |
| C1 filter 只有 href | 14736/255（墨量 0） | **0/0**（墨量 9532） | 9532/9532 | 0/0 |
| C2 filter 无区域 | 14736/255（墨量 0） | **0/0**（墨量 9532） | 9532/9532 | 0/0 |
| C3 mask 无区域 | 32000/255（墨量 0） | **0/0**（墨量 32000） | 32000/32000 | ERR/716 |
| C4 filter 自带区域 | 9360/89 | **0/0** | 9532/9532 | 0/0 |
| C5 href + 自带区域 | 9360/89 | **0/0** | 9532/9532 | 0/0 |
| C6 obb 无区域 | 8360/89 | **0/0** | 9532/9532 | 0/0 |
| C7 mask 自带区域 | 0/0 | 0/0 | 32000/32000 | 716/716 |
| C8 href + obb 区域（保持 `width="1"`） | 4224/126 | **0/0** | 8776/8776 | 0/0 |

`cairo` 侧对滤镜族**不作仲裁**：它要么完全忽略滤镜（A1/A3/A5/A6… 前后都 0），
要么按设备像素而非用户单位处理（A2 修前 1650）；文本族 cairo 因字体不同有固有差异。
按派单，滤镜/定义体语义以 d1render 为准。

## 7. 残余清单（登记，不声称修好）

1. **lighting 各向异性无法精确**（N8）：非等比缩放会改变表面法线本身，
   `surfaceScale` 也随各轴变；实测所有候选写法 xor ≈1.5×10⁵ 量级（本轮最好
   45934/2）。**只做等向**；各向异性保持用户长度按轴缩放但不保证像素一致。
2. **`feImage` 各向异性残余 640/127**（N6）：渲染器在**变换前**的用户空间里做
   `preserveAspectRatio` 适配（oracle 的 20×20 用户方块先变 80×100 设备矩形，
   产物按 80×80 适配）。加 `preserveAspectRatio="none"` 能让本用例 xor 0，但对
   非方形图片的 letterbox 语义是错的，**故意不加**。
3. **`feDisplacementMap` 残余 127/255**（A9）：合成文档按 1× 设备分辨率采样噪声场，
   2× oracle 采样密度是它的两倍；这是分辨率相关的噪声采样差，不是长度漏缩
   （`scale` 已按几何平均缩放；缩小 scale 反而更差：3161/3522）。
4. **定义体 `font-size` 各向异性残余 5880/255**（B8）：一个 `font-size` 无法同时
   表达 sx、sy 两个比例；几何平均让总墨量最接近（5328 vs 5352），但字形逐像素
   不可能一致。要精确需给该 `<text>` 追加 `transform="scale(1 k)"`（k=sy/sx），
   属于改语义，**本轮只登记**。
5. **普通文本仍不随策略缩放**：这是 tag-protocol 的既定契约
   （`ARCHITECTURE/tag-protocol.md`：`font-size` 在任何策略下都不缩放），本轮没有
   改变它；只有定义体里的文本才缩。

## 8. 验证（本轮实测）

- `cargo test --offline`：**633 passed, 0 failed**（306 baseline lib + 35 new =
  341 lib；59 main；233 selftest），测试只增不减。
  - 旧断言更正一处：`tests/selftest/defs/mod.rs:189` 原先断言"filter 图元不缩放
    (`radius='2'`)"，那正是本轮 A 的缺陷；改成 `radius="4"` 并注明依据
    （s=2 实测 radius 2 ⇒ xor 840，radius 4 ⇒ xor 0）。测试数不变。
- `cargo fmt --all --check` 干净；`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` 后 `sha256sum target/release/svg_grid ~/.local/bin/svg_grid`
  **两侧一致**：`4dbcbe095b3dabea0215f65b54e416029b9c6877d4ec4dade6881a14598fcf7b`。
- `svg_grid doctor`：**57 pass / 0 fail / 1 skip**。
- 语料逐字节：214 个 `git ls-files 'tests/**/*.svg'` 的 `convert`（含 `--report`）
  + 58 个 fixture 的 `plot-grid --width 600 --ncol 1`（含 `--plan`）对
  `/tmp/r50base/target/release/svg_grid`（@`e0c6033`，sha `75def0b8…`）
  **0 差异**（rc、stdout、产物字节全部相同）。
- 旧证据 R3..R57（53 轮，R49 shim 顶层模式，逐轮 `test.py`）：**51 轮 rc=0**。红的两轮都只红在
  "测试计数快照"上，没有一条行为断言失败：R19 把总数钉死为 598/306（本轮新增 35 条 lib 测试后实测
  633/341），R57 断言 R19 快照"本轮精确"且其源码写死 `== 598`。两个脚本自己都写明"增删测试必须同轮
  更新"，但本轮派单禁止改别人的证据脚本，故未改动；R57 的 `== 598` 是写死的，即使更新 R19 也不会变绿，
  需要后续轮次统一处置。
- 变异实测（7 条，每条都真跑、列出变红的测试名）：见 `round-data/R58/meta.toml`。

## 9. 体积与文件

- 新文件 `src/transform/filters.rs`（11608 B）、`src/transform/tests_r58.rs`
  （19622 B）；改 `dispatch.rs`（11902 B）、`walk.rs`（13587 B）、
  `primitives.rs`（9940 B）、`mod.rs`。全部 ≤ 20000 B（R8/R9 门禁集合）。
- `round-data/*/test.py` 不在该门禁内（R57 已把口径限定到 `src/`、`tests/`、
  `scripts/` 的 `.rs`/`.py`）。
