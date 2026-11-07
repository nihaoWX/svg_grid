#!/usr/bin/env python3
"""R8 的可跑证据（v7 round-data）——**体积拆分轮**。

用户裁决「那些大于 20kb 的都拆了吧」。本文件把这一轮的**两条底线**钉住：

1. **体积**：`src/`、`convert/src/`、`tests/`、`scripts/` 下的每个 `.rs`/`.py` **≤ 20000 B**（口径从严格字节）。
2. **不脱离模块树**：`src/**` 与 `tests/compose_tests/**` 里**不许出现 `include!(`**，且测试目录里每个
   非 `main.rs` 的 `.rs` 都必须被 `main.rs` 用 `mod <名字>;` 声明 —— 这一条是 R8 用血换来的：
   `include!` 进来的文件 **rustfmt 看不见**（我实测：塞一行丑代码 `cargo fmt --check` 仍 rc=0），
   会让 fmt 门禁**谎报干净**。用 `mod` 时 rustfmt 会跟进整棵模块树。

外加一条行为 smoke 与"既有四轮证据全绿"。

契约：`tools/svg_grid/ARCHITECTURE.md` §已定型的决定（体积口径 / 入口薄 / `mod` 不用 `include!`）；`tools/pitfalls-manager/references/02_consume.md` §4.3。
无参数；秒级；离线；不写仓库。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                      # round-data/R8 → PITFALLS → docs → svg_grid
LIMIT = 20000
ENTRY_LIMIT = 4500     # 入口/门面要薄
FAILS = []

SVG_H = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">'
BOX = '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:800])
    if not ok:
        FAILS.append(label)


def sources():
    out = []
    for base in ("src", "convert/src", "tests", "scripts"):
        for p in (SVG_GRID / base).rglob("*"):
            if p.suffix in (".rs", ".py") and "__pycache__" not in p.parts:
                out.append(p)
    return out


def main():
    # ── 1. 体积
    over = [(str(p.relative_to(SVG_GRID)), p.stat().st_size)
            for p in sources() if p.stat().st_size > LIMIT]
    check(not over, f"体积 所有源文件 ≤ {LIMIT} B（{len(sources())} 个文件）", f"{over}")
    biggest = sorted(((p.stat().st_size, str(p.relative_to(SVG_GRID))) for p in sources()), reverse=True)[:3]
    print("      最大三个: " + ", ".join(f"{n} {s}B" for s, n in biggest))

    # ── 2. 不脱离模块树（R8 的教训：include! 会让 rustfmt 看不见）
    inc = [str(p.relative_to(SVG_GRID)) for p in sources()
           if p.suffix == ".rs" and "include!(" in p.read_text()]
    check(not inc, "模块树 源码里没有 `include!(`（否则 rustfmt 看不见那些文件）", f"{inc}")

    tdir = SVG_GRID / "tests/compose_tests"
    if tdir.is_dir():
        main_rs = (tdir / "main.rs").read_text()
        orphans = [p.name for p in sorted(tdir.glob("*.rs"))
                   if p.name != "main.rs" and not re.search(rf"^\s*mod\s+{p.stem}\s*;", main_rs, re.M)]
        check(not orphans, "模块树 tests/compose_tests 下每个 .rs 都被 main.rs 的 `mod` 声明（rustfmt 跟得进去）",
              f"孤儿文件: {orphans}")
        check((tdir / "main.rs").stat().st_size <= ENTRY_LIMIT,
              f"门面 tests/compose_tests/main.rs ≤ {ENTRY_LIMIT} B",
              f"{(tdir / 'main.rs').stat().st_size} B")

    # ── 3. 入口保持薄：判据脚本仍是薄壳（判据在 deliverable_checks 包里）
    shell = SVG_GRID / "scripts/check_deliverable.py"
    body = shell.read_text()
    check(shell.stat().st_size <= ENTRY_LIMIT and "from deliverable_checks import" in body
          and "def check_dead_band" not in body,
          f"门面 scripts/check_deliverable.py ≤ {ENTRY_LIMIT} B 且只做再导出（判据实现不在入口里）",
          f"{shell.stat().st_size} B")

    # ── 4. 行为 smoke
    binary = next((SVG_GRID / r for r in ("target/debug/svg_grid", "target/release/svg_grid")
                   if (SVG_GRID / r).exists()), None)
    if binary is None:
        print("SKIP  smoke：没找到 svg_grid 二进制（先 cargo build --offline）")
    else:
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="r8-rounddata-"))
        for name, body_txt, want in (
            ("clean", '<rect x="10" y="10" width="50" height="50" fill="#000"/>', None),
            ("overflow", '<text x="100" y="240" font-size="8" text-anchor="middle">B</text>', "draws ink outside"),
            ("lenient", '<path d="M 10 20 30" fill="none" stroke="#000"/>', "could not be rewritten"),
        ):
            svg = tmp / f"{name}.svg"
            svg.write_text(SVG_H + BOX + body_txt + "</svg>")
            p = subprocess.run([str(binary), "plot-grid", "--input", str(svg),
                                "--output", str(tmp / f"{name}.out.svg"),
                                "--ncol", "1", "--plot-margin", "0", "--width", "200", "--height", "200"],
                               capture_output=True, text=True)
            ok = p.returncode == 0 and (want is None or want in p.stderr)
            check(ok, f"smoke `{name}` ⇒ rc=0 且 stderr {'含 ' + want if want else '为空（不误报）'}",
                  f"rc={p.returncode} stderr={p.stderr[:250]!r}")

    # ── 5. 既有四轮证据全绿
    for r in ("R4", "R5", "R6", "R7"):
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
