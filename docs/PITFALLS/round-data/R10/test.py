#!/usr/bin/env python3
"""R10 的可跑证据 —— **指南的规范源裁决 + 账本两条路径并轨**
（就地补：R10 入账时漏建，2026-10-06 补——见同目录 `meta.toml`）。

盯两族（都是"同一份文件的身份变了、而账本按路径索引跟不上"）：

1. **规范源唯一**：`tools/svg_grid/docs/general-figure-guide/` 是唯一维护处，另一处是**逐字节相同的复制件**；
   `ARCHITECTURE/general-figure-guide.md` §3 明写规范源 + `rsync -a --delete` 覆盖 + `diff -r` 校验 + 永不单独编辑。
2. **账本并轨**（`link` L6/L7/L8）：同一份文件的**两条路径**在 `query` 下命中**同一批**记录
   （`02a` / `05_checklist` / `SKILL.md` 三对），且相关位置**不许**悬空。

契约见 `tools/svg_grid/ARCHITECTURE.md` §已定型的决定 与 `ARCHITECTURE/general-figure-guide.md` §3。
无参数；秒级；离线；**只读**（不写仓库、不构建）。
"""
import pathlib
import re
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]          # round-data/R10 → PITFALLS → docs → svg_grid
REPO = HERE.parents[5]              # → bioagentforge（根仓库）
# 账本工具 2026-11-04 起是 Rust 二进制、以**已安装的命令**为唯一接口（`history.py` 已删）。
PM = shutil.which("pitfalls-manager")
GUIDE = SVG_GRID / "docs/general-figure-guide"
COPY = REPO / "tools/manuscripts_skills/general-figure-guide"
PAIRS = ["references/02a_svg_grid_assembly.md", "references/05_checklist.md", "SKILL.md"]
FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def ids(text):
    return set(re.findall(r"[A-Za-z][A-Za-z0-9_-]*-F\d+", text))


def query(path):
    # 2026-11-04 起账本由 pitfalls-manager（Rust 二进制）管理，按 --repo（项目目录）定位；
    # `history.py` 已随 Rust 化删除，故一律走**已安装的命令**。
    p = subprocess.run([PM, "query", path, "--repo", str(SVG_GRID)],
                       capture_output=True, text=True)
    return p.stdout, p.stderr


def main():
    if not PM:
        print("FAIL  账本工具不可用：`pitfalls-manager` 不在 PATH"
              "（安装见 tools/pitfalls-manager/SKILL.md；`history.py` 已随 Rust 化删除）")
        return 1
    # ── 1. 规范源唯一
    d = subprocess.run(["diff", "-r", str(GUIDE), str(COPY)], capture_output=True, text=True)
    check(d.returncode == 0, "规范源 复制件与规范源逐字节一致（`diff -r` 为空）", d.stdout[:400])
    text = (SVG_GRID / "ARCHITECTURE/general-figure-guide.md").read_text()
    for token in ("规范源", "rsync -a --delete", "diff -r", "永不单独编辑"):
        check(token in text, f"规范源 §3 写明「{token}」")

    # ── 2. 账本并轨：两条路径命中同一批 + 不悬空
    for rel in PAIRS:
        a = "tools/svg_grid/docs/general-figure-guide/" + rel
        b = "tools/manuscripts_skills/general-figure-guide/" + rel
        out_a, err_a = query(a)
        out_b, err_b = query(b)
        same = ids(out_a) and ids(out_a) == ids(out_b)
        check(same, f"并轨 {rel} 两条路径命中同一批（{len(ids(out_a))} 条）",
              f"A={sorted(ids(out_a))}\nB={sorted(ids(out_b))}")
        check("悬空" not in (err_a + err_b), f"并轨 {rel} query 的悬空自检无警告", (err_a + err_b)[:400])

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
