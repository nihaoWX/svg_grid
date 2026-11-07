#!/usr/bin/env python3
"""R9 的可跑证据 —— **HARNESS 归位 + skill 瘦身**。

钉住四件事（都是本轮定的、以后不许回退的）：

1. **文档体积**：HARNESS **两件套**（`ARCHITECTURE.md` + `PROGRESS.md`）、`ARCHITECTURE/*.md`、
   指南（`SKILL.md` + `references/*.md`）全部 ≤ 20000 B；且**不设 `DECISIONS.md`**（2026-10-06 起：
   决定并入 `ARCHITECTURE.md` §已定型的决定）。
2. **两份副本逐字节一致**（`diff -r` 为空）。
3. **指南只写现状**：不出现变更叙述词（"已支持/已修复/不再需要/早期脚本/已现场实测收口"），
   且 `02a` 里给出指向 `ARCHITECTURE/warnings.md` 的指针（告警清单不在指南里重复）。
4. **每条告警自带修法**：W1–W7 各由一个合成 fixture 触发，各自 stderr 必须含 `Fix:`；
   负向对照（干净输入）必须 stderr 0 字节。

外加：逐个跑既有五套证据（R4–R8）。

契约见 `tools/svg_grid/ARCHITECTURE.md` §已定型的决定（HARNESS 两件套、决定并入此处）与
`ARCHITECTURE/warnings.md` §4。无参数；秒级；离线；不写仓库。
"""
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]          # round-data/R9 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]              # → bioagentforge（根仓库）
LIMIT = 20000
FAILS = []

SVG_H = '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" ' \
        'viewBox="0 0 200 200" width="200" height="200">'
BOX = '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'
CLEAN_BODY = '<rect x="20" y="20" width="60" height="60" fill="#000"/>'


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def main():
    # ── 1. 文档体积（HARNESS 两件套；2026-10-06 起不设 DECISIONS.md）
    docs = [SVG_GRID / "ARCHITECTURE.md", SVG_GRID / "PROGRESS.md"]
    docs += sorted((SVG_GRID / "ARCHITECTURE").glob("*.md"))
    guide = SVG_GRID / "docs/general-figure-guide"
    docs += [guide / "SKILL.md"] + sorted((guide / "references").glob("*.md"))
    over = [(str(p.relative_to(SVG_GRID)), p.stat().st_size) for p in docs if p.stat().st_size > LIMIT]
    check(not over, f"体积 全部文档 ≤ {LIMIT} B；共 {len(docs)} 份：HARNESS 两件套 + ARCHITECTURE/* + 指南", over)
    biggest = sorted(((p.stat().st_size, str(p.relative_to(SVG_GRID))) for p in docs), reverse=True)[:3]
    print("      最大三个: " + ", ".join(f"{n} {s}B" for s, n in biggest))
    check(not (SVG_GRID / "DECISIONS.md").exists(),
          "HARNESS 两件套 不设 DECISIONS.md（2026-10-06；决定并入 ARCHITECTURE.md §已定型的决定）")
    check("已定型的决定" in (SVG_GRID / "ARCHITECTURE.md").read_text(),
          "ARCHITECTURE.md 有 `# 已定型的决定` 一节（迁入的决定都在这里）")

    # ── 2. 两副本一致
    other = REPO / "tools/manuscripts_skills/general-figure-guide"
    d = subprocess.run(["diff", "-r", str(guide), str(other)], capture_output=True, text=True)
    check(d.returncode == 0, "副本 两处 general-figure-guide 逐字节一致（diff -r 为空）", d.stdout[:400])

    # ── 3. 指南只写现状
    changelog = re.compile(r"已支持|已修复|不再需要|早期脚本|已现场实测收口|现在会被精确烘进")
    hits = []
    for p in [guide / "SKILL.md"] + sorted((guide / "references").glob("*.md")):
        for i, line in enumerate(p.read_text().splitlines(), 1):
            if changelog.search(line):
                hits.append(f"{p.name}:{i} {line.strip()[:70]}")
    check(not hits, "指南 不含变更叙述词（已支持/已修复/不再需要/早期脚本/已现场实测收口）", hits)
    text_02a = (guide / "references/02a_svg_grid_assembly.md").read_text()
    check("ARCHITECTURE/warnings.md" in text_02a,
          "指南 02a 里给了指向 `ARCHITECTURE/warnings.md` 的指针（告警清单不在这里重复）")

    # ── 4. 每条告警自带修法
    binary = SVG_GRID / "target/debug/svg_grid"
    if not binary.exists():
        print("SKIP  告警契约：没找到 svg_grid 二进制（先 cargo build --offline）")
    else:
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="r9-rounddata-"))
        def panel(name, body= CLEAN_BODY):
            p = tmp / f"{name}.svg"; p.write_text(SVG_H + BOX + body + "</svg>"); return p
        clean = panel("clean")
        w1 = panel("w1", '<g data-scale="xy"><rect x="20" y="20" width="60" height="60" fill="#000"/></g>')
        w6 = panel("w6", '<text x="100" y="240" font-size="8" text-anchor="middle">B</text>')
        w7 = panel("w7", '<path d="M 10 20 30" fill="none" stroke="#000"/>')
        w4b_1, w4b_2 = panel("w4b1"), panel("w4b2")

        def run(inputs, out, extra):
            cmd = [str(binary), "plot-grid", "--input", str(inputs[0])]
            for i in inputs[1:]:
                cmd += ["--input", str(i)]
            cmd += ["--output", str(out), "--width", "400", "--ncol", str(len(inputs))] + extra
            return subprocess.run(cmd, capture_output=True, text=True)

        # W3 需要"已带字母的输入" ⇒ 先自己造一个
        first = subprocess.run([str(binary), "plot-grid", "--input", str(w4b_1), "--input", str(w4b_2),
                                "--output", str(tmp / "lettered.svg"), "--width", "400", "--ncol", "2",
                                "--labels", "A,B", "--plot-margin", "0"],
                               capture_output=True, text=True)
        lettered = tmp / "lettered.svg"
        cases = [
            ("W1 输入被缩放", run([w1], tmp / "o1.svg", ["--plot-margin", "0"]), "scaled by the layout"),
            ("W2 格子被拉伸", run([clean], tmp / "o2.svg", ["--plot-margin", "0", "--height", "200"]), "are stretched"),
            ("W3 重复加字母", run([lettered], tmp / "o3.svg", ["--plot-margin", "0", "--labels", "C"]),
             "already contain panel letters"),
            ("W4a 字母个数不符", run([clean], tmp / "o4.svg", ["--plot-margin", "0", "--labels", "A,B"]),
             "entries but the grid has"),
            ("W4b 字母重复", run([w4b_1, w4b_2], tmp / "o5.svg", ["--plot-margin", "0", "--labels", "A,A"]),
             "repeats some letters"),
            ("W5 字母会被裁", run([clean], tmp / "o6.svg", ["--plot-margin", "0", "--labels", "A"]),
             "may be clipped by the canvas"),
            ("W6 内容越出画布", run([w6], tmp / "o7.svg", ["--plot-margin", "0"]), "draws ink outside its own canvas"),
            ("W7 无法重写", run([w7], tmp / "o8.svg", ["--plot-margin", "0"]), "could not be rewritten"),
        ]
        assert lettered.exists(), f"预备的带字母输入没生成：{first.stderr[:200]}"
        for label, p, marker in cases:
            stderr = p.stderr
            check(marker in stderr and "Fix:" in stderr,
                  f"告警契约 {label} ⇒ 有告警 **且自带 `Fix:`**", stderr[:400])
        neg = run([clean], tmp / "o9.svg", ["--plot-margin", "0", "--width", "200", "--height", "200"])
        check(neg.stderr == "" and neg.returncode == 0,
              "告警契约 负向对照：干净输入 ⇒ stderr 0 字节、rc=0（不误报）", f"rc={neg.returncode} {neg.stderr[:200]!r}")

    # ── 5. 既有五套证据
    for r in ("R4", "R5", "R6", "R7", "R8"):
        t = SVG_GRID / "docs/PITFALLS/round-data" / r / "test.py"
        p = subprocess.run([sys.executable, str(t)], capture_output=True, text=True)
        check(p.returncode == 0, f"回归 既有证据 round-data/{r} 仍全绿", (p.stdout[-300:] + p.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
