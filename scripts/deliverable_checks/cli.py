# -*- coding: utf-8 -*-
"""CLI：argparse + 调用 + 退出码（薄壳入口 `scripts/check_deliverable.py` 的唯一实现）。"""

import argparse
import json

from .common import (
    DEFAULT_MARGIN,
    DEFAULT_BAND,
    DEFAULT_INK_THRESHOLD,
    load_gray,
    parse_margin,
)
from .summary import run_checks, format_result
from .selftest import selftest


def build_parser():
    ap = argparse.ArgumentParser(
        prog="check_deliverable.py",
        description="成品图交付前的两条程序化检查（唯一判据；退出码即结论）。",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("image", nargs="?", help="成品 PNG（或任意 Pillow 可读的位图）")
    ap.add_argument("--margin", default=str(DEFAULT_MARGIN),
                    help="四边扣除的 margin，px：单个 N 或 T,R,B,L（默认 24）")
    ap.add_argument("--gap", action="store_true",
                    help="豁免内部整块无墨带（多名/多列图正常的行/列间隔）")
    ap.add_argument("--label-band", type=int, default=0, dest="label_band",
                    help="顶部额外扣掉的字母带高度，px（默认 0）")
    ap.add_argument("--band", type=int, default=DEFAULT_BAND,
                    help="死白带窗口大小，px（默认 200）")
    ap.add_argument("--ink-threshold", type=int, default=DEFAULT_INK_THRESHOLD,
                    dest="ink_threshold", help="墨阈值：灰度 < 该值算墨（默认 245）")
    ap.add_argument("--json", action="store_true", help="以 JSON 输出结论")
    ap.add_argument("--selftest", action="store_true",
                    help="在合成数组上自检（证明每条检查既会通过也会触发）")
    return ap


def main(argv=None):
    ap = build_parser()
    args = ap.parse_args(argv)
    if args.selftest:
        return selftest()
    if not args.image:
        ap.error("缺少 <成品.png>（或用 --selftest）")

    gray = load_gray(args.image)
    H, W = gray.shape
    results, code = run_checks(gray, margin=args.margin, band=args.band,
                               ink_threshold=args.ink_threshold,
                               label_band=args.label_band, allow_gap=args.gap)

    if args.json:
        payload = dict(image=args.image, size=[W, H],
                       ink_threshold=args.ink_threshold,
                       margin=parse_margin(args.margin),
                       band=args.band, label_band=args.label_band, gap=args.gap,
                       results=[{k: v for k, v in r.items()} for r in results],
                       exit_code=code)
        print(json.dumps(payload, ensure_ascii=False, indent=2, default=str))
        return code

    ink_frac = float((gray < args.ink_threshold).mean())
    print(f"check_deliverable: {args.image}")
    print(f"image: {W}x{H} px  ink(<{args.ink_threshold})={ink_frac * 100:.3f}%")
    for i, r in enumerate(results, 1):
        print(format_result(i, r))
    overall = "PASS" if code == 0 else "FAIL"
    nfail = sum(1 for r in results if r["status"] == "fail")
    nskip = sum(1 for r in results if r["status"] == "skip")
    print(f"RESULT: {overall} ({nfail} failed, {nskip} skipped)  exit {code}")
    return code
