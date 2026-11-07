# -*- coding: utf-8 -*-
"""检查 1：死白带（扣 margin + 字母带后只查完整窗口）。"""

from .common import (
    DEFAULT_MARGIN,
    DEFAULT_BAND,
    DEFAULT_INK_THRESHOLD,
    CHECK_NAMES,
    parse_margin,
    _ink,
    _background_runs,
)


def check_dead_band(gray, margin=DEFAULT_MARGIN, band=DEFAULT_BAND,
                    ink_threshold=DEFAULT_INK_THRESHOLD, label_band=0,
                    allow_gap=False):
    """修正后的死白带判据。返回一个结果 dict（status: pass/fail/skip）。"""
    mt, mr, mb, ml = parse_margin(margin)
    band = int(band)
    label_band = int(label_band)
    H, W = gray.shape
    top = mt + label_band
    y0, y1 = top, H - mb
    x0, x1 = ml, W - mr

    deduct = ("扣除: 四边 margin=(t{mt},r{mr},b{mb},l{ml})px"
              " + 顶部字母带 {lb}px".format(mt=mt, mr=mr, mb=mb, ml=ml, lb=label_band))

    if y1 - y0 < band or x1 - x0 < band:
        return dict(key="dead_band", name=CHECK_NAMES["dead_band"], status="skip",
                    note=deduct,
                    reason=(f"扣除 margin/字母带后区域 {max(0, x1 - x0)}x{max(0, y1 - y0)} "
                            f"不足一个 {band}px 完整窗口，死白带检查跳过"),
                    deduct=dict(margin=(mt, mr, mb, ml), label_band=label_band))

    region = _ink(gray, ink_threshold)[y0:y1, x0:x1]
    row_present = region.any(axis=1)
    col_present = region.any(axis=0)

    # (a) 页边距 / 面板自带留白：扣掉区域两端紧贴边、完全无墨的连续行/列
    r_lo, r_hi = _background_runs(row_present)
    c_lo, c_hi = _background_runs(col_present)
    edge = dict(top=int(r_lo), bottom=int(r_hi), left=int(c_lo), right=int(c_hi))
    if r_hi + r_lo >= len(row_present) or c_hi + c_lo >= len(col_present):
        return dict(key="dead_band", name=CHECK_NAMES["dead_band"], status="skip",
                    note=deduct + "；页边距豁免后无剩余核心（整块区域无墨）",
                    reason="扣除 margin/字母带并豁免页边距后，区域整块无墨，死白带检查跳过",
                    deduct=dict(margin=(mt, mr, mb, ml), label_band=label_band, edge=edge))

    core_rows = row_present[r_lo:len(row_present) - r_hi]
    core_cols = col_present[c_lo:len(col_present) - c_hi]

    def _windows(present):
        n = len(present)
        nwin = n // band
        remainder = n - nwin * band
        dead = [k for k in range(nwin) if not present[k * band:(k + 1) * band].any()]
        return nwin, remainder, dead, n

    r_nwin, r_rem, r_dead, r_core = _windows(core_rows)
    c_nwin, c_rem, c_dead, c_core = _windows(core_cols)

    note = (deduct
            + f"；页边距豁免(t{edge['top']}/b{edge['bottom']}/l{edge['left']}/r{edge['right']}px)"
            + f"；核心 {r_core}x{c_core}px 完整窗口 {c_nwin}列x{r_nwin}行"
            + f"；末尾余量 行{r_rem}/列{c_rem}px(豁免)"
            + ("；--gap 豁免内部间隔" if allow_gap else ""))

    # (b) 完整窗口：某窗口整块无墨即死带
    dead_rows = [(r_lo + k * band, r_lo + (k + 1) * band) for k in r_dead]
    dead_cols = [(c_lo + k * band, c_lo + (k + 1) * band) for k in c_dead]
    if allow_gap:
        # 内部间隔 = 核心里的整块无墨带；核心两端已扣过页边距 ⇒ 这些都属"内部"
        gap_rows, gap_cols = dead_rows, dead_cols
        dead_rows, dead_cols = [], []
    else:
        gap_rows = gap_cols = []

    if not dead_rows and not dead_cols:
        note2 = note
        if allow_gap and (gap_rows or gap_cols):
            note2 += (f"；豁免的内部间隔 行{[(a, b) for a, b in gap_rows]}"
                      f" 列{[(a, b) for a, b in gap_cols]}")
        return dict(key="dead_band", name=CHECK_NAMES["dead_band"], status="pass",
                    note=note2,
                    deduct=dict(margin=(mt, mr, mb, ml), label_band=label_band, edge=edge,
                                core=[r_core, c_core], windows=[c_nwin, r_nwin],
                                remainder=[r_rem, c_rem], gap_exempt=allow_gap))

    detail = []
    sx, sy = x0 + c_lo, y0 + r_lo  # 核心在整图中的原点
    for a, b in dead_cols:
        detail.append(f"  列死带 x∈[{sx + a}, {sx + b})  (区域/核心坐标 {a}..{b})，"
                      f"该 {band}px 窗口墨迹占比=0")
    for a, b in dead_rows:
        detail.append(f"  行死带 y∈[{sy + a}, {sy + b})  (区域/核心坐标 {a}..{b})，"
                      f"该 {band}px 窗口墨迹占比=0")
    return dict(key="dead_band", name=CHECK_NAMES["dead_band"], status="fail",
                note=note,
                reason=(f"发现 {len(dead_rows)} 条行死带 / {len(dead_cols)} 条列死带"
                        "（完整窗口内无任何墨迹；若这是多名/多列图正常的行/列间隔，"
                        "请加 --gap）"),
                detail=detail,
                deduct=dict(margin=(mt, mr, mb, ml), label_band=label_band, edge=edge,
                            core=[r_core, c_core], windows=[c_nwin, r_nwin],
                            remainder=[r_rem, c_rem], gap_exempt=allow_gap))
