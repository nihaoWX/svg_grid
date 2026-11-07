#!/usr/bin/env python3
"""R11 的可跑证据 —— **DECISIONS 并入 ARCHITECTURE、HARNESS 变两件套**。

盯三族：

1. **两件套成立**：`tools/` 下不再有任何 `DECISIONS.md`；`tools/svg_grid/ARCHITECTURE.md` 有 `# 已定型的决定`
   一节，且**关键取舍没在迁移中丢掉**（几个只此一处的决定点仍在）。
2. **根 `AGENTS.md` 已把 DECISIONS 移出 HARNESS**（一字不提）。
3. **没有活引用残留**：出现在 **HARNESS / skill / 文档**里的每一处 `DECISIONS`，那一行必须同时是
   **现行口径的陈述**（不设 / 已删 / 已于 / 并入 / 移出 / 全仓 / 去设计文档 / 这些条目记在）。
   范围排除两类**历史载体**（按机制不该改写）：`round-data/**`（旧轮证据）与 `docs/PITFALLS/*.md`（报告 / 驱动文档）。

契约见 `tools/svg_grid/ARCHITECTURE.md` §已定型的决定 与根 `AGENTS.md`。
无参数；秒级；离线；只读。
"""
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]          # round-data/R11 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]              # → bioagentforge（根仓库）
TOOLS = REPO / "tools"
NEW_RULE = re.compile(r"不设|已删|已于|并入|移出|全仓|去设计文档|这些条目记在|未迁")
KEEP = ["raster-images", "plot_grid.R", "不做 `--strict`", "include!", "rsync -a --delete"]
FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def decision_rows(arch):
    """`# 已定型的决定` 一节表格里的**决定行数**（不含表头与分隔行）。"""
    m = re.search(r"^# 已定型的决定\s*$", arch, re.M)
    if not m:
        return -1
    tail = arch[m.end():].split("\n# ")[0]
    return len([l for l in tail.splitlines() if l.startswith("|") and "---" not in l]) - 1


def historically_excluded(p):
    s = str(p)
    return "/round-data/" in s or "/docs/PITFALLS/" in s or p == pathlib.Path(__file__)


def main():
    # ── 1. 两件套成立 + 迁移没丢关键取舍
    leftovers = sorted(str(p.relative_to(REPO)) for p in TOOLS.rglob("DECISIONS.md"))
    check(not leftovers, "两件套 tools/ 下不再有 DECISIONS.md", leftovers)
    arch = (SVG_GRID / "ARCHITECTURE.md").read_text()
    n = decision_rows(arch)
    check(n >= 10, f"迁移 `# 已定型的决定` 有 {n} 行决定（表头另计；≥10 行）")
    missing = [t for t in KEEP if t not in arch]
    check(not missing, "迁移 关键取舍都还在（没在合并时丢掉）", f"缺：{missing}")

    # ── 2. 根 AGENTS.md 一字不提 DECISIONS
    check("DECISIONS" not in (REPO / "AGENTS.md").read_text(),
          "根 AGENTS.md 已把 DECISIONS 移出 HARNESS（一字不提）")

    # ── 3. 没有活引用残留
    #
    # 范围 = **git 跟踪的文件**（不是整个 tools/ 文件系统）。2026-11-06 起 tools/ 下有别的工具的
    # 运行产物（如 `tools/cao/.build/**`，根 `.gitignore` 已忽略的 vendored 源码），它们不是本仓文档，
    # 把它们算进来会让这条断言随别的工具的运行状态变红 —— 那是范围错，不是本仓残留。
    # 判据仍是同一条：本仓现行的 md/py 里不许有指向已删 DECISIONS 文件、又没带新规则的活引用。
    tracked = subprocess.run(["git", "-C", str(REPO), "ls-files", "--", "tools"],
                             capture_output=True, text=True, check=True).stdout.splitlines()
    candidates = [
        REPO / rel for rel in tracked
        if rel.endswith((".md", ".py")) and (REPO / rel).is_file()
    ]
    bad = []
    for p in sorted(candidates):
        if historically_excluded(p):
            continue
        for i, line in enumerate(p.read_text(errors="ignore").splitlines(), 1):
            if "DECISIONS" in line and not NEW_RULE.search(line):
                bad.append(f"{p.relative_to(REPO)}:{i} {line.strip()[:76]}")
    check(not bad, "残留 现行文档里没有指向已删文件的活引用", bad)

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
