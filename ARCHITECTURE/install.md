# 安装（`install.sh`）：编译、装二进制、**装 skill**、装语料与字体

**一句话**：开发/使用这台机器上只需要一条 `tools/svg_grid/install.sh`——它 release 编译、把唯一的那个二进制 `svg_grid`
装进 `~/.local/bin`、把**随工具发布的 skill**（`general-figure-guide`，只含使用侧 `SKILL.md` + `references/`）
拷进 harness 的 skill 目录、把语料与预设字体装进数据根 `~/svg_grid/`；**任何代码或 skill 改动之后都要再跑一次**。

## 为什么需要它

- 之前根本没有"装上"这一步：二进制只在 `target/debug/`（被 gitignore），文档里的 `svg_grid …` 命令
  **在 PATH 里不存在**（shell 里跑直接 command not found）。
- 字体目录自 R14 起是运行时解析（`--font-dir` > `SVG_GRID_FONT_DIR` > `$HOME/svg_grid/fontlib`，
  见 `cli.md`），所以安装必须顺带把字体装到**那个**目录，否则"装好了却渲染不出字"。
- **skill 不装 = 工具不存在**：agent 是靠 harness 的 skill 目录发现 `general-figure-guide` 的；
  只装二进制，组图这条工作流在别的会话里根本不会被触发（同 `agent-tool-design/references/tool-install.md`）。
  因此 guide 的规范源（`docs/general-figure-guide/`）一改，**也要重跑一次**。

## 契约

```
install.sh [--prefix <dir>] [--project <dir>] [--no-skill] [--no-fixtures]
           [--no-fonts] [--no-build] [--dry-run] [--check] [--help]
```

| 步骤 | 做什么 | 判据 |
|---|---|---|
| 1. 编译 | `cargo build --release --offline`（主 crate；`convert/` 作为**库依赖**一起编进同一个二进制，不再单独装） | `target/release/svg_grid` 存在 |
| 2. 装二进制 | 复制 **`svg_grid` 一个**二进制到 `<prefix>`（默认 `~/.local/bin`），`chmod +x`，**同名覆盖** | `command -v svg_grid` 有输出 |
| 3. 装 skill | **白名单拷贝** `SKILL.md` + `references/` → `~/.commandcode/skills/general-figure-guide/`（`--project <dir>` ⇒ `<dir>/.commandcode/skills/…`）。暂存目录 → `mv` 原子替换；目录名取 `SKILL.md` 的 `name:` | `diff -r` 与源逐字节一致；装好的目录里**没有** `ARCHITECTURE*/`、`src/`、`docs/`、`PROGRESS.md` |
| 4. 装语料 | `rsync -a --delete tools/svg_grid/tests/fixtures/ ~/svg_grid/fixtures/`（`doctor` 默认读这个数据根，见 `testing.md` §语料目录怎么解析） | 数据根里有 58 个 `.svg` + `MANIFEST.toml` |
| 5. 装字体 | `fontlib/setup.sh --dir ~/svg_grid/fontlib`（从**本机**复制 Arial / Times New Roman；不下载、不入库——微软字体不得随仓库分发） | 数据根里有 8 个 face |
| 6. 报告 | 版本、二进制装到哪、**skill 装到哪（新装/已是最新）**、语料几条、字体几个、`<prefix>` 是否在 `PATH` 里 | 输出**全英**、简洁、成功路径一句话一条 |

- **幂等**：重复跑不报错、不重复下载；`rsync -a --delete` 只搬差异；字体 `already up-to-date` 就跳过；
  skill 用 `diff -r` 比，一致则什么都不动（报告 `up to date`）。
- **拷贝、不用软链**：外部使用者 clone 下来没有开发树，软链必断。
- **可测**：`--dry-run` 只打印将要做的事、**不写任何文件**（R15 的断言用它）；
  `--no-build` / `--no-fonts` / `--no-fixtures` / `--no-skill` 各自只跳过那一步。
- **`--check`**：只读自检（不编译、不写），逐项报告，任一项不满足 `exit 2` 并给 `fix:`——
  ① 装着的二进制 == 最近一次 release 产物（sha256）；② skill 与源逐字节一致且**没有多出**开发侧文件；
  ③ 目标目录确实是 harness 的识别位置（`<root>/.commandcode/skills/<name>`）。
- **不许** `sudo`、不许写仓库外的别的地方（除 `<prefix>`、`~/.commandcode/skills/` 与 `~/svg_grid/`）。
- 失败即非零退出并说清哪一步失败（英文一句话 + `Fix:`）。

## 每次开发结束后

**改完代码或改完 guide → `install.sh` → 才去用装着的 `svg_grid` / 装着的 skill。**
装的是副本，不重装就会"改了没生效"：二进制看 `--version` 与 sha256，skill 看 `--check` 的 `diff -r`。

## 与 skill 的关系

使用侧 agent 只读装好的 `SKILL.md` + `references/`，**不读**这份文档（也不读 `ARCHITECTURE*/`、`PROGRESS.md`）。
本文件只规定"装什么、装到哪、怎么验"。
