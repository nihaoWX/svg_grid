#!/usr/bin/env python3
"""R30 的可跑证据 —— **默认网格改为 cowplot 式近方形**。

不给 `--ncol/--nrow` 时：`cols = ceil(sqrt(n))`、`rows = ceil(n / cols)`，n 是输入槽位数
（含显式 `NULL`），末行可以不满，缺失的尾槽不生成 cell。断言五族：

1. 1..17 的映射：canvas 高、每格 w×h、cell 数量逐条对表。
2. 5 图默认（2 行 3+2）：canvas 1000×533.333，全格 333.333×266.667，行 y=0/266.667。
3. 15 图默认（4×4）：canvas 1000×800，全格 250×200，行 y=0/200/400/600。
4. 单图/双图默认与旧的一行写法（`--ncol 1` / `--ncol 2`）逐字节相同。
5. 显式 `--ncol/--nrow` 的既有行为不变；`--input none` 占一个槽位；`--plan` 与真跑同值。

无参数；秒级；离线；只写临时目录。找不到二进制时打 SKIP 并以 1 退出（证据未跑）。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]  # round-data/R30 → PITFALLS → docs → svg_grid
BIN = next(
    (REPO / rel for rel in ("target/release/svg_grid", "target/debug/svg_grid")
     if (REPO / rel).exists()),
    None,
)
FAILS = []

SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">'
    '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>'
    '<g data-scale="position"><circle cx="50" cy="40" r="3" fill="#000000"/></g>'
    "</svg>"
)


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def run(args):
    return subprocess.run([str(BIN), *args], capture_output=True, text=True)


def close(actual, expected, tol=1e-2):
    return abs(float(actual) - float(expected)) <= tol


def plan(inputs, extra, tmp, name="plan"):
    args = ["plot-grid"]
    for path in inputs:
        args += ["--input", str(path)]
    args += ["--plan", "--width", "1000", "--plot-margin", "0", *extra]
    proc = run(args)
    if proc.returncode != 0:
        raise RuntimeError(f"plan failed rc={proc.returncode}: {proc.stderr[:500]}")
    canvas = re.search(r"^canvas ([0-9.]+)x([0-9.]+)$", proc.stdout, re.M)
    cells = re.findall(r"^#\d+ .*?cell ([0-9.]+)x([0-9.]+)", proc.stdout, re.M)
    if canvas is None or len(cells) == 0:
        raise RuntimeError(f"unparseable plan output:\n{proc.stdout[:800]}")
    return (float(canvas.group(1)), float(canvas.group(2))), [
        (float(w), float(h)) for w, h in cells
    ]


def compose(inputs, extra, tmp, name):
    out = tmp / name
    args = ["plot-grid"]
    for path in inputs:
        args += ["--input", str(path)]
    args += ["--output", str(out), "--width", "1000", "--plot-margin", "0", *extra]
    proc = run(args)
    if proc.returncode != 0:
        raise RuntimeError(f"plot failed rc={proc.returncode}: {proc.stderr[:500]}")
    return out.read_text()


def cell_origins(text):
    found = re.findall(
        r'data-svg-grid-cell="(\d+)">\s*<rect data-panel-box="main" '
        r'x="([0-9.eE+-]+)" y="([0-9.eE+-]+)"',
        text,
    )
    return {int(i): (float(x), float(y)) for i, x, y in found}


def expect_cells(actual, expected, label):
    ok = len(actual) == len(expected) and all(
        close(w, ew) and close(h, eh) for (w, h), (ew, eh) in zip(actual, expected)
    )
    check(ok, label, f"actual={actual} expected={expected}")


def main():
    if BIN is None:
        print("SKIP  no svg_grid binary (cargo build --offline first)")
        return 1

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r30-rounddata-"))
    inputs = []
    for index in range(17):
        path = tmp / f"p{index}.svg"
        path.write_text(SVG)
        inputs.append(path)

    # ── 0. 显式网格的既有行为不变（D2 的行高语义）
    for extra, canvas_h in (("--ncol 2", 1200), ("--nrow 2", 533.333)):
        canvas, cells = plan(inputs[:5], extra.split(), tmp)
        check(close(canvas[0], 1000) and close(canvas[1], canvas_h)
              and len(cells) == 5,
              f"5 图 {extra} 的显式网格仍不变（canvas 高 {canvas_h}）",
              f"canvas={canvas} cells={cells[:2]}")
    canvas, cells = plan(inputs[:15], ["--ncol", "4"], tmp)
    check(close(canvas[1], 800) and len(cells) == 15
          and all(close(w, 250) and close(h, 200) for w, h in cells),
          "15 图 --ncol 4 仍是 4×4（canvas 高 800、格 250×200）",
          f"canvas={canvas} cells={cells[:2]}")

    # ── 1. 1..17 映射表
    #    n → (rows, cols); 1000 宽、aspect 1.25 ⇒ cell_w = 1000/cols,
    #    cell_h = cell_w/1.25, canvas_h = rows*cell_h.
    table = {
        1: (1, 1), 2: (1, 2), 3: (2, 2), 4: (2, 2), 5: (2, 3), 6: (2, 3),
        7: (3, 3), 8: (3, 3), 9: (3, 3), 10: (3, 4), 11: (3, 4), 12: (3, 4),
        13: (4, 4), 14: (4, 4), 15: (4, 4), 16: (4, 4), 17: (4, 5),
    }
    for n, (rows, cols) in table.items():
        cell = (1000.0 / cols, (1000.0 / cols) / 1.25)
        canvas, cells = plan(inputs[:n], [], tmp)
        expect_cells(cells, [cell] * n, f"{n} 图默认 ⇒ {rows}×{cols}、每格 {cell[0]:.3f}×{cell[1]:.3f}")
        check(close(canvas[0], 1000) and close(canvas[1], rows * cell[1])
              and len(cells) == n,
              f"{n} 图默认 ⇒ canvas 1000×{rows * cell[1]:.3f}，{n} 个 cell",
              f"canvas={canvas} cells={len(cells)}")

    # ── 2. 5 图默认 = 2 行 3+2，行 y=0/266.667
    text = compose(inputs[:5], [], tmp, "five.svg")
    origins = cell_origins(text)
    expect = {
        0: (0, 0), 1: (333.333, 0), 2: (666.667, 0),
        3: (0, 266.667), 4: (333.333, 266.667),
    }
    check(all(close(origins.get(i, (-1, -1))[0], x)
              and close(origins.get(i, (-1, -1))[1], y)
              for i, (x, y) in expect.items())
          and len(origins) == 5,
          "5 图默认 ⇒ 行 y=0/266.667，末行 2 格 x=0/333.333",
          f"origins={origins}")

    # ── 3. 15 图默认 = 4×4，全格 250×200
    text = compose(inputs[:15], [], tmp, "fifteen.svg")
    origins = cell_origins(text)
    check(len(origins) == 15
          and all(close(origins[i][0], (i % 4) * 250)
                  and close(origins[i][1], (i // 4) * 200) for i in range(15)),
          "15 图默认 ⇒ 4×4，行 y=0/200/400/600，列 x=0/250/500/750",
          f"origins={origins}")

    # ── 4. 单图/双图默认与旧的一行写法逐字节相同
    for n, explicit in ((1, "--ncol 1"), (2, "--ncol 2")):
        auto = compose(inputs[:n], [], tmp, f"auto{n}.svg")
        old = compose(inputs[:n], explicit.split(), tmp, f"old{n}.svg")
        check(auto == old, f"{n} 图默认与 {explicit} 逐字节相同")

    # ── 5. NULL 占一个槽位（4 图 + NULL ⇒ 5 槽 ⇒ 2×3，而不是 4 槽 ⇒ 2×2）
    canvas, cells = plan(inputs[:4] + ["none"], [], tmp)
    check(close(canvas[1], 533.333) and len(cells) == 5,
          "4 图 + 显式 NULL ⇒ 按 5 个槽位排 2×3（canvas 高 533.333）",
          f"canvas={canvas} cells={len(cells)}")

    # ── 6. --plan 与真跑同源：5 图默认的 canvas 高度与 cell 数一致
    canvas, cells = plan(inputs[:5], [], tmp)
    text = compose(inputs[:5], [], tmp, "five_again.svg")
    height = re.search(r'<svg width="1000" height="([0-9.]+)"', text)
    check(height is not None and close(height.group(1), canvas[1])
          and len(cells) == 5 and text.count("data-svg-grid-cell=") == 5,
          "--plan 与真跑同值：canvas 高与 5 个 cell 一致",
          f"plan={canvas} output_height={height.group(1) if height else None}")

    print()
    if FAILS:
        print(f"== {len(FAILS)} FAIL ==")
        for label in FAILS:
            print("  -", label)
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
