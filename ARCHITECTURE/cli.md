# CLI 契约：参数面、退出码、`--help`、字体目录解析

使用侧的入口只有一条命令；命令面的契约写在这里，**skill 不复述**（只给指针）。

## 子命令与参数面

```
svg_grid plot-grid --input A.svg [...]      # 组图
svg_grid plot-grid --input A.svg [...] --plan
                                            # 只报布局（画布 / 每格尺寸 / s），零写入
svg_grid convert --input PANEL.svg [--output OUT.svg] [--check] [--report] [--force]
                                            # 方言 → 协议格式（原独立二进制 svg_grid_convert，现为子命令）
svg_grid doctor [--fixtures <dir>]          # 自检：把 fixture 集整体跑一遍（见 ARCHITECTURE/testing.md）
svg_grid --help | -h                        # 用法，**stdout**，退出 0
svg_grid --version                          # 版本，stdout，退出 0
```

- **同一语义只有一种拼法，不保留未文档化别名**（用户裁决，R26）：`--font_dir`、`--label_colour` 这类型
  下划线写法已从解析器删除，只认 `--help` 里列出的 kebab-case 那一种；任何未登记的写法一律
  `unknown flag "<x>" (run `svg_grid --help` for usage)`、退出 2，**不许**静默接受。
  `src/cli/parse.rs` 的 `FLAGS` 仍保留「正名 + 别名」两列，但**别名列对每个 flag 都是空的**，
  且 `canonical()` **只读正名列**（别名列即便被填也不会被接受，单测 `no_flag_has_an_alias` 锁住它为空）；
  「帮助覆盖全部 flag」的单测直接走这张表。

- **只有一个二进制 `svg_grid`**（用户 2026-10-06 裁决）：`convert` 是**子命令**，不是第二个可执行文件；
  `convert/` 仍是独立 crate（为了结构清楚），但以**库依赖**被主 crate 用（`src/cli/convert.rs` 只是薄壳）。
  `convert/` 自己的 `svg_grid_convert` 二进制留在工作区供其进程级测试用，**不再安装到 PATH**。
  ⇒ `doctor` 不再需要"找外部 convert 二进制"，那个 `exit 3` 分支随之消失。

- 参数**有意保持少**，且每个参数都要么有智能默认、要么是"用户真要改才传"的旋钮；
  文档/skill 里**不再复述**参数语义（`--help` 是唯一权威）。
- 帮助文本必须**覆盖全部 flag**（单测锁死：解析器认识的每个 flag 都要在帮助文本里出现一次）。

## `--plan`：只报布局，零写入（R26）

调用方不再手算「每格多大」：给**与真跑完全相同**的输入/布局参数（`--input/--ncol/--nrow/--rel-*/
--width/--height/--plot-margin/--gap/--normalize-input-max-side/--labels`），加 `--plan` 就打印这张图会怎么排。

- **零写入**：不写文件、不栅格化、不读字体库；与任何输出 flag（`--output`/`--output-svgz`/
  `--output-png`/`--output-avif`）同时给是**用法错**，退出 2（"静默忽略输出"会让调用方以为图已生成）。
- **同一处计算**：`plan_grid()` 与 `plot_grid()` 共用 `lib.rs::prepare()`（读入 → `--normalize-input-max-side`
  → `compute_grid`），`sx/sy` 共用 `geom::cell_scale()`；计划里没有任何第二套布局算术。
- **输出契约**（stdout，英文，确定性，输入顺序；数字用与 SVG 相同的 `format_number`）：

```
canvas <W>x<H>                                          # = 最终输出 svg 的 width/height
#<i> <path>  canvas <w>x<h>  cell <w>x<h>  s=<sx>x<sy>  Δ<percent>
note: …                                                 # 每个输入恰好一行
```

  - `canvas` 是输入**归一化之后**的画布（即运行时真正用的那个）；`cell` 是该输入落进的格子；
    `s` 就是 `cell / canvas`；`Δ = (sx/sy − 1)`，`0.00%` = 未变形（判变形仍以 W2 的 1% 容差）。
  - `note:` 说明该输入**是否需要先归一化**：需要时下一行给出**可直接照抄**的命令
    `svg_grid plot-grid --input <src> --output <src 同级 stem>.cell.svg --width <格宽> --ncol 1 --plot-margin 0 --normalize-input-max-side <格长边>`；
    "需要"的判据就是 W1 的 `SCALE_TOLERANCE`（0.5%），所以计划与真跑的告警不会各说一套。
  - 空位（`--input none` / `--spacer`）打成 `#<i> (blank cell)  cell <w>x<h>`，不报 `s`。
- 退出码沿用分档：参数错 2、输入读不了 1（且**点名文件**，R24）。

## 退出码（分档，别都塞 1）

| 码 | 含义 | 例子 |
|---|---|---|
| 0 | 成功（**可以有非致命告警**；告警不改退出码——用户已裁决不做 `--strict`） | 正常组图 |
| 1 | 运行期失败（IO / 解析 XML 失败 / 栅格化失败 / 内部错误） | 读不到输入文件 |
| 2 | **用法或输入不合法**（参数值越界、未知参数、输入 SVG 结构坏） | `--ncol 0`、`--avif-quality 0`、未知 flag |
| 3 | **配置错**（字体库缺失/为空，属于环境问题） | `fontlib` 空且要出 PNG |

**铁律**：用户输入导致的错误一律 `Result`/`SvgGridError`，**不许** `assert!`/`unwrap()`/panic
（`rc=101` 只会让调用方以为工具本身有 bug）。`main` 不许把错误拍平成一串 `String`。

## `--help` 契约

- `-h/--help` 在**取值之前**被特判：`stdout` 打完整帮助，**退出 0**（不是错误）。
- 未知 flag ⇒ `stderr` 一句话 + 提示 `--help`，退出 **2**。
- 缺少必需项（`--output` 之类）⇒ 一句话 + 提示 `--help`，退出 **2**。
- 帮助文本住独立模块（`src/cli.rs`），不塞进 `main.rs`（`main.rs` 只做 `main()` 与委托）。

## 字体目录解析（**不许编译期绝对路径**）

顺序（前一个生效即止）：

1. `--font-dir <dir>`（显式）
2. 环境变量 `SVG_GRID_FONT_DIR`
3. **数据根** `<home>/svg_grid/fontlib`，其中 `<home>` 由 `src/home.rs` 在运行时解析：
   **`HOME` → `USERPROFILE`**（Linux/macOS 只有前者；Windows 原生 shell 只有后者；Git Bash 两者都有，
   取 `HOME`）。空值跳过（`env -i` 里 `HOME=` 不能让数据根变成 `/svg_grid`）。

- 数据根**只有一个实现**（`src/home.rs`），字体（`raster::data_root_font_dir`）与语料（`doctor`）都走它，
  免得两边对"家在哪儿"各有一套判断。
- 代码里**不许出现** `env!("CARGO_MANIFEST_DIR")` 之类把构建机路径烧进二进制的写法
  （判据：`strings <二进制> | grep -F "<构建机绝对路径>"` 必须为空）。
- 仓库内的 `tools/svg_grid/fontlib/` 只存 `README.md` + `setup.sh`（字体文件不入库）；
  `setup.sh [--dir <目标>]`（默认就是数据根）把本机字体复制进去 —— **安装脚本复用同一个脚本**，
  于是"开发时跑的"与"装完跑的"读的是**同一个目录**，不会各读各的。
- **Windows** 没有 `install.sh`（bash + rsync 的分叉不维护）：二进制同样在本机编译，数据根/字体/语料/skill
  由用户按 `docs/install-windows.md` 手动就位。文本里写数据根一律用 `<home>/svg_grid`，不写死 `~/`。

## 单测要求

- 解析器（`src/cli.rs`）必须有单测：每个 flag 的取值、越界、缺值、未知 flag、`--help`、
  帮助文本覆盖全部 flag、退出码分档。
- 单测不许依赖 cwd（字体目录一律用 `SVG_GRID_FONT_DIR` 或 `--font-dir` 指到临时目录）。
