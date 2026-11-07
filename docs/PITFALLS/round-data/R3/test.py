#!/usr/bin/env python3
"""R3 的 round-data —— R3 补进 `02a_svg_grid_assembly.md` 的那些文档结论，今天是否仍写在纸上。

R3（2026-09-27）的闭包全是**文档缺口**类：它没有改代码，而是把 svg_grid 的静默失败与尺度陷阱
写进 `references/02a_svg_grid_assembly.md`（坑 9 的 W1 缩放、同一行必须同宽同高、`--plot-margin`
单值/四值语义、外层必须显式 0、同纵横比优先单次 flat、字母带按行累加、行调用留 24 内缩、
`data-panel-box` 不能当面板数）。这类结论**没有运行时可断言的对象**——唯一能机械判定的是
「这些话还在不在」。

因此本证据按**语义锚点**（不是行号：行号每次编辑都会漂，R3 记的 128/136/145-148 早已移动）
逐条 grep：少一条就红，提醒后来者「你以为只是删了一句，其实删掉的是 R3 的整个结论」。

只读、无参数、秒级、离线；不写仓库、不建目录。
"""
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]          # round-data/R3 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]              # → bioagentforge
GUIDE = SVG_GRID / "docs/general-figure-guide"
COPY = REPO / "tools/manuscripts_skills/general-figure-guide"
DOC = GUIDE / "references/02a_svg_grid_assembly.md"

fails = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"  [{detail}]"))
    if not ok:
        fails.append(label)
    return ok


# (锚点, 它承载的规则) —— 锚点取"这句话只有这一层意思"的短语，避免被无关行误命中。
ANCHORS = [
    ("给 1 个值时**四边同值**",
     "R3-F3 --plot-margin 单值 = 四边同值（4 个才是 top,right,bottom,left）"),
    ("同一行内的面板必须",
     "R3-F2 同一行内必须同宽同高（不只是同宽）"),
    ("同宽 + 同高",
     "R3-F2 同上（行高共享 ⇒ 纸面字号由行高决定）"),
    ("一行内的面板等宽等高",
     "R3-F2 交付前必须成立的 5 件事之⑤"),
    ("会自动留出字母带",
     "R3-F1 用 --labels 且未显式给 --plot-margin 时自动留字母带（W1 缩放的诱因）"),
    ("必须显式给 `--plot-margin 0`",
     "R3-F4 外层拼装必须显式给 --plot-margin 0（默认 24 会把行块再缩一次）"),
    ("单次 flat 调用",
     "R3-F4 同纵横比优先单次 flat 调用（逐行+外层只在纵横比不同时才必要）"),
    ("字母带按行累加",
     "R3-F5 逐行加 --labels ⇒ 每行各留一条字母带，画布高按行累加"),
    ("行调用自动字母带的横向内缩",
     "R3-F6 行调用留的字母带内缩 = 默认 --plot-margin(24)，须与格子模型对齐"),
    ("的元素坐标反解",
     "R3-F7 核对 s 用带 data-scale 标签的元素坐标反解（不要看 data-panel-box）"),
    ("的出现次数不是面板数",
     "R3-F7 data-panel-box=\"main\" 的出现次数不能当面板数"),
]


def main():
    if not DOC.is_file():
        print(f"FAIL  {DOC} 不存在\n")
        return 1
    text = DOC.read_text(encoding="utf-8")
    size = len(DOC.read_bytes())

    print("== ① 文档在、且没超体积口径 ==")
    check(size <= 20000, f"02a_svg_grid_assembly.md = {size} B（≤ 20000）", f"{size} B")

    print("\n== ② R3 补进去的每条结论仍在 ==")
    for anchor, rule in ANCHORS:
        check(anchor in text, f"{rule}   ⟨锚点: {anchor}⟩")

    print("\n== ③ 两副本都还在（结论不能只留在规范源）==")
    d = subprocess.run(["diff", "-r", str(GUIDE), str(COPY)], capture_output=True, text=True)
    check(d.returncode == 0, "复制件与规范源逐字节一致（`diff -r` 为空）", d.stdout[:300])
    copy_doc = COPY / "references/02a_svg_grid_assembly.md"
    check(copy_doc.is_file() and copy_doc.read_text(encoding="utf-8") == text,
          "复制件里的 02a 与规范源同文")

    print()
    if fails:
        print(f"== {len(fails)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
