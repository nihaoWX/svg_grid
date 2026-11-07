# -*- coding: utf-8 -*-
"""检查 2：外框 2 px 墨迹 = 0（并报墨迹 bbox）。"""

import numpy as np

from .common import DEFAULT_INK_THRESHOLD, OUTER_FRAME, CHECK_NAMES, _ink


def check_outer_frame(gray, ink_threshold=DEFAULT_INK_THRESHOLD, frame=OUTER_FRAME):
    """检查 2：外框 2 px 圈内非背景像素数必须为 0；**同一条输出并报整图墨迹 bbox**。

    这是抓"图例/长标签被右缘/左缘静默裁掉"的**唯一程序化手段**——被裁内容的
    bbox 恰好贴着边缘，拿"内容包围盒是否在画布内"判它抓不到（那是恒真的死重量，
    已删）。故**「无裁切」由本条承担**；输入侧更早由 **W6**（`plot-grid` 的
    "输入内容越出它自己的画布"告警）抓。

    在**同一条** note 里并报整图非背景墨迹的包围盒（`bbox = [x_min, y_min, x_max, y_max]`），
    于是"有没有触边"与"内容落在哪"一次给齐。
    """
    H, W = gray.shape
    ink = _ink(gray, ink_threshold)
    ring = np.zeros((H, W), dtype=bool)
    f = int(frame)
    ring[:f, :] = True
    ring[H - f:, :] = True
    ring[:, :f] = True
    ring[:, W - f:] = True
    n = int(np.count_nonzero(ink & ring))

    # 墨迹 bbox（整图非背景墨迹的包围盒）——并入本条输出
    ys, xs = np.where(ink)
    if len(xs):
        bbox = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]
        bbox_txt = (f"墨迹 bbox x∈[{bbox[0]},{bbox[2]}] y∈[{bbox[1]},{bbox[3]}]"
                    f"（画布 {W}x{H}）")
    else:
        bbox = None
        bbox_txt = f"整图无非背景墨迹（画布 {W}x{H}）"

    rec = dict(key="outer_frame", name=CHECK_NAMES["outer_frame"],
               status="fail" if n else "pass",
               note=f"外框 {f}px 圈内非背景像素 = {n}；{bbox_txt}",
               count=n, frame=f, bbox=bbox, canvas=[W, H],
               deduct=dict(outer_frame=f))
    if n:
        ys2, xs2 = np.where(ink & ring)
        rec["reason"] = (f"外框 {f}px 圈内有 {n} 个非背景像素（内容触边/静默裁切）；"
                         f"样例坐标 {list(zip(xs2[:5].tolist(), ys2[:5].tolist()))}")
    return rec
