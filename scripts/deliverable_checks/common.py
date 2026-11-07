# -*- coding: utf-8 -*-
"""`check_deliverable` 的共享常量与基础工具（包内私有）。

判据本身在 `dead_band.py` / `outer_frame.py`；这里只放两条检查都要用到的
常量、图片读取与 margin 解析，避免它们各自复制一份。
"""

import argparse

import numpy as np
from PIL import Image

DEFAULT_MARGIN = 24
DEFAULT_BAND = 200
DEFAULT_INK_THRESHOLD = 245
OUTER_FRAME = 2

CHECK_NAMES = {
    "dead_band": "1. 死白带(扣 margin+字母带后只查完整窗口)",
    "outer_frame": "2. 外框 2px 墨迹=0(并报墨迹 bbox)",
}


def load_gray(path):
    """把任意图片读成 uint8 灰度数组（L 模式）。"""
    with Image.open(path) as img:
        return np.asarray(img.convert("L")).copy()


def parse_margin(value):
    """`N` / `T,R,B,L` 字符串，或长度 1/4 的序列 -> (top, right, bottom, left)。"""
    if isinstance(value, (list, tuple)):
        vals = [int(round(float(v))) for v in value]
    else:
        parts = [p for p in str(value).replace(",", " ").split() if p]
        try:
            vals = [int(round(float(p))) for p in parts]
        except ValueError as exc:  # pragma: no cover - argparse 会先拦
            raise argparse.ArgumentTypeError(f"--margin 不是数字: {value!r}") from exc
    if len(vals) == 1:
        return (vals[0], vals[0], vals[0], vals[0])
    if len(vals) == 4:
        return tuple(vals)
    raise argparse.ArgumentTypeError(
        f"--margin 需要 1 个或 4 个数字 (top,right,bottom,left)，收到 {value!r}")


def _ink(gray, ink_threshold):
    return gray < ink_threshold


def _background_runs(present):
    """present: bool 1-D（该 行/列 是否有墨）。返回两端连续 False 的长度 (lo, hi)。"""
    n = len(present)
    lo = 0
    while lo < n and not present[lo]:
        lo += 1
    hi = n
    while hi > lo and not present[hi - 1]:
        hi -= 1
    return lo, n - hi
