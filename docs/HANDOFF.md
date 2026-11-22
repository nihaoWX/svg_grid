# 交接文档 — `svg_grid`（2026-11-22）

> 本文件是 2026-11-22 的审计交接：缺陷已复现并定位到 `file:line`。规则来源是用户手写的
> `INSTRUCTIONS.md`（最高优先级）。**修复进度见下面的 §0**，修复后的语义以 `PROGRESS.md` 与
> `ARCHITECTURE/` 为准；§2 起的缺陷描述保留作复现步骤。

## 0. 修复进度（2026-11-22 晚更新）

| 缺陷 | 状态 | 提交 / 账本 |
|---|---|---|
| D1 convert 填死字形孔 | ✅ 已修，对抗评审 PASS | `1e05ce1`→`3a9747a`，`1da48f0`→`64e4551`；R27/R28 |
| D2 末行格子尺寸随行拉伸 | ✅ 已修，评审 PASS | `66fba3c`→`cf76029`→`f8be8ef`→`8b08524`；R29 |
| D3 默认网格是单行 | ✅ 已修，评审 PASS | `e09c33b`→`dc9a900`；R30 |
| D6 `rows×cols` 上界 + `--verbose` | ✅ 已修；D6b 关闭性复核通过（R4–R14 绿） | `3311eec`→`676d82d`；R31；连带面 `79cf9e5`/`dc0bda6` |
| D4 `plot-grid` 自动 convert | ✅ 已修 | `162c0cd`→`79cf9e5`→`a2d5fb5`；R32 |
| D5 `--labels` NULL/W3/W8 | ✅ 已修 | `e9425d2`→`398a21e`；R33 |

## 1. 先读什么（按顺序）

1. `INSTRUCTIONS.md` —— 用户手写、只由用户改。**产品定义与所有语义以它为准**。
2. `PROGRESS.md` → `# Known Issues` —— 8 条待办，每条带 `file:line` 与实测数字。
3. `ARCHITECTURE.md` → 用户期望 **#9–#13**（本次新加的四条）。
4. `ARCHITECTURE/convert.md` → §3 末尾的 **⚠️**（bridge 净绕数为 0 只对 N=2 成立）。
5. `ARCHITECTURE/layout.md` → §2 末尾两条 ⚠️（末行格子尺寸、`rows×cols` 校验）。
6. `ARCHITECTURE/cli.md` → `--verbose` 契约。

## 2. 用户最终定下的规则（实现依据）

| 主题 | 规则 | 出处 |
|---|---|---|
| 单次调用 | 一个**二维网格**放图；**不支持跨格（span）** | `INSTRUCTIONS.md` §网格组图 |
| 填满 | 除**声明过的 `NULL`** 外不许空出一格；显式 `ncol/nrow` 时空位必须写 `--input NULL` | 同上「重在声明」 |
| 默认网格 | 不给 `ncol/nrow`：`cols=ceil(sqrt(n))`、`rows=ceil(n/cols)`，**末格自动当 NULL**。15→4×4，5→2×3 | 同上 |
| `rows×cols` | 必须装得下；且**不许有整行全空**（显式 `NULL` 不算空）。例：5/6 个输入给 3×3=9 格 ⇒ 报错 | 同上 `--ncol/nrow` |
| 格子尺寸 | **只由网格决定**：行高按该行占满 `cols` 列算，**不随该行实际几个图变** | cowplot 1.2.0 实测 + 用户"尽量填满" |
| `--labels` | 作用于该次调用的直接输入；按调用层可调；裸 `--labels` 的 AUTO **跳过 NULL 槽**；显式列表**按槽位**（含 NULL），`""` 跳过 | 同上 `--labels` |
| `--plot-margin` | **默认 24**；每层都加、**允许累积**；只在设置它的那层可调 | 同上 |
| `--verbose` | 默认只输出"必要信息"= **报错**（`ncol/nrow` 不合法、重复给 labels、缺字体）；其余 W1–W9 静音 | 同上 `--verbose` |
| 占比 | = 该散图在最终图里**占几格**，**不是** `--rel-*` | 同上 流程第 2 步 |
| `convert` | **必须自动**，且**不许损坏原图**（字形孔必须保真） | 同上 |

两个 5 图经典例子（用户后续都按这两个指代）：

- **例子 1**：`ABC` 行（3 图）+ `DE` 行（2 图）上下叠，**共 5 格无 NULL** ⇒ 只能靠**嵌套最小步骤**做出来。
- **例子 2**：`ABC` 行 + `DE+NULL` 行，**共 6 格有 NULL** ⇒ 单次 2×3 即可，也正是默认行为。

## 3. 已确认的缺陷（按建议修复顺序）

### D1 `convert` 把 N≥3 子路径的字形孔填死（产物错，最优先）

- 位置：`convert/src/translate/shapes/path_form.rs:52`（纯填充分支）、`:83`（填充+描边拆分分支），
  两者都调 `join_closed_points`（同文件 `:155-166`）。
- 根因：N 条子路径首尾相接后，`<polygon>` 的**隐式闭合**又从末点连回整串首点 ⇒ N 条 bridge 成**闭环**。
  **N=2** 时两条 bridge 是同线段反向、面积 0；**N≥3** 时闭环是真多边形，圈内 winding ±1 ⇒ 内孔被填死。
  现有测试 `tests.rs:29`/`:139` 恰好只覆盖 N=2，所以全绿。
- 实测（`research/白藜芦醇胰腺癌/.../pandamap_all/pandamap_ALB_A.svg`）：
  - 14 个 `<text>` **逐字节保留**（用户以为"文字变 polygon"是误判：残基标签在原图里本来就是字形 `<path>`）；
  - 46 条 ≥3 子路径的 path **全部命中**，24/94 个字孔翻转；"ASP 451" 的 bridge 环面积 192.968 单位²；
  - 隔离渲染黑墨 18,312 → 29,016 px（**1.58×**）。曲线展平**不是**原因（最大偏差 0.029 px @2×）。
- 最小复现：`/tmp/render/donuts2.svg`（2 个甜甜圈 = 4 条子路径）；或用
  `<path d="M0 0 L40 0 L40 20 L0 20 z M10 5 L10 15 L20 15 L20 5 z M26 5 L36 5 L36 15 L26 15 z" .../>`。
- 修法：`subpaths.len() >= 3` 的填充路径**不要合并成单 polygon**，直接输出展平后的 `<path>`（`M/L/Z`，
  resvg 完整支持、语义无损）。同时改掉 `translate/shapes/mod.rs:33-37` 的错误论证，并补 N≥3 回归用例。

### D2 末行格子尺寸随"该行实际有几个图"变（E 被拉长的根因）

- 位置：`src/layout.rs` 的 `row_aspect_sums`（`:197-208`，`index < count` 在 `:203`）与
  `natural = available_w / sum`（`:122-130`）。
- 现象：5 输入 `--ncol 2` ⇒ 末行只有 1 个面板 ⇒ 该格 **476×972.614**、`Δ=−50%`；其余 4 格 476×486.3。
  根因一句话：**行高按"本行几个图"算、列宽按"全网格几列"算**，两套基准。
- cowplot 1.2.0 实测（`Rscript --vanilla`）：5 图 `ncol=2` 的 5 个格子全 `1/2 × 1/3`，**同格、不拉伸**；
  5 图默认 3 列 ⇒ 3+2、左对齐、第 6 格空。
- 修法：**格子尺寸只由网格决定** —— 行高 `= available_w / Σ(column weights)`（`--rel-heights` 仍作倍数），
  末行少几个图不影响格子大小。**不要**改成"末行跨满宽"（那是早期被旧措辞带偏的错误方向）。

### D3 默认网格是"一行"，与 cowplot 相反

- 位置：`src/layout.rs:69-74`，`(None, None) => (1, spec.count)`。
- 现象：5 图不给 `ncol/nrow` ⇒ `1×5`（`--plan` 实测 canvas 1000×242.5）；用户要 `2×3`（末格 NULL）。
- 修法：`cols = ceil(sqrt(n))`、`rows = ceil(n/cols)`；末格自动当 NULL。

### D4（修前）`plot-grid` 不会自动 convert

- 位置：`src/geom.rs:21`（`find_panel_box` 的报错）；唯一调用点 `src/plan.rs:144`
  （`plot_grid` 与 `plan_grid` 共用）。
- 现象：未打标签输入 ⇒ `rc=2`，`input SVG must contain a <rect data-panel-box="main" .../> element`。
- 现成钩子：`svg_grid_convert::convert(&str, bool)`（`convert/src/pipeline.rs:22`，纯字符串、无 I/O；
  `Cargo.toml` 已依赖 `convert`，**无需新依赖**）。`doctor` 的进程内调用可作范式：
  `src/cli/doctor/run.rs:205` → `convert_in_process`（`:467-484`）→ `run_in`。
- 注意：已组好的组合图有多个 panel-box，`convert` 会报 `Incomplete`——但这类输入本来就有 panel box，
  不会进自动分支。

### D5（修前）`--labels` 的 NULL 规则与"必要信息"报错

- 现状：`src/labels.rs:16-18` 按**输入下标**取字母；空位（`None`）在 `lib.rs` 的循环里 `continue`，
  所以 NULL 槽永远拿不到字母。
- 需要：裸 `--labels` 的 AUTO **跳过 NULL**；显式列表**按槽位**（含 NULL）——给 NULL 槽写字母就画、`""` 跳过。
- W3（给已带字母的输入再赋非空字母）现在只是告警（`src/lib.rs:367-379`、`src/warnings/labels.rs:18-36`），
  必须改成**报错**（属于 `--verbose` 规则里的"必要信息"）。
- W8（缺字体族、静默回退）也要改成**报错**。
- 已验证：`--labels ,C` 对已带字母的输入**不触发 W3**，输出 `A、B（继承）+ C`——这就是用户 `plot_grid.R` 的写法。

### D6（修前）`--verbose` 未实现；`rows×cols` 上界没校验

- `--verbose`：现在 W1–W9 与聚合 `note:` 一律打 stderr（`src/warnings/mod.rs` 的 `emit`）。
  需要默认静音、`--verbose` 才展开；`--plan` 的 stdout `note:` 行不受影响（它是计划契约）。
- 网格校验：现在只查 `rows*cols < count`（`src/layout.rs:75-77`）；需补"不许有整行全空"。
  注意这个规则等价于 `rows == ceil(count/cols)`（装得下 + 无空行 ⇒ 行数必为最小）。

## 4. 环境与验证入口

- 二进制：`~/.local/bin/svg_grid`（本次核实与 `target/release/svg_grid` 同 sha256）。
  **改完代码必须重跑 `./install.sh`**（见 `ARCHITECTURE/install.md`）。
- 回归：`cargo test`（主 crate 74 lib + 59 selftest；`convert/` 81 lib + 12 CLI），
  `svg_grid doctor`（**57 pass / 0 fail / 1 skip**；`matplotlib/mathtext` 是登记的 SKIP）。
- R：`~/micromamba/envs/r453/bin/Rscript --vanilla`——**必须加 `--vanilla`**，否则启动时 BiocManager
  联网校验会卡 60 s 然后失败。cowplot 1.2.0 已装。
- 本次会话留下的临时材料（重启后可能没了，需要就重建）：`/tmp/sgtest/`（ncol=2 复现、嵌套）、
  `/tmp/render/`（字形孔对比渲染、donuts2）、`/tmp/svg_grid_audit_report.md`、`/tmp/svgaudit/REPORT.md`。

## 5. 账本（`pitfalls-manager`）——已迁移，别再当坑

**2026-11-22 已完成 reanchor**（用户指定锚点改为工具自己的仓）：

- `meta.repo_paths` = **`["/home/nihao/bioagentforge/tools/svg_grid", "/home/nihao/bioagentforge"]`**。
  第一个是工具仓（新记录用），第二个是旧 monorepo（**必须保留**：R1–R26 的 `code_before/after`
  与报告 commit 只在 monorepo 里，去掉会让 `record` 直接失败）。
- `pitfalls-manager check --repo .../tools/svg_grid`：迁移前 **46 条** → 迁移后 **0 条**（0 字节）。
- `query src/layout.rs` 仍命中 `R14-F1`；`ids` 迁移前后逐字一致；对 46 条 distinct `where` 全扫，
  **`⚠ suspect anchor` = 0**；新轮 `record` 在临时库里 `rc=0`。
- **`where` 故意没重写**（仍是 `tools/svg_grid/...`）：那是旧快照里的真实坐标，也是 `git show` 的取回坐标；
  改成 `src/...` 会让 `validate_ledger` 整本失败并毁掉旧证据取回。并轨靠新增的 **`link` 组 L11–L55**
  （`tools/svg_grid/<p>` ↔ `<p>`）与 1 条 retired（L56，R11-F2 的跨项目路径）。
- **报告路径未改**（`tools/svg_grid/docs/PITFALLS/...`），旧证据仍可 `git -C /home/nihao/bioagentforge show <sha>:<path>`。
- 工具侧新增受支持的 **`reanchor`** 子命令（`tools/pitfalls-manager` commit `bcb19c0`，已装 `~/.local/bin`）。
- 回滚备份：`/home/nihao/pitfalls-manager-backups/svg_grid-reanchor-20261122-221338/`（含迁移前 `history.db`、
  前后 `check/query/ids`、REPORT.md）。
- **新轮次**：`where` 用工具仓相对路径（`src/layout.rs`、`convert/src/...`）；查询旧路径靠后缀匹配 + link。
- `docs/PITFALLS/` 目前**没有待处理报告**（只剩 `round-data/R3…R26` 的可跑证据）。

## 6. 本次会话的提交（都在本目录的仓）

```
9335155 HARNESS: 第五次对齐（ncol/nrow 校验、--verbose、margin 24、修正末行修法）
8f1571b HARNESS: 第四次对齐（两个 5 图经典例子 / NULL 字母规则 / margin 默认 24）
2bdcdc2 HARNESS: 第三次对齐（默认网格 / NULL 声明 / margin 分层）
2124c70 HARNESS: 第二次对齐（二维网格语义、NULL 声明、labels 分层）
5b06f14 HARNESS: 首轮验收审计（四条要求全部未满足 + file:line）
```

> `INSTRUCTIONS.md` 是用户的文件；这次每轮对齐都把它一起提交了。用户随后又改过一版，本交接文档
> 与它同一提交入库。

## 7. 建议的下一步

1. **先解账本锚点**（§5），否则 `record` 可能过不了闸门。
2. 按 **D1 → D2 → D3 → D6 → D4 → D5** 动手（先产物正确性，后易用性）。
   每轮走 `query <位置> --repo .` → 改 → `cargo test` → `doctor` → `install.sh` → commit → `record`；
   若与旧轮同址（复发），在 `docs/PITFALLS/round-data/<新轮>/` 留可跑证据。
3. D1 修完用 `pandamap_ALB_A.svg` 做**逐像素**对比（原图 vs 转换后再组图），并把 `tests/user_tests/2026-11-20/`
   当真实语料跑一遍：那里现在放着 5 个已转换的 panel 和用户当时的成品 `ABCDE.svg`（注意：那份成品
   与 `--ncol 5` 的输出**逐字节相同**，不是 README 里写的 `--ncol 2`，别拿它当 ncol=2 的期望值）。
4. 用户 2026-11-22 的提交 `6e582be` 删掉了 `tests/poppler_dialect` 与 `tests/shape_paint`
   （后者含 `fixture_glyph_hole.svg`，但只测 N=2）——**先确认是否你本意**；若是有意，D1 的回归用例
   要新建；若是误删，从 `6e582be^` 取回。

## 8. 未决 / 需要用户点头的点

- §5 的账本锚点。
- 显式 `NULL` 是否允许出现在**列表中间**（例子 2 只演示了末尾；实现上建议允许）。
- W8 改成报错后，**只出 PNG/AVIF 时的缺字体**是否也报错（现在空库是 `rc=3`，缺族是告警）。
- `--verbose` 是否也覆盖 `convert` 子命令的告警（建议覆盖，保持一致）。
