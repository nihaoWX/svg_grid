# convert：D1 评审遗留三项（`<path>` 白名单 provenance / 拆分填充的 stroke paint / `--report` 计数）

- 日期：2026-11-22
- 范围：D1（`convert` 把 N≥3 子路径的填充合并成单 `<polygon>`、字孔被填死）的评审遗留项，来自本轮 D1 评审
- 前序：`1e05ce1`（D1 修法 + 首轮报告）、`3a9747a`（首轮报告入账 R27 后删除）

## 现象

1. **provenance 漏洞**：`convert/src/self_check.rs:is_flattened_fill_path` 只按 `d` 的**字面形状**放行
   （`M`/`L`/`Z`、无 `transform`）。评审复现：`<mask id="m"><path d="M 0 0 L 10 0 L 10 10 Z"/></mask>`
   在 D1 白名单之前 rc=1（判残渣），白名单之后 rc=0 —— 未翻译的 mask/marker 内容被当成"转换产物"放过了。
2. **拆分分支残留描边属性**：`convert/src/translate/shapes/path_form.rs` 填充+描边多子路径分支里，
   填充形状只删了 `stroke-width`，仍留着 `stroke-dasharray` / `stroke-linecap` / `stroke-linejoin` /
   `stroke-opacity`。`stroke="none"` 下无视觉影响，但"填充半"与"描边半"的划分不干净。
3. **`--report` 漏计 `<path>`**：`convert/src/tag.rs:walk_count` 的元素名单里没有 `path`。
   一个 1 个展平 `<path>` + 3 个 `<polygon>` 的产物，`convert --report` 打出 `primitives: rect=1, polygon=3`。

## 复现（改前）

```bash
cd tools/svg_grid && ./target/release/svg_grid convert --input mask_root.svg --output /tmp/out.svg
# 1) 改前（含 D1 白名单）：rc=0，mask 内容被当成产物
# 2) 改前（D1 之前）：rc=1，post-conversion self-check failed: … a <path>
```

`mask_root.svg`：`<mask id='m'><path d='M 0,0 L 10,0 L 10,10 Z'/></mask>` + 一个 `mask="url(#m)"` 的 `<rect>`。

## 根因

1. 白名单把"形状像展平填充路径"当成"是 shapes 阶段写的"。但 shapes 阶段只改写它遍历到的几何；mask/marker
   内容不翻译。**单看几何本身无法给出 provenance**——尤其不能按容器一刀切：matplotlib 的 hatch 填充
   `<path>` 位于 `<defs><pattern>` 内、由 shapes 原地改写（`matplotlib/bars_grouped`），把 defs 整类排除会
   把这个合法产物也判成残渣（实测 doctor 56 pass / 1 fail）。
2. 拆分分支只删 `stroke-width` 是历史补丁，未把"整组 stroke paint"当成一个整体。
3. `walk_count` 的名单是手写的，D1 新增 `<path>` 产物时漏了同步。

## 修法

1. `convert/src/self_check.rs`
   - `check_residue(element, in_mask_or_marker, problems)`：进入 `<mask>` / `<marker>` 时置位，
     其中的 `<path>` 一律判残渣（shapes 阶段不翻译这里）。
   - `is_flattened_fill_path` 从"只扫字母命令"收紧为**接受 `joined_closed_path_data` 的确切序列化**：
     `M x,y L x,y … Z` 轮廓、轮廓间单个空格、坐标为 `fmt_num` 形式（整数或三位小数）、无其它空白。
     手写 `d` 恰好符合该文法时它本身已经是"直线展平的填充区域"，放行无害；其余（空格/换行、相对命令、
     曲线、缺 `Z`）仍判残渣。边界写进了函数注释。
2. `convert/src/translate/shapes/path_form.rs`：新增 `drop_stroke_paint(&mut shape)` —— 删掉 `stroke` 与
   全部 `stroke-*` 属性、同样过滤内联 `style` 中的声明，最后强制 `stroke="none"`；拆分分支的填充半改为
   `fill_region_shape(...); drop_stroke_paint(&mut fill_shape);`（不再只删 `stroke-width`）。
3. `convert/src/tag.rs`：`walk_count` 名单加入 `"path"`。

## 验证

- `mask` 复现（含逗号坐标的同形 `d`）：改后 rc=1，`post-conversion self-check failed: … a <path>`，无产物。
- `matplotlib/bars_grouped`：只排除 mask/marker（保留 pattern 产物的放行）后通过。
- 拆分产物：`<path fill="#ff0000" fill-rule="evenodd" d="…" stroke="none"/>`，无任何 `stroke-*` 属性/style。
- `convert --report`：`primitives:  rect=1, path=1`。
- 测试：根 crate `cargo test` 74+59+74 = 207 passed；`convert` 88 lib + 12 CLI = 100 passed（新增
  `self_check_rejects_flattened_paths_inside_masks_and_markers`、`primitive_report_counts_flattened_paths`，
  加强 `fill_and_stroke_three_subpaths_split_fill_path_from_strokes`）。
- `cargo fmt --check` 两 crate 干净；`cargo clippy --all-targets -- -D warnings` 两 crate rc=0；
  `./install.sh` 成功；`svg_grid doctor` = **57 pass / 0 fail / 1 skip**。
- 隔离渲染复核（cairosvg 2.9.1，白底，展平容差未动）：整图 4659×4760、亮度 <128 的黑墨
  原图 473,398 px → 修复前 492,050（1.039×，多出 26,892）→ 修复后 473,359（多 125 / 少 164，MAE 0.0052）；
  `pandamap_ALB_A.svg` 的 `ASP 451` 标签组（viewBox `455 140 60 18`、600×180、亮度 <64）：
  17,575 → 28,564（1.63×）→ 17,552。
- 证据：`docs/PITFALLS/round-data/R28/`（`test.py` 3 项全 PASS，秒级，黑盒 CLI）。
