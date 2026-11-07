# -*- coding: utf-8 -*-
"""--selftest：在合成数组上证明 检查 1/2 "既会通过、也会触发"。"""

import numpy as np

from .common import DEFAULT_BAND
from .dead_band import check_dead_band
from .outer_frame import check_outer_frame


def _rect(gray, y0, y1, x0, x1, value):
    gray[y0:y1, x0:x1] = value


def _label_band_figure():
    """一张"面板 + 顶部字母带"的合成图：字母带的墨迹上沿落在 margin(24) 之内
    （y=15，与真实 Figure S39/S40 一致），但整幅墨迹仍在**画布**内、外框 2 px 圈内无墨。

    它就是 `--margin` 之外那条"字母带住在 margin 里属正常排版"的正例——检查 1
    （扣掉字母带）与检查 2（只看最外 2 px）都必须判 pass。
    """
    img = np.full((900, 900), 255, np.uint8)
    _rect(img, 60, 840, 60, 840, 100)     # 面板主体
    _rect(img, 15, 48, 62, 110, 0)        # 面板左上外侧的字母带（墨迹进入 margin）
    return img


def _selftest_images():
    """返回 {name: (gray, kwargs, expected_status_by_check)}。"""
    band = DEFAULT_BAND

    # A. 干净成品：内容落在画布内、外框 2px 无墨、无内部整块无墨带 -> 两条全过
    clean = np.full((900, 900), 255, np.uint8)
    _rect(clean, 60, 840, 60, 840, 100)

    # B. 真死带：在内容中间横切一条 260px 全白带（上下都有内容 = 内部） -> 检查 1 报
    dead = clean.copy()
    _rect(dead, 400, 660, 0, 900, 255)

    # C. 通栏面板 + 轴右侧外侧图例：内容约占 60-85%，最右是页边距（>band） -> 检查 1 不报
    wide = np.full((500, 1200), 255, np.uint8)
    _rect(wide, 80, 420, 40, 700, 30)     # 小图（左）
    _rect(wide, 200, 260, 820, 900, 0)    # 右侧外侧图例

    # D. 外框 2px 越界墨（左上角 2x2，坐标 x=0,y=0 仍在**画布**内）-> 只由检查 2 报
    frame = clean.copy()
    _rect(frame, 0, 2, 0, 2, 0)

    # F. 字母带落在 margin 内（= 真实 Figure S39/S40/Figure 7 的形态）：两条全过。
    letters = _label_band_figure()

    return {
        "A_clean": (clean, dict(band=band, margin=24, allow_gap=False),
                    dict(dead_band="pass", outer_frame="pass")),
        "B_real_dead_band": (dead, dict(band=band, margin=24, allow_gap=False),
                             dict(dead_band="fail", outer_frame="pass")),
        "C_fullwidth_right_legend": (wide, dict(band=band, margin=24, allow_gap=False),
                                     dict(dead_band="pass", outer_frame="pass")),
        "D_outer_frame_ink": (frame, dict(band=band, margin=24, allow_gap=False),
                              dict(dead_band="pass", outer_frame="fail")),
        "F_letter_band_in_margin": (letters, dict(band=band, margin=24, allow_gap=False),
                                    dict(dead_band="pass", outer_frame="pass")),
    }


def selftest():
    """在合成数组上跑两条检查，断言期望的 pass/fail。全对 -> 0，否则 -> 1。"""
    images = _selftest_images()
    failures = []
    print("check_deliverable --selftest（合成数组；检查 1/2 既会通过也会触发）")
    for name, (gray, kwargs, expected) in images.items():
        results = [
            check_dead_band(gray, kwargs["margin"], kwargs["band"],
                            allow_gap=kwargs.get("allow_gap", False)),
            check_outer_frame(gray),
        ]
        got = {r["key"]: r["status"] for r in results}
        line = f"  {name:26s} " + " ".join(
            f"{key}={got[key]}(exp {expected[key]})" for key in expected)
        ok = all(got[k] == expected[k] for k in expected)
        print(("  OK  " if ok else "  BAD ") + line.strip())
        if not ok:
            failures.append(name)

    # --- 负向对照：两条保留的检查都必须会触发 ---
    print("  --- 负向对照 ---")
    dead = np.full((900, 900), 255, np.uint8)
    _rect(dead, 60, 840, 60, 840, 100)
    _rect(dead, 400, 660, 0, 900, 255)
    r_b = check_dead_band(dead, 24, DEFAULT_BAND)
    print(f"  造真死带      -> 检查 1 = {r_b['status']} (须为 fail)；{r_b.get('reason', '')}")
    if r_b["status"] != "fail":
        failures.append("B_real_dead_band(assert)")
    out = np.full((900, 900), 255, np.uint8)
    _rect(out, 60, 840, 60, 840, 100)
    _rect(out, 0, 2, 0, 2, 0)
    r_f = check_outer_frame(out)
    print(f"  造 2px 越界墨  -> 检查 2 = {r_f['status']} (须为 fail)；{r_f['note']}")
    if r_f["status"] != "fail":
        failures.append("D_outer_frame_ink(assert)")

    # --- 正向对照：两条保留的检查都必须会通过 ---
    print("  --- 正向对照 ---")
    clean = np.full((900, 900), 255, np.uint8)
    _rect(clean, 60, 840, 60, 840, 100)
    r_d = check_dead_band(clean, 24, DEFAULT_BAND)
    r_o = check_outer_frame(clean)
    print(f"  造干净图      -> 检查 1 = {r_d['status']} / 检查 2 = {r_o['status']} "
          f"(须都 pass)；{r_o['note']}")
    if r_d["status"] != "pass" or r_o["status"] != "pass":
        failures.append("A_clean(assert)")

    if failures:
        print(f"SELFTEST FAILED: {failures}")
        return 1
    print("SELFTEST PASSED: 检查 1/2 都既通过又触发；正向（干净图）/ 负向（真死带、2px 越界墨）"
          "对照均符合预期。")
    return 0
