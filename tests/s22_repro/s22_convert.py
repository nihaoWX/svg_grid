#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""`compose_s22` 第 1 节：把 sized 面板转成 tag 协议（`svg_grid_convert`）。"""

import os
import re
import subprocess

from s22_common import ALL_PANELS, CONVERT, PANELS_SIZED, TAGGED


def convert_panels():
    os.makedirs(TAGGED, exist_ok=True)
    rows = []
    for panel in ALL_PANELS:
        src = os.path.join(PANELS_SIZED, panel + ".svg")
        dst = os.path.join(TAGGED, panel + ".svg")
        result = subprocess.run(
            [CONVERT, "--input", src, "--output", dst, "--report"],
            capture_output=True, text=True,
        )
        svg = open(dst, encoding="utf-8").read() if os.path.exists(dst) else ""
        rows.append(dict(
            panel=panel,
            exit=result.returncode,
            path=svg.count("<path"),
            use=svg.count("<use"),
            gtransform=len(re.findall(r"<g\b[^>]*\btransform=", svg)),
            imgtf=len(re.findall(r"<image\b[^>]*\btransform=", svg)),
            text=svg.count("<text"),
            polyline=svg.count("<polyline"),
            polygon=svg.count("<polygon"),
            image=svg.count("<image"),
        ))
    return rows
