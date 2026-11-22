# convert 把 N≥3 子路径的填充 `<path>` 合成单个 `<polygon>`，字孔被填死

来源：`svg_grid` D1（2026-11-22）。输入 `pandamap_ALB_A.svg`（matplotlib 输出，14 个 `<text>` 全部转成了
曲线 `<path>`，每个字形 3–10 条子路径：外轮廓 + 字孔 + 笔画内孔）。

## 现象

`svg_grid convert` 的产物里，标签字形（如 `ASP 451`）的孔（`P`/`A`/`4`/`5`/`0` 的内孔）变成实心黑块：

- 把原图与转换产物里的同一个标签组 `<g id="text_4">` 抽出来，在同一 `viewBox`（`455 140 60 18`，
  600×180 px，白底）隔离渲染，数黑墨（亮度 <64）：
  **原图 17,575 px → 转换后 28,891 px（1.64×）**；子路径 ≥3 的填充路径全图 46 条（23 个标签 × 2 份）。
- 全图（4659×4760，亮度 <128 记墨）：原图 459,639 px → 转换后 478,334 px；与原图的墨迹掩码异或
  34,751 px，且**只增不减**（无一处变白）——即纯粹是「孔被填上」。
- 卷曲展平不是原因：容差 0.02 的展平偏差 ≤0.03 px @2×，且转换前重跑产物与原坏产物逐字节相同
  （`cmp=0`），故可排除渲染/展平漂移。

## 复现

```bash
# 1) 转换（原图见 research/白藜芦醇胰腺癌/steps/02_分子对接/pandamap_all/pandamap_ALB_A.svg）
svg_grid convert --input pandamap_ALB_A.svg --output /tmp/after.svg

# 2) 看结构：修复前 0 个 <path> / 216 个 <polygon>；其中 46 个 polygon 由 ≥3 条子路径拼成
grep -c '<path' /tmp/after.svg; grep -c '<polygon' /tmp/after.svg

# 3) 最小复现：一块板 + 两个孔，fill-rule 任意；把 d 交给转换器。
#    修复前产物是单个 <polygon>，两个孔被填实；中心点亮度与板相同。
```

最小输入（3 条子路径 = 外框 + 2 个孔）：

```svg
<path d="M 0 0 L 60 0 L 60 40 L 0 40 Z
         M 10 10 L 30 10 L 30 30 L 10 30 Z
         M 40 10 L 50 10 L 50 30 L 40 30 Z" fill="#123456"/>
```

## 根因

`convert/src/translate/shapes/path_form.rs` 的填充分支把该路径的**全部**子路径点串首尾相接，输出为
**一个** `<polygon>`（`join_closed_points`，`convert/src/translate/shapes/path_form.rs:join_closed_points`）。
`<polygon>` 的语义是「末点隐式连回整串首点」，于是第 i 条子路径的末点与第 i+1 条子路径的首点之间多了
一条 bridge，**最后一条子路径的末点到第一条子路径首点又补一条回程 bridge**：

- N=2 时两条 bridge 是同一线段的反向遍历，围出的面积恒为 0，绕数不翻转 ⇒ 老测试（外轮廓 + 内孔）全绿；
- N≥3 时回程 bridge 与去程 bridge 围出**真多边形**，其绕数按 `fill-rule` 翻转内部区域 ⇒ 与回程环重叠的
  字孔被填死。模块文档里「bridge 与回程同线段反向、互相抵消」的论证只对 N=2 成立
  （`convert/src/translate/shapes/mod.rs`）。

## 修法

- `fill_region_shape()`：`subpaths.len() >= 3` 时输出**展平的 `<path>`**（每条子路径一个
  `M/L…Z`，`joined_closed_path_data`），否则仍用原 `<polygon>`；`fill-rule` 由 `shape_skeleton` 原样带过，
  `nonzero`/`evenodd` 语义不变。
- 填充+描边分支：描边仍按子路径分别输出（不变），只有填充部分改走 `fill_region_shape`（多子路径时是一条
  多轮廓 `<path>`），并显式 `stroke="none"`。
- `convert/src/self_check.rs`：残渣检查原先拒绝任何 `<path>`，现只放行「无 `transform`、命令集 ⊆ {M,L}、
  至少一个 Z」的展平填充路径；不带 Z 的 `<path>` 残渣仍报错。
- 修正 `convert/src/translate/shapes/mod.rs` 的错误论证；`convert/src/translate/mod.rs`、`convert/src/lib.rs`
  的注释同步为「≥3 条子路径 ⇒ 展平多轮廓 `<path>`」。
- 回归测试（`convert/src/translate/shapes/tests.rs`）：`fill_only_three_subpaths_become_one_flattened_path_not_a_polygon`、
  `two_donuts_with_evenodd_keep_their_holes_as_one_path`、`fill_and_stroke_three_subpaths_split_fill_path_from_strokes`、
  `two_subpaths_still_take_the_single_polygon_path`；`convert/src/self_check.rs` 增
  `self_check_accepts_the_flattened_multi_contour_fill_path`。

## 修后实测

同一隔离渲染（`<g id="text_4">`，600×180，白底）：黑墨 **17,575 → 17,552 px**（原图 17,575），与修复前的
28,891 相比回落 99.6%；像素差 361 px（占 108,000 的 0.33%，全部为抗锯齿级），新增黑墨 5 px、丢失黑墨 26 px。
全图（4659×4760）：亮度 <128 墨量 478,334 → **459,633**（原图 459,639），与原图异或 34,751 → **168** px。

未改曲线展平容差（`DEFAULT_TOLERANCE = 0.02`）。
