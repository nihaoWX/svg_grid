#!/usr/bin/env python3
"""R6 的可跑证据（v7 round-data）。

R6 改了三件**行为**（用户裁决），本文件盯的就是它们别再被"修回去"：

1. **`<path>` 宽松化**（用户：「浏览器宽松设计应该是有道理的，我建议就是警告但还是可以跑通」）：
   畸形 `d` / 非有限数值 ⇒ **警告 + 原样透传**（`rc=0`、产物逐字节不变），**不再 fail-closed**。
2. **口径没有扩到别处**：`rect x="abc"` 这种"属性串根本不成形"**仍然 `exit≠0`**（负向对照）。
3. **W6 的微尘提示**：越界量**全部 < 2 px** 时在报文末尾加一句；只要有一条 ≥ 2 px 就不加。
4. **判据脚本从三条变两条**（删掉对位图恒真的"内容未触边"，其信息并进"外框 2 px 墨迹 = 0"）。

契约：`tools/svg_grid/ARCHITECTURE.md` §已定型的决定 + `ARCHITECTURE/tag-protocol.md` §5；`tools/pitfalls-manager/references/02_consume.md` §4.3。
无参数；秒级；离线；不写仓库（fixture 在临时目录自合成，故本目录**没有 `data/`**）。
"""
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R6 → PITFALLS → docs → svg_grid
SCRIPTS = SVG_GRID / "scripts"
FAILS = []

SVG_H = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">'
BOX = '<rect data-panel-box="main" x="0" y="0" width="100" height="100" fill="none" stroke="none"/>'


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


def run(tmp, name, body):
    svg = pathlib.Path(tmp) / f"{name}.svg"
    svg.write_text(SVG_H + BOX + body + "</svg>")
    out = pathlib.Path(tmp) / f"{name}.out.svg"
    p = subprocess.run([str(find_bin()), "plot-grid", "--input", str(svg), "--output", str(out),
                        "--ncol", "1", "--plot-margin", "0", "--width", "100", "--height", "100"],
                       capture_output=True, text=True)
    return p, (out if out.exists() else None)


def main():
    if find_bin() is None:
        print("SKIP  没找到 svg_grid 二进制 —— 先 `cd tools/svg_grid && cargo build --offline`"
              "（这是环境问题，不是行为差异）")
        return 0
    tmp = tempfile.mkdtemp(prefix="r6-rounddata-")
    try:
        # ── 1. 宽松化：畸形 d / 非有限值 ⇒ 出声但不阻断
        p, out = run(tmp, "bad_d", '<path d="M 10 20 30" fill="none" stroke="#000"/>')
        d = re.search(r'\bd="([^"]*)"', out.read_text()).group(1) if out else ""
        check(p.returncode == 0 and "could not be rewritten" in p.stderr and d == "M 10 20 30",
              "宽松化 畸形 d ⇒ rc=0 + W7 出声 + 产物里 d 逐字节原样",
              f"rc={p.returncode} d={d!r} stderr={p.stderr[:300]!r}")

        p, out = run(tmp, "nan_cx", '<circle cx="nan" cy="50" r="5" fill="#000"/>')
        ok = p.returncode == 0 and "non-finite" in p.stderr and out and 'cx="NaN"' in out.read_text()
        check(ok, "宽松化 非有限数值（cx=nan）⇒ rc=0 + W7 出声 + 产物仍是 NaN（只加声音）",
              f"rc={p.returncode} stderr={p.stderr[:300]!r}")

        # ── 2. 负向对照：口径**没有**被放宽到"属性串不成形"
        p, _ = run(tmp, "abc_rect", '<rect x="abc" y="0" width="10" height="10" fill="#000"/>')
        check(p.returncode != 0, "口径 属性串不成形（rect x=abc）⇒ 仍然 fail-closed（exit≠0）",
              f"rc={p.returncode} stderr={p.stderr[:200]!r}")

        # ── 3. W6 微尘提示：< 2 px 加、≥ 2 px 不加
        p, _ = run(tmp, "dust", '<text x="50" y="100.5" font-size="4" text-anchor="middle">x</text>')
        check("all overflows < 2 px" in p.stderr and "1.500 px" in p.stderr,
              "微尘提示 越界 1.5 px ⇒ 有提示（且仍报出真实数字）", f"stderr={p.stderr[:300]!r}")
        p, _ = run(tmp, "not_dust", '<text x="50" y="130" font-size="8" text-anchor="middle">x</text>')
        check("all overflows < 2 px" not in p.stderr,
              "微尘提示 越界 32 px ⇒ 无提示（负向对照）", f"stderr={p.stderr[:300]!r}")

        # ── 4. 判据脚本：两条检查 + selftest + 真实成品
        script = SCRIPTS / "check_deliverable.py"
        if not script.exists():
            print("SKIP  判据脚本不在（scripts/check_deliverable.py）")
        else:
            s = subprocess.run([sys.executable, str(script), "--selftest"], capture_output=True, text=True)
            check(s.returncode == 0, "判据脚本 --selftest rc=0（两条检查都既通过又触发）", s.stdout[-300:])
            real = SVG_GRID.parent.parent / "research/白藜芦醇胰腺癌/steps/10_下游表型/出图/成品"
            picks = [("Figure S39.png", 24), ("Figure S40.png", 24), ("Figure 7.png", 150)]
            got = [(n, m) for n, m in picks if (real / n).exists()]
            if not got:
                print("SKIP  真实成品不在（research 下那份目录）")
            for name, margin in got:
                r = subprocess.run([sys.executable, str(script), str(real / name), "--margin", str(margin)],
                                   capture_output=True, text=True)
                n_checks = len([l for l in r.stdout.splitlines() if l.startswith("[")])
                check(r.returncode == 0 and n_checks == 2,
                      f"判据脚本 {name} --margin {margin} ⇒ 两条检查全过、rc=0",
                      r.stdout[-400:] + r.stderr[-200:])
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
