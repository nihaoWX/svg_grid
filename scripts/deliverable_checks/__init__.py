# -*- coding: utf-8 -*-
"""成品图交付前的两条程序化检查（**唯一判据**）。

这是 `general-figure-guide` 里 `05_checklist.md` C 节（以及 `tests/*/compose_*.py`
里那份会误报的副本）的**唯一实现**——判据只写在这里，别处一律"跑这个脚本"。

入口是薄壳 `scripts/check_deliverable.py`（argparse + 调用 + 退出码 + 再导出本包）。

用法
----
    check_deliverable.py <成品.png> [--margin 24] [--gap] [--label-band N]
                         [--band 200] [--ink-threshold 245] [--json] [--selftest]

* **退出码即结论**：`0` = 两条全过；`1` = 有失败。（用法错误由 argparse 以 `2` 退出。）
* **每条检查一行**；**只有失败才给细节**；**跳过（skip）一定打印并给出原因**。
* 每条检查都打印**它的判定依据**（检查 1：扣掉的 margin / 字母带 / 页边距豁免 /
  末尾余量；检查 2（外框）：圈内像素数 + 墨迹 bbox），判据因此可解释。
* 只依赖 `numpy` + `Pillow`；**离线、不写仓库**。
* 判据写成函数（`check_dead_band` / `check_outer_frame` / `run_checks`），
  可直接 `import` 复用（`round-data/R4` 就用它）。

两条检查
--------
1. **死白带（修正后的判据）**：先扣掉四边 `--margin` 与顶部 `--label-band`（字母带），
   再把区域两端**紧贴扣减边界、完全无墨**的连续行/列按"页边距 / 面板自带留白"豁免
   （这是 `05_checklist` 既有的"统计时先扣掉面板边距"豁免，**会打印豁免了多少 px**），
   然后在剩余核心内**只查完整的 `--band` 窗口**（行带与列带都查）；
   某个完整窗口的墨迹占比 == 0 即判**死带**。末尾不足一个窗口的余量**另判**（豁免）。
   - 修正 ①**通栏面板的右页边距 > `--band`**：它落在"页边距豁免"里 ⇒ 不再误报；
   - 修正 ②**行间隔里坐着字母**：`--label-band` 把字母带整体扣掉 ⇒ 不再被当成"间隔"。
   - `--gap`：额外豁免**内部**整块无墨带（多名/多列图里正常的行/列间隔；
     单行单列图不必给，从而仍能抓"某列整块面板缺失"）。
2. **外框 2 px 墨迹 = 0（并报墨迹 bbox）**：图像最外 2 px 圈内的非背景像素数必须为 0
   （这是抓"图例/长标签被右缘/左缘静默裁掉"的**唯一程序化手段**，不能省）；
   **同一条输出里并报整图墨迹 bbox**（"内容落在哪"），一条检查即同时给出
   "有没有触边"与"内容落在哪"。**「无裁切」由本条承担**——位图输入的墨迹 bbox 必然
   落在画布内（对位图恒真），故**不再单列**"内容未触边 / 画布内 bbox"那条检查；
   输入侧更早由 **W6**（`plot-grid` 的"输入内容越界"告警）抓。
"""

from .common import (
    DEFAULT_MARGIN,
    DEFAULT_BAND,
    DEFAULT_INK_THRESHOLD,
    OUTER_FRAME,
    CHECK_NAMES,
    load_gray,
    parse_margin,
)
from .dead_band import check_dead_band
from .outer_frame import check_outer_frame
from .summary import run_checks, format_result
from .selftest import selftest
from .cli import build_parser, main

__all__ = [
    "DEFAULT_MARGIN",
    "DEFAULT_BAND",
    "DEFAULT_INK_THRESHOLD",
    "OUTER_FRAME",
    "CHECK_NAMES",
    "load_gray",
    "parse_margin",
    "check_dead_band",
    "check_outer_frame",
    "run_checks",
    "format_result",
    "selftest",
    "build_parser",
    "main",
]
