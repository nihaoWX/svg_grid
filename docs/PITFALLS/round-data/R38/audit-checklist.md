# R38 审计清单（旧轮证据的处置依据）

本文件是 R38 对**旧轮账本条目**做「修正 / 作废」时所引用的证据（`modify` / `retire` 的 `--evidence`）。
每一条都给了「原记录」与「实测」，实测命令可在本仓任意提交上复跑。

## 1. R37 的 5 处行号（原记录 vs 实测）

用 `git show <rev>:<path> | nl -ba` 逐个数函数体（含签名行与闭合 `}`）：
`code_before = e39b3e9`、`code_after = 18102d8`。

| finding | 位置 | 原记录 | 实测 | 结论 |
|---|---|---|---|---|
| R37-F3 | `convert/src/translate/bake.rs:bake_element` | `lines_before=37-133` | **37-121**（e39b3e9：`fn` 在 37、`Ok(())` 120、`}` 121） | 原记录把 `lines_after` 的 133 抄到了 `lines_before`；`lines_after=37-133` 正确 |
| R37-F1 | `convert/src/translate/uses.rs:prune_defs` | `lines_after=192-240` | **192-215**（`}` 在 215；217 起是 `prune_defs_with`，240 是它的 `}`） | 原记录把相邻函数并进来了 |
| R37-F2 | `convert/src/translate/uses.rs:check_dangling_references` | `lines_after=356-405` | **356-371**（`}` 在 371；373 `collect_defined_ids`、384-405 `walk_references`） | 原记录把两个 helper 并进来了 |
| R37-F4 | `convert/src/self_check.rs:check_residue` | `lines_before=49-110` | **66-99**（e39b3e9：`fn check_residue(…, in_mask_or_marker: bool, …)` 在 66、`}` 99；49-65 是文档注释） | 原记录从注释开头算起 |
| R37-F5 | `src/transform/mod.rs:transform_element` | `lines_before=56-92` | **56-91**（e39b3e9：`fn` 56、`Ok(())` 90、`}` 91） | 差 1（把函数后的空行算进去了） |

## 2. R37 `meta.toml` note 里被证伪的两句

1. 原句：「pattern 相位错位是 R16 时代遗留——resvg 把 pattern 内容当 tile 局部坐标」+「本轮没有改变任何
   pattern 渲染，故不在本轮修」。
   实测（R38）：`/tmp/r38probe/cA.svg`（tile x=24,y=24,w=40,h=40 + body 14,6,12,8）在 **resvg 0.47 与
   cairosvg 2.9.1 下 ink 都是 576、bbox 都是 (38,30,129,77)**；`cB.svg`（tile 0,0 + `patternTransform=
   "translate(24,24)"`）与 cA **逐像素相同** ⇒ body 是 tile 局部坐标是两条渲染器共同语义，不是 resvg 特有；
   相位错位是 **composer 缺陷**。
2. 原句：「marker 两种单位都不重写也正确」。
   实测（R38）：`markerUnits="userSpaceOnUse"` 在 s=2 下与正确重写的产物 **XOR=624**（不是 0）⇒
   「两种单位都不重写」不成立；`markerWidth/Height=3` + `strokeWidth` 单位仍会随 stroke-width 重写而缩放，
   但 `userSpaceOnUse` 的 body 坐标需要额外换算。

## 3. R18：`install.sh --dry-run` 的「只有一行」

- 原断言：`dry.stdout.count("would install") == 1`。
- 实测：**2 行** —— `would install svg_grid -> …` 与 `would install skill general-figure-guide -> …`。
- 归因：`4133c83`（2026_11_07，install.sh 首次加入 skill 安装）之后该断言即失效；**在 base `756dfb2`
  上实测同样 FAIL**（`/tmp/r38base` 里 `./install.sh --dry-run | grep -c "dry-run: would install"` ⇒ 2），
  与本轮改动无关。R38 改成「一个二进制 + 一个 skill」的精确计数。

## 4. R19：作废（deprecated）

- 原断言 ①：「`tests/` 下只有 `selftest/` + 7 个历史资产目录」——后续轮次新增目录后不成立。
- 原断言 ②：用例总数写死 `BASELINE=190`、`COUNTS={lib:60, main:57, selftest:73}`。
  **base `756dfb2` 实测就是 89 lib + 59 main + 104 selftest**（R38 后 89 + 59 + 108）⇒ 从 base 起不可能成立。
- 原断言 ④：「独立二进制仍是 `svg_grid_convert:`（usage 行同）」——该二进制已合并进主二进制，
  `install.sh` 只装一个；base 上 `convert/target/debug/svg_grid_convert` 已不存在，脚本在①的前置即 rc=1。
- 结论：属「目标功能被删」⇒ `retire --kind deprecated`。R20 的回归列表同步摘掉 R19（`modify R20`），
  R4–R17 由顶层逐个跑覆盖。

## 5. R24：④ 的「未转换的原始 fixture ⇒ rc=2」

- 原断言：`plot-grid --input tests/fixtures/ggplot2/bars_stacked.svg` ⇒ rc=2 + `invalid input`。
- 实测：**rc=0**（`plot-grid` 现在会自动转换未转换的原始 fixture；该能力落于 R24 之后）。
  **base `756dfb2` 上同一命令同样 rc=0**（已实测），非本轮引入。
- R38 改用永久无效输入（`<rect x="abc" …>` 非数值几何 ⇒ convert 拒绝）⇒ 仍 `invalid input` rc=2，
  保住「退出码分档」的原意。

## 6. 复跑记录

- R3–R37 逐个顶层复跑（嵌套回归用 `/tmp/shim` 表驱动短路，只对**已在顶层跑绿**的轮次放行）。
- R37 的 `test.py` 在 R38 里被改：⑥ 段从「编码错误相位」改成「body 不平移」的回归判据
  （否则 R37 会为 R38 修好的缺陷报红），并加 `SVG_GRID_*_BIN` 覆盖与渲染器缺失 fail-closed。
