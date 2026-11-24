# svg_grid 阻塞点 — W7“值被原样保留”在非 `--verbose` 下完全静默

场景：svg_grid 0.1.0，HEAD `b6f6741`（HARNESS `552eeef`），已 `./install.sh`；
环境 `D1_FONT_DIR` 不参与本现象（不渲染，只看 stdout/stderr/退出码/产物）。

## 现象

W7（`… could not be rewritten, kept verbatim`）表示“这个数字没跟着几何缩放、
被原样留在产物里”。但 `src/warnings/mod.rs::emit(verbose)` 在未给 `--verbose`
时整段 `return`，于是 W7 与汇总行一起消失：`rc=0`、`stderr` 0 字节、
`convert --check` 仍报 compatible，组图产物里的数字与几何不一致却没有任何输出。

实测（本机 `b6f6741` 装机版）：

```
$ printf '%s\n' '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">' \
  '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>' \
  '<rect x="10" y="10" width="30" height="20" stroke-width="1e999"/></svg>' > /tmp/w7.svg
$ svg_grid convert --input /tmp/w7.svg --output /tmp/w7.out.svg --check; echo rc=$?
compatible: /tmp/w7.svg; rc=0
$ svg_grid plot-grid --input /tmp/w7.svg --output /tmp/w7.png --width 600 --ncol 1 2>/tmp/w7.err; echo rc=$?
rc=0
$ wc -c < /tmp/w7.err
0
$ svg_grid plot-grid --input /tmp/w7.svg --output /tmp/w7b.png --width 600 --ncol 1 --verbose 2>&1 | grep 'kept verbatim'
svg_grid: warning: 1 attribute could not be rewritten, kept verbatim: input #0 <rect stroke-width>: non-finite number "1e999". Fix: correct the value at the plotting end.
```

同一现象也出现在 `<pattern patternTransform="wobble(1)">`、
`<feGaussianBlur stdDeviation="2 3 4">`、`style="stroke-width:3e38"` 等输入上。

量化：`b6f6741` 上 214 个 `tests/**/*.svg` 跑 `convert --verbose` 得到 W7 0 条；
58 个 `tests/fixtures/**/*.svg` 跑 `plot-grid --verbose` 也得到 W7 0 条 ——
正常语料不依赖 W7，但“被保原文”的输入在默认路径上完全无声。

## 复现步骤

1. 用上面的 `printf` 生成 `/tmp/w7.svg`（或任一含 `stroke-width="1e999"`、
   `stdDeviation="2 3 4"`、`patternTransform="wobble(1)"` 的 panel）。
2. `svg_grid convert --input /tmp/w7.svg --output /tmp/w7.out.svg --check`：
   `rc=0`，输出 `compatible: …`。
3. `svg_grid plot-grid --input /tmp/w7.svg --output /tmp/w7.png --width 600 --ncol 1 2>/tmp/w7.err`：
   `rc=0`，`wc -c < /tmp/w7.err` 为 `0`。
4. 同一条命令加 `--verbose`：stderr 才出现
   `svg_grid: warning: 1 attribute could not be rewritten, kept verbatim: …`。
