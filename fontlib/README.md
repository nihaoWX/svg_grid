# `fontlib/` —— svg_grid 唯一的字体来源

渲染 PNG / AVIF 时（面板字母、面板内文字）用到的字体**只**从**字体库目录**读；svg_grid **不读**
宿主机的系统字体库。这样同一份 SVG 资产在哪台机器上出图都一致，并且"用了哪个字体"是有据可查的一件事实。

## 字体库目录在哪

运行时按顺序解析（前一个生效即止）：

1. `--font-dir <dir>`（显式）
2. 环境变量 `SVG_GRID_FONT_DIR`
3. **数据根** `$HOME/svg_grid/fontlib`

`--font-dir` 试完不再探测别处。**没有编译期路径**：二进制里不含构建机的 checkout 位置，
所以装到别处的同一份二进制读的仍是 `$HOME` 下的数据根。

本目录（仓库里的 `tools/svg_grid/fontlib/`）是**脚本与说明的家**：只有 `README.md` 与 `setup.sh`，
字体文件不入库（见下）。

## 往字体库里放什么

- 直接放字体文件：`*.ttf` / `*.otf` / `*.ttc`（可以带子目录，`fontdb` 会递归扫描）。
- 族名以字体文件自己的 name 表为准（`Arial_Bold_Italic.ttf` 的族名仍是 `Arial`）。
- 放进去之后用 `svg_grid plot-grid --list-fonts` 看这台机器现在**实际能用**的族名（去重排序）；
  写 `--label-fontfamily "想要的族, 下一个, sans-serif"` 即回退链（第一个可用者生效）。

## 怎么放（换机器时一条命令）

```bash
fontlib/setup.sh                  # 幂等：把本机的 Arial / Times New Roman（Regular/Bold/Italic/Bold Italic）
                                  # 复制进数据根 $HOME/svg_grid/fontlib
fontlib/setup.sh --dir <目标>      # 复制进另一个字体库（比如 --font-dir 指向的那个）
fontlib/setup.sh --dry-run        # 只看会复制什么，不落盘
```

`setup.sh` 用 `fc-match` 定位本机字体，因此需要 fontconfig（`fc-match`）。它打印每一个 face 的结果
（`copied` / `up-to-date` / `missing`），最后给一个汇总；**重复跑不会重复复制**（逐字节相同就跳过）。

想要的族（`setup.sh` 的清单）：**Arial**、**Times New Roman**，各 Regular / Bold / Italic / Bold Italic。

## 授权：字体文件**不入库**

Arial / Times New Roman 是微软字体，随公开仓库分发属违规，所以本目录的 `*.ttf|*.otf|*.ttc` 被
`tools/svg_grid/.gitignore` 排除；入库的只有本文件与 `setup.sh`。换机器 / 新 checkout 后跑一次
`setup.sh` 补齐即可。

## 空库是硬错误（配置错，退出码 3）

字体库缺失或一个字体都没有、而这次要出 PNG/AVIF 或要加面板字母 ⇒ svg_grid **退出码 3** 并说明
"往哪个目录放字体、或跑 `fontlib/setup.sh`"，同时把**实际用的那个目录**打出来。纯矢量、不加字母的
运行不需要字体，不拦。
