# 阻塞点：`<use>` 引用的 `<defs>` 定义不跟随面板缩放（R64）

## 现象

源图 100×80、`plot-grid --width 200 --height 160 --ncol 1` ⇒ 面板 s=2。被 `<use>` 引用、写在
`<defs>` 里的目标（`<rect id="r">`、`<g id="g">`）在产物里**保持源尺寸**，而引用它的 `<use x y>`
已经按 s 缩放，于是定义被画到错的位置/尺寸：

```xml
<defs><rect id="r" width="10" height="10" fill="#000"/></defs>
<g data-scale="xy"><use href="#r" x="10" y="10"/></g>
```

| 用例 | 修前 xor/md | 修前 ink vs oracle | 修后 xor/md | 修后 ink vs oracle |
|---|---|---|---|---|
| u2 单 rect | 300/255 | 100 vs 400 | 0/0 | 400 = 400 |
| u3 组目标（rect+circle） | 408/255 | 133 vs 524 | 0/0 | 524 = 524 |
| u5 链（定义里的 `<use>`） | 372/255 | 100 vs 400 | 0/0 | 400 = 400 |
| u17 两个 `<use>` 同目标 | 600/255 | 200 vs 800 | 0/0 | 800 = 800 |
| u7 目标在可绘制树里 | 600/255 | 200 vs 800 | 600/255 | 200 vs 800（登记，见下） |

（xor 单位 = 不同像素数；派单/历史口径的 900/1224/1116/1800 是逐通道和，等于本表的 3 倍。
u16/u4 是纯属性用例，没有单轴/多策略 oracle，只断言属性：u16 `data-scale="x"` 修前 def
`10×10`、修后 `20×10` 且 `use x=20 y=10`；u4 修前两份副本都 `10×10`，修后 `r` 为 `20×20`、
`r--svg-grid-y` 为 `10×20`，且 `href` 指向各自的副本。）

修后的像素与 `d1render 源图 2`（`data-scale="xy"` 用例的正确 oracle）逐像素相等；cairosvg 复核
u2/u5 同结论。

## 根因

`<use href="#r">` **一直**被当成引用：R46 的 `referenced_ids` 已经解析 `href`/`xlink:href`
（`link_of_text` → `internal_target`），所以 `refs[r]` 里有策略、u4 的按策略克隆（`r` 与
`r--svg-grid-y`）因此早就生效。断点有两处：

1. `DefsPlan::slot_of` 先 `if !is_definition(element) { return None }`。七个 `DEFINITION_TAGS`
   （pattern/marker/mask/filter/linearGradient/radialGradient/clipPath）之外的目标拿不到槽位，
   引用者策略无处落地。
2. `referrers::collect` 只给 `is_definition` 元素登记 owner/bodies，定义子树里的引用边因此
   没有归属；`<g id="g"><use href="#r" x="2" y="2"/></g>` 的内层 `use` 坐标从未被缩。

## 修法

- `src/transform/defs.rs`：`slot_of(element, reference_only)`；`reference_only` = 元素在
  `<defs>` 里或在某个定义 body 里。`collect` 增加 `in_defs`，owner/bodies 的登记放宽到
  “`reference_only` 且带 id”的一切元素（不止七个定义 tag），`split_children` 的既有权重逻辑不变。
- `src/transform/walk.rs`：`transform_element` 增加 `in_defs`，用同一个 `reference_only` 查槽位；
  因此定义子树里的 `<use>`、`<rect>` 等和叶子图元一样按引用者的域缩放。
- `src/transform/domains.rs`：`body_is_verbatim` 增加 `symbol`（有合法 `viewBox` 时 body 已由
  `<use>` 的 width/height 映射到实例，属 Verbatim）。`<symbol>` **不**进 `DEFINITION_TAGS`：
  它只在被 `<use>` 实例化时渲染，body 域由 viewBox 决定，不是“按引用者复制”的对象。
- 字节保真：Verbatim 的**普通**元素不让 walk 直接 return，而是以 `None` 继续走一遍
  `rewrite_primitive`（数值 no-op，但会 materialise `<rect>` 的 `x="0" y="0"`）；symbol 分支
  也以 `None` 下探一层。这两条让两条对照的产物与修前**逐字节相同**。

### 手改产物验证（先证后写）

对**修前产物文本**只替换期望数字，再与源图 s=2 渲染比较：

- u2：def `10×10` → `20×20` ⇒ xor 0/0、ink 400=400。
- u3：def rect `10×10`→`20×20`、circle `cx=15 cy=5 r=3`→`30/10/6` ⇒ xor 0/0、ink 524=524。
- u5：def `10×10`→`20×20`、内层 `<use x=2 y=2>`→`4/4` ⇒ xor 0/0、ink 400=400。

三条都在 `test.py` 的 A2 族里可跑。

## 对照（必须逐字节不变）

- u6 `<symbol viewBox="0 0 10 10">` 经 `<use x=10 y=10 width=20 height=20>`：修前/修后 xor 0/0、
  ink 1600=1600，产物 `cmp` identical。
- u15 `viewBox` pattern body 里的 `<use href="#r">`：xor 0/0、ink 32000=32000（pattern 覆盖
  全画布，派单里 5120 是另一套取景），产物 `cmp` identical；tile 10→20、body 原样。

## B：`<foreignObject>` / `<switch>` 静默放行

实测（内联 fixture，修前二进制）：`convert --check --verbose` rc=0、stdout 含 `compatible`、
stderr 0 字节，写模式正常产出文件。`<switch>` 在 d1render 里只画第一个子元素（黑 100 px、红 0 px），
`<foreignObject>` 画 0 ink——即内容被静默丢掉。

**选择 fail-closed**：`convert` 对两者各报一条 structural finding（`--check` = NOT compatible、
写模式 rc=1、不写产物、stderr 带 `no output written`）。理由：convert 的契约是“产物会被
plot-grid 原样搬运”，foreignObject 的内容这里根本不渲染、switch 的条件这里无法评估，两者都会让
产物与浏览器画面不一致；登记为“原样保留”等于承认会丢内容。语料实测
`git ls-files 'tests/**/*.svg' | xargs grep -l '<foreignObject\|<switch'` = 0 命中，214 个
tracked svg 的 convert 产物字节 0 差，所以 fail-closed 的代价是 0。

## C：obb `clipPath` body 里引用 pattern（只登记，未做）

- 数字：默认 plot-margin、200×160、s=2 下修前/修后都是 **16000/255**（R62 口径）；用
  `--plot-margin 0` 复测是 12000/255。
- 装置：`<pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="4"
  height="4" fill="white"/></pattern>` + `<clipPath id="c" clipPathUnits="objectBoundingBox">
  <rect width="1" height="0.5" fill="url(#p)"/></clipPath>` + 红色 100×80 rect。
- 评审给的手工候选（clip body 保持 1×0.5、pattern tile 16/8）**在本轮复现不出 0/0**：产物里
  clip body 的 `x/y` 被写成面板坐标（x=24 y=24）；删掉 x/y、tile 改 16×8 / 16×16、tile 内 rect
  改 8×8，以及这些改动的组合，xor 仍全部是 16000/255（margin 0 时 12000/255），并且 tile 8 与
  16 的像素**完全相同**。d1render 与 cairosvg 对源图都画整块 16000 红、对产物只画 25% 覆盖的
  4000 红，说明两台 oracle 都不消费 obb clip body 内的 pattern 覆盖。
- 为什么没做：这不是“数字没算对”，而是“obb clip body 里的 pattern 该在哪个坐标系解析”还没有
  定义；语料 0 命中、不影响任何现有证据。候选复现记录见 `round-data/R64/meta.toml` 与
  `test.py`。**派单里“候选可 0/0”的前提在本装置上不成立**，已在本报告如实记录。

## 登记不改：`<use>` 的目标在可绘制树里

`<g id="g">` 同时是“原位内容”和“被引用内容”。给槽位会把原位绘制也缩放（双重缩放），克隆又把
body 画两遍；u7 修前/修后都 600/255（ink 200 vs 800）。留作已知残余。

## 回归与门禁

- `cargo test --offline`：**711 = 419 + 59 + 233**（基线 701，+10：`tests_r64.rs` 9 条 +
  `scan` 的 foreignObject/switch 1 条）；R19 的 `BASELINE`/`COUNTS` 与 R57 写死总数同轮更新。
- `cargo fmt --all --check`、`cargo clippy --offline --all-targets -- -D warnings` 干净。
- `./install.sh` 后 `sha256sum target/release/svg_grid ~/.local/bin/svg_grid` 一致
  （`0ff7d775…7f03`）；`doctor` 57 pass / 0 fail / 1 skip。
- 回归：214 个 tracked `tests/**/*.svg` 的 `convert` 与 58 个 fixture 的
  `plot-grid --width 600 --ncol 1` 对 `/tmp/r50base/target/release/svg_grid`（@e0c6033）
  rc + 产物字节 0 差。
- 变异（5 条，全部实跑变红）：`slot_of` 去掉 `reference_only`（A 6 条红）、`collect` 的
  `reference_only=false`（A/u5 红，定义子树内 `<use>` 坐标不缩）、symbol 分支禁用（B 2 条红）、
  Verbatim 对所有元素直接 return（B/u15 字节红）、scan 两条 arm 禁用（C 2 条红）。

证据：`docs/PITFALLS/round-data/R64/{test.py,meta.toml}`。修前二进制 =
`6742535` 的构建（sha256 `ef5fc28b…e329`），`SVG_GRID_PIN_BIN` 指向它时 A 族 6 条必须红。
