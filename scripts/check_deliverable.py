#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""薄壳入口：成品图交付前的两条程序化检查（**唯一判据**）。

判据本身住在同目录的包 `deliverable_checks/`（`dead_band` / `outer_frame` /
`selftest` / `summary` / `cli`），这里只做 **argparse + 调用 + 退出码 + 把函数再导出**，
于是调用方的**路径与用法一个字都不用改**：

    ~/.venv/bin/python …/scripts/check_deliverable.py <成品.png> --margin N

或者

    sys.path.insert(0, "…/scripts")
    import check_deliverable as cd
    cd.check_dead_band(gray, margin=24, label_band=126, allow_gap=True)

两条检查的判据说明见包 `deliverable_checks/__init__.py`（与旧版逐字一致）。
"""

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

from deliverable_checks import (  # noqa: E402
    DEFAULT_MARGIN,
    DEFAULT_BAND,
    DEFAULT_INK_THRESHOLD,
    OUTER_FRAME,
    CHECK_NAMES,
    load_gray,
    parse_margin,
    check_dead_band,
    check_outer_frame,
    run_checks,
    format_result,
    selftest,
    build_parser,
    main,
)

# 再导出给 `import check_deliverable as cd` 的调用方（R4 的 test.py、两个 compose 脚本）。
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

if __name__ == "__main__":
    sys.exit(main())
