#!/usr/bin/env python3
"""R29 的可跑证据 —— **短末行不再被拉伸**（行高只由网格的列权重决定）。

本轮把 `src/layout.rs` 的 `compute_grid` 行高信号从「本行实际图数的 aspect 和」改成
「全网格列权重和」，四族断言：

1. 5 图 `--ncol 2`（3 行 2+2+1）：canvas 1000×1200，5 格全 500×400；合成图里行 y=0/400/800。
2. 5 图 `--ncol 3`（2 行 3+2）与 `--nrow 2`（推导 cols=3）：canvas 1000×533.333，
   全格 333.333×266.667；第二行 y=266.667，末两格 x=0/333.333。
3. 满行网格几何不变：4/2×2、6/2×3、9/3×3、16/4×4、15/5×3 的 canvas 与格子尺寸全按旧值。
4. `--rel-*` 只做倍数：`rel-widths 1,2` ≡ `2,4`（canvas/格子完全一致，且 4 图 2×2 仍是
   1000×800、333.333/666.667×400）；5 图 `--ncol 2 --rel-heights 1,1,2` 的末格仍是 500 宽、
   行高 400/400/800。

无参数；秒级；离线；只写临时目录。找不到二进制时打 SKIP 并以 1 退出（证据未跑）。
"""
import pathlib
import re
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]  # round-data/R29 → PITFALLS → docs → svg_grid
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


def plan(inputs, extra):
    args = ["plot-grid"]
    for path in inputs:
        args += ["--input", str(path)]
    args += ["--plan", "--width", "1000", "--plot-margin", "0", *extra]
    proc = run(args)
    if proc.returncode != 0:
        raise RuntimeError(f"plan failed rc={proc.returncode}: {proc.stderr[:500]}")
    canvas = re.search(r"^canvas ([0-9.]+)x([0-9.]+)$", proc.stdout, re.M)
    cells = re.findall(
        r"^#\d+ .*?cell ([0-9.]+)x([0-9.]+)", proc.stdout, re.M
    )
    if canvas is None or len(cells) == 0:
        raise RuntimeError(f"unparseable plan output:\n{proc.stdout[:800]}")
    return (float(canvas.group(1)), float(canvas.group(2))), [
        (float(w), float(h)) for w, h in cells
    ]


def cell_origins(inputs, extra, tmp):
    """Write the composite and read each cell's origin from its panel box.

    The fixtures put `data-panel-box` at (0,0), so the translated panel box in
    the output *is* the cell origin.  (A nonzero panel-box origin would need
    reversing the per-panel scale; not needed for this geometric assertion.)
    """
    out = tmp / "out.svg"
    args = ["plot-grid"]
    for path in inputs:
        args += ["--input", str(path)]
    args += ["--output", str(out), "--width", "1000", "--plot-margin", "0", *extra]
    proc = run(args)
    if proc.returncode != 0:
        raise RuntimeError(f"plot failed rc={proc.returncode}: {proc.stderr[:500]}")
    text = out.read_text()
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

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r29-rounddata-"))
    inputs = []
    for index in range(16):
        path = tmp / f"p{index}.svg"
        path.write_text(SVG)
        inputs.append(path)

    # ── 1. 5 图 / 2 列 ⇒ 3 行 2+2+1，全部 500×400，行 y=0/400/800
    canvas, cells = plan(inputs[:5], ["--ncol", "2"])
    check(close(canvas[0], 1000) and close(canvas[1], 1200),
          "5 图 --ncol 2 ⇒ canvas 1000×1200", f"canvas={canvas}")
    expect_cells(cells, [(500, 400)] * 5, "5 图 --ncol 2 ⇒ 5 格全 500×400")
    origins = cell_origins(inputs[:5], ["--ncol", "2"], tmp)
    check(origins == {0: (0, 0), 1: (500, 0), 2: (0, 400),
                      3: (500, 400), 4: (0, 800)},
          "5 图 --ncol 2 ⇒ 行 y=0/400/800，末格在 (0,800)",
          f"origins={origins}")

    # ── 2. 5 图 / 3 列 ⇒ 2 行 3+2；--nrow 2 推导出同一网格
    for extra, name in ((["--ncol", "3"], "--ncol 3"), (["--nrow", "2"], "--nrow 2")):
        canvas, cells = plan(inputs[:5], extra)
        check(close(canvas[0], 1000) and close(canvas[1], 533.333),
              f"5 图 {name} ⇒ canvas 1000×533.333", f"canvas={canvas}")
        expect_cells(cells, [(333.333, 266.667)] * 5,
                     f"5 图 {name} ⇒ 5 格全 333.333×266.667")
        origins = cell_origins(inputs[:5], extra, tmp)
        check(origins[0][1] == 0 and close(origins[3][1], 266.667)
              and close(origins[4][1], 266.667)
              and close(origins[3][0], 0) and close(origins[4][0], 333.333),
              f"5 图 {name} ⇒ 两行 y=0/266.667，末行 x=0/333.333",
              f"origins={origins}")

    # ── 3. 满行网格几何与旧值一致（1000 宽、1.25 aspect、零 margin/gap）
    for inputs_n, cols, canvas_h, cell in (
        (4, 2, 800, (500, 400)),
        (6, 2, 1200, (500, 400)),
        (9, 3, 800, (333.333, 266.667)),
        (16, 4, 800, (250, 200)),
        (15, 5, 480, (200, 160)),
    ):
        canvas, cells = plan(inputs[:inputs_n], ["--ncol", str(cols)])
        ok = close(canvas[1], canvas_h) and all(
            close(w, cell[0]) and close(h, cell[1]) for w, h in cells
        )
        check(ok, f"满行 {inputs_n} 图 / {cols} 列 ⇒ canvas 1000×{canvas_h}、格 {cell[0]}×{cell[1]}",
              f"canvas={canvas} cells={cells[:3]}")

    # ── 4. rel-* 只做倍数
    canvas_u, unit = plan(inputs[:4], ["--ncol", "2", "--rel-widths", "1,2"])
    canvas_d, doubled = plan(inputs[:4], ["--ncol", "2", "--rel-widths", "2,4"])
    expect_cells(unit, [(333.333, 400), (666.667, 400)] * 2,
                 "4 图 2×2 --rel-widths 1,2 ⇒ 高度仍 400（满行几何不变）")
    check(close(canvas_u[1], 800),
          "4 图 2×2 --rel-widths 1,2 ⇒ canvas 高度仍 800",
          f"canvas={canvas_u}")
    check(all(close(a, b) for ca, cb in zip(unit, doubled) for a, b in zip(ca, cb))
          and close(canvas_u[1], canvas_d[1]),
          "--rel-widths 1,2 与 2,4 是同一张图（倍数只算比值）",
          f"unit={unit} doubled={doubled}")

    canvas, cells = plan(inputs[:5], ["--ncol", "2", "--rel-heights", "1,1,2"])
    check(close(canvas[1], 1600) and close(cells[4][0], 500)
          and close(cells[4][1], 800) and close(cells[0][1], 400),
          "--rel-heights 1,1,2 对 5 图 2 列 ⇒ 行高 400/400/800，末格仍 500 宽",
          f"canvas={canvas} cells={cells}")

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
