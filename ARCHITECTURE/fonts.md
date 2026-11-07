# 字体库（`fontlib/`）与面板字母的字体

**契约一句话**：字体的**唯一来源**是 `tools/svg_grid/fontlib/`；svg_grid **永不**读宿主机的系统字体库。

## 为什么（已发生的问题）

渲染 PNG/AVIF 时原来用 `load_system_fonts()`：同一份 SVG 资产换台机器、或换到 CI，字形与换行就可能不同，
而且没有任何记录说用了哪个字体。另外族名写错是**静默回退**——实测一个不存在的族名与 `serif`
**逐像素完全一致**（都退到渲染库内置默认），stderr 一声不响，成品只表现为"字宽有点不一样"。

## 契约（现行）

| 项 | 口径 |
|---|---|
| 字体来源 | **只有** `fontlib/`。禁止 `load_system_fonts()`（代码里不得再出现） |
| 目录解析 | **运行时**解析，顺序：`--font-dir <dir>` > 环境变量 `SVG_GRID_FONT_DIR` > 数据根 `$HOME/svg_grid/fontlib`（契约见 `cli.md`）。**不许**把构建机路径烧进二进制（代码里不得出现 `env!("CARGO_MANIFEST_DIR")`，R14 已删） |
| 默认族 | **`Arial`**：① 面板字母的 `--label-fontfamily` 默认值；② 渲染库的"无声明族默认族"也设为 Arial（**不**留它自己的内置默认） |
| 可调 | `--label-fontfamily` 接受 CSS 列表 ⇒ **回退链**（`"Arial", "Liberation Sans", sans-serif`），第一个可用者生效 |
| 可查 | `--list-fonts`：按字母序列出 `fontlib/` 里实际可用的 family（去重），不组合图 |
| 缺族 | **W8**（非致命）：请求的族里第一个**非 generic** 名字不在字体库里 ⇒ stderr 出声（会回退到 X、字形宽度/换行会变）+ 修法。**正常路径零输出** |
| 库为空/缺失 | **fail-closed**：这是配置错误（不是输入畸形）——否则文字整体不渲染而退出码为 0。报文必须说清"往哪个目录放字体、或跑 `fontlib/setup.sh`" |
| 内容文字 | **不动**：图里原有的 `font-family` 原样搬运；本次只涉及工具自己画的字母（`--labels`） |
| 交付 | SVG 里照旧**写字体名**（矢量，交给读者环境解析）；PNG/AVIF 由 `fontlib/` 决定 ⇒ 同资产跨机器一致 |

## `fontlib/` 的成员

```
fontlib/
├── README.md     # 往里放什么、怎么放、授权上能不能入库（入库即分发）
├── setup.sh      # 从本机把规定的几个族复制进来（幂等；打印复制了什么）
└── *.ttf/*.otf/*.ttc   # 字体文件本身 —— **不入库**（.gitignore 排除）
```

- **字体文件不入库**：Arial / Times New Roman 是微软字体，随公开仓库分发属违规。换机器时跑 `setup.sh` 补齐。
- **想要的族**（`setup.sh` 的清单）：Arial、Times New Roman，各 Regular / Bold / Italic / Bold Italic。

## 明确不做 / 待议

- **族名别名表**（如 `Arial → LiberationSans-*.ttf`）：不加。真要让仓库**自带**可分发字体（SIL OFL 的
  Liberation 系与 Arial/TNR **度量兼容**），再一起加别名，否则 `fontlib/` 里只有 Liberation、请求 `Arial`
  仍会落空。**待议**。
- **字体嵌入 / 文本转轮廓**：不做（会牺牲可编辑性与体积）。若投稿硬要求"逐字一致"，再单独议。
- 系统字体的"兜底读取"：**不做**——那正好是本次要消灭的不确定性来源。

## 谁来管

- 代码位置：`src/raster.rs`（字体库装载与默认族）、`src/main.rs`（`--label-fontfamily` 默认值、`--font-dir`、
  `--list-fonts`）、`src/warnings/`（W8）、`fontlib/`。
- 文案：`docs/general-figure-guide/references/{01_canvas_layout,02a_svg_grid_assembly}.md`（**规范源**，
  改完 `rsync` 到 `tools/manuscripts_skills/general-figure-guide/`，`diff -r` 必须为空）。
- 证据：`docs/PITFALLS/round-data/R13/`（默认 Arial、W8 出声、不读系统字体、空库 fail-closed）。
