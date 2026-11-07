# -*- coding: utf-8 -*-
"""汇总：跑完两条检查，并把一条结果渲染成"一行"。"""

from .common import DEFAULT_MARGIN, DEFAULT_BAND, DEFAULT_INK_THRESHOLD
from .dead_band import check_dead_band
from .outer_frame import check_outer_frame


def run_checks(gray, margin=DEFAULT_MARGIN, band=DEFAULT_BAND,
               ink_threshold=DEFAULT_INK_THRESHOLD, label_band=0, allow_gap=False):
    """跑完两条检查，返回 (results, exit_code)。"""
    results = [
        check_dead_band(gray, margin, band, ink_threshold, label_band, allow_gap),
        check_outer_frame(gray, ink_threshold),
    ]
    failed = sum(1 for r in results if r["status"] == "fail")
    return results, (0 if failed == 0 else 1)


def format_result(index, result):
    """把一条结果渲染成"一行"（失败时附细节行）。"""
    tag = {"pass": "PASS", "fail": "FAIL", "skip": "SKIP"}[result["status"]]
    line = f"[{index}/2] {result['name']} ... {tag} | {result.get('note', '')}"
    lines = [line]
    if result["status"] == "fail":
        if result.get("reason"):
            lines.append("    原因: " + result["reason"])
        for extra in result.get("detail", []):
            lines.append("    " + extra)
    if result["status"] == "skip" and result.get("reason"):
        lines.append("    跳过原因: " + result["reason"])
    return "\n".join(lines)
