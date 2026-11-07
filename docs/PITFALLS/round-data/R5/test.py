#!/usr/bin/env python3
"""R5 的可跑证据（v7 round-data）。

本轮把 `<path>` 纳入主库（用户裁决「直接做进主库」：**几何等价的搬运属于主库能力**），
并补上 ink.rs 对 `<path>` 与 `<defs>` 的可见性 —— 因为复核发现**同一个几何换个写法结论就不一样**：

  `polyline` ⇒ W6 报；**`path`** ⇒ 静默；**`<defs>`+`<use>`** ⇒ 静默。

本文件盯的就是这两件事：**三种写法必须有一致结论**；以及搬运/弧/归一化/defs 的几条硬口径。
契约：`tools/svg_grid/ARCHITECTURE.md` §已定型的决定 + `ARCHITECTURE/tag-protocol.md` §3/§4；`tools/pitfalls-manager/references/02_consume.md` §4.3。

无参数；秒级；离线；不写仓库（fixture 在临时目录自合成，故本目录**没有 `data/`**）。
"""
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R5 → PITFALLS → docs → svg_grid
FAILS = []

SVG_H = ('<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
         'viewBox="0 0 {w} {h}" width="{w}" height="{h}">')
BOX = '<rect data-panel-box="main" x="0" y="0" width="{w}" height="{h}" fill="none" stroke="none"/>'


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def find_bin():
    for rel in ("target/debug/svg_grid", "target/release/svg_grid"):
        if (SVG_GRID / rel).exists():
            return SVG_GRID / rel
    return None


def run(tmp, name, body, w, h, out_w, out_h):
    svg = pathlib.Path(tmp) / f"{name}.svg"
    svg.write_text((SVG_H + BOX + "</svg>").format(w=w, h=h).replace("</svg>", body + "</svg>"))
    out = pathlib.Path(tmp) / f"{name}.out.svg"
    p = subprocess.run([str(find_bin()), "plot-grid", "--input", str(svg), "--output", str(out),
                        "--ncol", "1", "--plot-margin", "0", "--width", str(out_w), "--height", str(out_h)],
                       capture_output=True, text=True)
    return p, (out if out.exists() else None)


def overflow(p):
    m = re.search(r"overflows ([a-z]+) by ([0-9.]+) px", p.stderr)
    return (m.group(1), m.group(2)) if m else None


def main():
    if find_bin() is None:
        print("SKIP  没找到 svg_grid 二进制 —— 先 `cd tools/svg_grid && cargo build --offline`"
              "（这是环境问题，不是行为差异）")
        return 0
    tmp = tempfile.mkdtemp(prefix="r5-rounddata-")
    try:
        # ── 族：三种写法必须有一致结论（本轮的核心）
        geom = [(10, 10), (172.5, 10), (172.5, 40)]
        bodies = {
            "polyline": '<polyline points="10,10 172.5,10 172.5,40" fill="none" stroke="#000"/>',
            "path":     '<path d="M 10 10 L 172.5 10 L 172.5 40" fill="none" stroke="#000"/>',
            "defs_use": '<defs><polyline id="g" points="0,0 162.5,0 162.5,30" fill="none" stroke="#000"/>'
                        '</defs><use xlink:href="#g" x="10" y="10"/>',
        }
        res = {}
        for k, body in bodies.items():
            p, _ = run(tmp, k, body, 100, 80, 100, 80)
            res[k] = (p.returncode, p.stderr, overflow(p))
        base = res["polyline"]
        for k in ("path", "defs_use"):
            check(res[k][1] == base[1] and res[k][2] == base[2] and res[k][2] is not None,
                  f"族-一致性 `{k}` 的 W6 与 `polyline` 一字不差（同一几何、三种写法）",
                  f"base={base[1][:200]!r}\n{k}={res[k][1][:200]!r}")

        p, _ = run(tmp, "clean", '<path d="M 10 10 L 90 10 L 90 70" fill="none" stroke="#000"/>', 100, 80, 100, 80)
        check(p.stderr == "", "族-负向 画布内的 path ⇒ stderr 0 字节（不误报）", f"stderr={p.stderr!r}")
        check(geom and base[2] is not None, "族-前置 越界确实被测到（非空转）", f"{base}")

        # ── 搬运：path 与 polyline 同值
        scale = '<g data-scale="xy">'
        pa, oa = run(tmp, "tr_poly", scale + '<polyline points="10,10 90,10 90,40" fill="none" stroke="#000"/></g>', 100, 80, 200, 160)
        pb, ob = run(tmp, "tr_path", scale + '<path d="M 10 10 L 90 10 L 90 40" fill="none" stroke="#000"/></g>', 100, 80, 200, 160)
        pts = re.search(r'points="([^"]*)"', oa.read_text()).group(1) if oa else ""
        d = re.search(r'\bd="([^"]*)"', ob.read_text()).group(1) if ob else ""
        nums_pts = [float(x) for x in re.split(r"[, ]+", pts) if x]
        nums_d = [float(x) for x in re.split(r"[ ,]+", d.replace("M", " ").replace("L", " ")) if x]
        check(nums_pts == nums_d and len(nums_pts) == 6,
              "搬运 path 与 polyline 得到同一组数值（data-scale=xy、2×）", f"pts={pts!r} d={d!r}")

        # ── 弧的两条规则
        a1, o1 = run(tmp, "arc_u", scale + '<path d="M 10 50 A 10 20 0 0 1 50 40" fill="none" stroke="#000"/></g>', 100, 100, 200, 200)
        s1 = re.search(r'\bd="([^"]*)"', o1.read_text()).group(1) if o1 else ""
        check("A 20 40 0 0 1" in s1,
              "弧-等比：保 `A`、rx/ry 按轴缩放、rotation 与两个 flag 逐字节不动", f"d={s1!r}")
        a2, o2 = run(tmp, "arc_n", scale + '<path d="M 10 50 A 10 20 30 0 1 50 40" fill="none" stroke="#000"/></g>', 100, 100, 200, 100)
        s2 = re.search(r'\bd="([^"]*)"', o2.read_text()).group(1) if o2 else ""
        check(not re.search(r"[Aa]\s", s2) and "L" in s2,
              "弧-非等比×旋转：该弧展平成折线（产物里没有 `A`）", f"d={s2[:150]!r}")

        # ── 悬空 <use> 不得误报（复核发现并修）
        p, _ = run(tmp, "dangling", '<use xlink:href="#nope" x="200" y="200"/>', 100, 100, 100, 100)
        check(overflow(p) is None, "defs-悬空引用：什么都不画 ⇒ 不报 W6（不误报）", f"stderr={p.stderr[:300]!r}")
        # (b) 存在但不受支持（symbol）：保留 placement 盒 ⇒ 仍报（防"悬空"那支把 (b) 也一起放过）
        p, _ = run(tmp, "symbol_use",
                   '<defs><symbol id="sym"><rect x="0" y="0" width="10" height="10" fill="#000"/></symbol></defs>'
                   '<use xlink:href="#sym" x="200" y="200"/>', 100, 100, 100, 100)
        check(overflow(p) is not None,
              "defs-存在但不受支持（symbol）⇒ 仍报 W6（只松开了悬空那支）", f"stderr={p.stderr[:300]!r}")
        # ── 未被引用的 <defs> 内容不得参与墨迹
        p, _ = run(tmp, "unused_defs",
                   '<defs><rect id="u" x="200" y="200" width="50" height="50" fill="#000"/></defs>'
                   '<rect x="10" y="10" width="20" height="20" fill="#000"/>', 100, 100, 100, 100)
        check(overflow(p) is None, "defs-未被引用：其中的内容不参与墨迹（不误报）", f"stderr={p.stderr[:300]!r}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
