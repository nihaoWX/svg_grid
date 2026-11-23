#!/usr/bin/env python3
"""R28 —— D1 评审三项：自我检查的 provenance、拆分填充去净 stroke paint、`--report` 计 `<path>`。

三族判据（黑盒，只调 CLI）：
  ① provenance：`<filter>` 里的 `<path d='M 0 0 L 10 10'>`（未翻译、非展平形态）仍判残渣，rc=1 且无产物；
     合法的展平填充路径（②③）仍放行。
     R37 起 mask/marker 的内容会被翻译（bake 递归进去），所以原来「mask 内同形 path 判残渣」的
     判据已被取代——provenance 的边界从「mask/marker 容器」收窄到「filter 容器」。
  ② 拆分：填充+描边三分支产出的填充 `<path>` 只有 `stroke="none"`，没有任何 `stroke-*` 属性或 style 声明；
  ③ 计数：`convert --report` 的 primitives 里 `path=1` 且不出现 `polygon=`。

无参；秒级。只读：产物只写系统临时目录。
"""
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
DATA = HERE / "data"
PROJECT = HERE.parents[3]              # round-data/R28 → PITFALLS → docs → svg_grid
FAILS = []


def binary():
    for candidate in (
        os.environ.get("SVG_GRID_BIN"),
        shutil.which("svg_grid"),
        PROJECT / "target/release/svg_grid",
        PROJECT / "target/debug/svg_grid",
    ):
        if candidate and pathlib.Path(candidate).exists():
            return str(candidate)
    return None


def convert(binary_path, source, output, *extra):
    return subprocess.run(
        [binary_path, "convert", "--input", str(source), "--output", str(output), *extra],
        capture_output=True, text=True, timeout=60, cwd="/tmp",
    )


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:1500])
    if not ok:
        FAILS.append(label)


def main():
    binary_path = binary()
    if not binary_path:
        print("SKIP  all — no svg_grid binary (run `cargo build --release`)")
        return 0
    with tempfile.TemporaryDirectory() as scratch:
        tmp = pathlib.Path(scratch)

        # ① filter 内的未翻译 path 仍是残渣（R37 起 mask/marker 内容会被翻译）。
        out = tmp / "filter.svg"
        done = convert(binary_path, DATA / "filter_path.svg", out)
        produced = out.read_text() if out.exists() else None
        check(
            done.returncode == 1
            and "untranslated dialect nodes" in done.stderr
            and "path" in done.stderr
            and produced is None,
            "filter 内未翻译的 <path> 仍判残渣（rc=1、无产物）",
            f"rc={done.returncode}\nstderr={done.stderr}\nproduced={produced!r}",
        )

        # ② 拆分出的填充 path 不带任何 stroke paint。
        out = tmp / "split.svg"
        done = convert(binary_path, DATA / "split_fill.svg", out)
        svg = out.read_text() if out.exists() else ""
        paths = re.findall(r"<path\b[^>]*>", svg)
        fill = paths[0] if len(paths) == 1 else ""
        stroke_attrs = re.findall(r'\s(stroke(?:-[a-z-]+)?)=', fill)
        style = re.search(r'\sstyle="([^"]*)"', fill)
        style_text = style.group(1) if style else ""
        check(
            done.returncode == 0
            and len(paths) == 1
            and 'stroke="none"' in fill
            and all(name == "stroke" for name in stroke_attrs)
            and "stroke" not in style_text,
            '拆分的填充 <path> 只有 stroke="none"，无 stroke-* 属性/style',
            f"rc={done.returncode}\npath={fill}\nstyle={style_text}",
        )

        # ③ --report 计入展平 path。
        done = convert(binary_path, DATA / "three_subpaths.svg", tmp / "three.svg", "--report")
        primitives = next(
            (line for line in done.stdout.splitlines() if "primitives:" in line), ""
        )
        check(
            done.returncode == 0 and "path=1" in primitives and "polygon=" not in primitives,
            "convert --report 把展平填充 <path> 计入 primitives",
            f"rc={done.returncode}\nstdout={done.stdout}\nstderr={done.stderr}",
        )

    print(f"R28: {len(FAILS)} failure(s), {3 - len(FAILS)} pass")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
