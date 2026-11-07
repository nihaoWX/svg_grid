#!/usr/bin/env python3
"""R7 的可跑证据（v7 round-data）——**重构轮**。

R7 是**行为保持型重构**（用户裁决「拆吧」）：`src/ink.rs`(969) 与 `src/path.rs`(793) 拆成目录。
所以本文件不该盯"某个行为"，而该盯**重构的两条底线**：

1. **结构底线**：拆完了（旧的单文件不存在、目录存在）、**门面够薄**、**每个新文件不超几百行**。
2. **行为底线**：拆分**没有改行为** —— 用一组 smoke fixture 断言关键行为仍在（正常 path 含弧 / 畸形 `d` 走 W7 /
   越界走 W6），并且**既有三轮证据（R4/R5/R6）全部仍绿**（"重构不得弄坏前面"）。

（不比对 md5：那是拆分当时的一次性验证——用 `before_bin` 逐字节比对，已由 R7 的执行方与 Orchestrator 各自
用**两组不同**的 fixture 做过，结论一致；写进这里会因为任何一次正当的行为改动而失效。）

契约：`tools/svg_grid/ARCHITECTURE.md` §已定型的决定（体积口径 / 入口薄 / `mod` 不用 `include!`）；`tools/pitfalls-manager/references/02_consume.md` §4.3。
无参数；秒级；离线；不写仓库。
"""
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R7 → PITFALLS → docs → svg_grid
FAILS = []
MAX_FILE_LINES = 350        # 拆分契约：ink/** 与 path/** 每个文件 ≤ 350 行
MAX_MOD_LINES = 110         # 门面（mod.rs）要薄

SVG_H = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">'
BOX = '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:800])
    if not ok:
        FAILS.append(label)


def find_bin():
    for rel in ("target/debug/svg_grid", "target/release/svg_grid"):
        if (SVG_GRID / rel).exists():
            return SVG_GRID / rel
    return None


def main():
    # ── 1. 结构底线
    old = [SVG_GRID / "src/ink.rs", SVG_GRID / "src/path.rs"]
    check(all(not p.exists() for p in old),
          "结构 旧的 src/ink.rs 与 src/path.rs 已不存在（拆成目录）",
          f"{[str(p) for p in old if p.exists()]}")
    for d in ("src/ink", "src/path"):
        mod = SVG_GRID / d / "mod.rs"
        body = mod.read_text() if mod.exists() else ""
        n = len(body.splitlines())
        has_fn = bool(re.search(r"^\s*(pub\s+)?fn\s", body, re.M))
        check(mod.exists() and n <= MAX_MOD_LINES and not has_fn,
              f"结构 {d}/mod.rs 存在且够薄（{n} 行 ≤ {MAX_MOD_LINES}、不含 fn 实现）",
              body[:400])

    rs = sorted((SVG_GRID / "src").rglob("*.rs"))
    over = [(str(p.relative_to(SVG_GRID)), len(p.read_text().splitlines()))
            for p in rs if "/ink/" in str(p) or "/path/" in str(p)
            if len(p.read_text().splitlines()) > MAX_FILE_LINES]
    check(not over, f"结构 ink/** 与 path/** 每个文件 ≤ {MAX_FILE_LINES} 行", f"{over}")

    # ── 2. 行为底线（smoke）
    binary = find_bin()
    if binary is None:
        print("SKIP  行为 smoke：没找到 svg_grid 二进制 —— 先 `cd tools/svg_grid && cargo build --offline`")
    else:
        import tempfile
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="r7-rounddata-"))
        cases = {
            "arc": ('<g data-scale="xy"><path d="M 10 10 A 30 10 45 0 1 150 120" fill="none" stroke="#000"/></g>', 0, None),
            "bad_d": ('<path d="M 10 20 30" fill="none" stroke="#000"/>', 0, "could not be rewritten"),
            "overflow": ('<text x="100" y="240" font-size="8" text-anchor="middle">B</text>', 0, "draws ink outside"),
        }
        for name, (body, want_rc, want) in cases.items():
            svg = tmp / f"{name}.svg"
            svg.write_text(SVG_H + BOX + body + "</svg>")
            p = subprocess.run([str(binary), "plot-grid", "--input", str(svg),
                                "--output", str(tmp / f"{name}.out.svg"),
                                "--ncol", "1", "--plot-margin", "0", "--width", "200", "--height", "200"],
                               capture_output=True, text=True)
            ok = p.returncode == want_rc and (want is None or want in p.stderr)
            check(ok, f"行为 smoke `{name}` ⇒ rc={want_rc} 且 stderr 含 {want or '（不检）'}",
                  f"rc={p.returncode} stderr={p.stderr[:300]!r}")

    # ── 3. 重构不得弄坏前面：既有三轮证据必须仍绿
    for r in ("R4", "R5", "R6"):
        t = SVG_GRID / "docs/PITFALLS/round-data" / r / "test.py"
        if not t.exists():
            print(f"SKIP  {r} 的证据不在")
            continue
        p = subprocess.run([sys.executable, str(t)], capture_output=True, text=True)
        check(p.returncode == 0, f"回归 既有证据 round-data/{r} 仍全绿（重构不得弄坏前面）",
              (p.stdout[-400:] + p.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
