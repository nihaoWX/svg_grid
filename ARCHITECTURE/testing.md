# 测试语料库与 `doctor`

**自检是一件事，不是两套概念**（用户 2026-10-06 裁决）：它就是"这个工具还能正常工作吗"，分两层而已——

| 层 | 入口 | 跑什么 | 什么时候用 |
|---|---|---|---|
| **源码层** | `cargo test`（**唯一一个测试目标** `tests/selftest/`，模块按主题：`compose/`、`labels/`、`layout/`、`transform/`、`warnings/`、`refs/`、`fonts/`、`doctor/`…） | 直接调库函数的细粒度断言 + doctor 的 CLI 行为 | 改代码时（编译期、秒级） |
| **运行期层** | `svg_grid doctor` | 58 张真实语料的**全链路**（convert → compose → PNG） | 装好之后（`install.sh` 已把语料装到数据根） |

两层的 fixture 来源不同（源码层用手写最小 SVG；运行期层用真实绘图包产物），**但语义上都是自检**，
所以不要再出现"compose_tests 是测试、doctor 是工具"这种切割：doctor 是自检的**运行期入口**，
`cargo test` 是自检的**源码入口**，`tests/` 下不按"给 compose 测 / 给 doctor 测"分目录。

**一句话**：测试不再靠手写 SVG，而是**用真实绘图包生成一批 SVG**（ggplot2 / matplotlib / seaborn），
入库成"语料集"；`svg_grid doctor` 把整集跑一遍，一条命令给出"哪张图过不了、卡在哪一步"。

## 为什么

现有 `tests/selftest/` 里的面板是**手写的简单 SVG**（`common/mod.rs` 里的 `PANEL_100X80` 之类），
只覆盖了"我们想到的"构造。真实世界的 SVG 来自绘图包：`<use>`/`<g transform>`/渐变/裁剪/虚线阵列/
mathtext/位图内嵌/facet 子图……这些才是会出问题的地方。手写永远追不上。

## 语料集（`tests/fixtures/`）

```
tests/fixtures/
├── MANIFEST.toml          # 每条 fixture 的登记表（见下）
├── ggplot2/               # 生成脚本 *.R + 产物 *.svg（**都要入库**：SVG 是文本）
├── matplotlib/            # 生成脚本 *.py + 产物 *.svg
└── seaborn/               # 同（选做）
```

**规矩**

- **入库的是源**：生成脚本 + 产出的 `.svg`（文本）+ `MANIFEST.toml`。**PNG 不入库**（生成物）。
- 生成参数**确定化**：固定尺寸（约 3×2 in）、固定 `seed`、不依赖随机数据；
  matplotlib 用 `svg.fonttype = 'none'`（文字保持 `<text>`，不转路径）；每张图 **≤ 60 KB**。
- **按图型穷举**，不是按"好看"挑。至少覆盖：
  - ggplot2：point/line/path/step、bar/col、histogram/density、boxplot/violin/dotplot、
    smooth(loess)、text/label（hjust/vjust/angle）、tile(heatmap)/raster、polygon/ribbon/area、
    errorbar/linerange、segment/curve、hline/vline/abline、jitter、quantile、contour、
    coord_flip/coord_polar、facet_wrap/facet_grid、scales(log/reverse/date)、
    guides(legend 位置/合并)、themes、annotate(text/rect)、scale_fill_gradient、alpha/shapes/linetypes。
  - matplotlib：plot(多线)/scatter(大小+色标)/bar(分组+堆叠)/barh/hist/boxplot/violinplot/errorbar/
    fill_between/step/stem/pie、imshow(位图内嵌)/pcolormesh/contour(f)/quiver/hexbin、colorbar、
    twinx/twiny、subplots/gridspec、log/date 轴、mathtext(`$\\alpha$`)、annotate(arrow)、
    patches(Rectangle/Circle/Polygon/Wedge/Ellipse/Path 曲线与孔)、legend、suptitle。
  - seaborn（选做）：heatmap/kdeplot/regplot/catplot。
- 每张 fixture 在 `MANIFEST.toml` 里登记：
  ```toml
  [[fixture]]
  id = "ggplot2/geom_boxplot"
  package = "ggplot2"
  call = "geom_boxplot() + geom_jitter(width=0.1)"     # 一句话说明这张图在测什么
  covers = ["<use>", "<g transform>", "stroke-dasharray"]   # 它带来的构造（作为覆盖度证据）
  expect = "pass"        # pass | unsupported（配 why = "..."）
  ```
  `expect = "unsupported"` 只给**确实不支持**的构造（例如 fp 精度无损的场景），必须写 `why`；
  `doctor` 遇到它算 **SKIP 并打印原因**（不许静默跳过）。

## `svg_grid doctor`

```
svg_grid doctor [--fixtures <dir>] [--only <glob>] [--jobs <n>] [--json] [--keep <dir>]
```

**语料目录怎么解析**（前一个命中即止，不做别的探测）：

1. `--fixtures <dir>`
2. 环境变量 `SVG_GRID_FIXTURES`
3. **数据根** `$HOME/svg_grid/fixtures`（`install.sh` 把语料同步到这里）
4. 仓库内开发用：`<exe dir>/../../tests/fixtures`（只在 `target/debug/…` 这类仓库构建里成立）

1/2 是**显式**的：给了就照用，指错就报配置错（rc=3），不悄悄换一个语料跑；3/4 是**候选**，
第一个存在的目录胜出。装到 `~/.local/bin` 的二进制因此默认读数据根（R17 装了两种都断言）。

对每条 fixture **走完整链路**（真实管线是两段）：

1. 转换（`svg_grid_convert` **库**，进程内调用；不再去找外部二进制）`<fixture.svg>` → `<tmp>/tagged.svg`（方言 → 协议格式）
2. `svg_grid plot-grid --input <tmp>/tagged.svg --output <tmp>/fig.svg --output-png <tmp>/fig.png --png-dpi 300`
3. 判据（任一不成立 = FAIL，报文说清是哪一步、卡在哪条判据）：
   - 两段都 `rc == 0`（`unsupported` 允许第 1 段非零，但必须打印原因且计 SKIP）
   - 产出的 SVG/PNG 非空，PNG 不是纯白（墨水像素 > 0）
   - `stderr` 里**没有 W7（透传/未识别）和 W9（悬空 `url(#…)`）**；其余告警只计数不算失败
4. 输出：每条一行 `PASS/FAIL/SKIP  id  step  ms  warnings=N`；末尾
   `summary: N pass, M fail, K skip`；有 FAIL ⇒ **退出 1**。`--json` 给机读版（给 agent/CI 用）。

**语言**：工具输出一律**英文**（`ARCHITECTURE/warnings.md` 的规矩同样适用于 doctor）。

## 与既有测试的关系

- `cargo test`（127 项）继续做**单元/回归**；`doctor` 做**语料集全链路回归**，两者不互相替代。
- 语料集是"后续测试的默认集合"：新加图形能力、改 `convert/`、改告警判据之后，先跑 `doctor`。
- 证据：`round-data/R16` 跑 `doctor` 并要求 `rc=0`，语料条数与家族覆盖也一并断言。
