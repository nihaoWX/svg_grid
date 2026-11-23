#!/usr/bin/env python3
"""R42 证据：`convert/` crate 并入主库之后，"只有一个二进制" 的事实核对。

R42 是搬迁轮（`convert/` → `src/convert/`，语义零变化）。它没有可渲染的旧缺陷，
可验证的是搬迁后的**结构事实**——这正是最容易"只在文档里声称"的部分：

* A 目录/构建：`convert/` 目录不存在、`src/convert/mod.rs` 存在、`Cargo.toml` 没有
  `svg_grid_convert` 依赖也没有独立 `[[bin]]`（隐式 target 只有 `src/main.rs` 的
  `svg_grid`），`target/` 下没有残留的 `svg_grid_convert` 可执行文件；
* B 子命令可用：`svg_grid convert` 能跑通一次真实转换（`--check`）；
* C 名字收敛：`--help` 里只有 `svg_grid convert --input`，没有独立二进制的名字；
* D 诊断前缀：失败行是 `svg_grid convert: ...`；
* E 残余引用：`git grep svg_grid_convert` 的命中只能是"历史说明/证据文本/stale 清理"，
  不允许再出现在可执行的代码路径上（doc 注释允许）。

退出码：0 = 全部成立；1 = 有 FAIL。无参数、离线、不写仓库，只读文件 + 跑 CLI。
"""

import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(HERE))))
BIN = os.environ.get("SVG_GRID_BIN") or os.path.join(ROOT, "target", "debug", "svg_grid")
FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        if detail:
            print("      " + detail)
        FAILS.append(label)
    return ok


def read(path):
    with open(path, "r", encoding="utf-8") as handle:
        return handle.read()


def run(args):
    return subprocess.run(args, capture_output=True, text=True)


def family_a_layout():
    check(not os.path.isdir(os.path.join(ROOT, "convert")), "A1 the convert/ crate directory is gone")
    check(os.path.isfile(os.path.join(ROOT, "src", "convert", "mod.rs")), "A2 src/convert/mod.rs exists")
    manifest = read(os.path.join(ROOT, "Cargo.toml"))
    check("svg_grid_convert" not in manifest, "A3 Cargo.toml has no svg_grid_convert dependency")
    check("[[bin]]" not in manifest, "A4 no explicit [[bin]]: the only target is src/main.rs")
    stray = []
    for profile in ("debug", "release"):
        for name in os.listdir(os.path.join(ROOT, "target", profile)) if os.path.isdir(
            os.path.join(ROOT, "target", profile)
        ) else []:
            if name == "svg_grid_convert" or name.startswith("svg_grid_convert."):
                stray.append(os.path.join(profile, name))
    check(not stray, "A5 no svg_grid_convert executable under target/", str(stray))


def family_b_subcommand():
    doc = os.path.join(tempfile.mkdtemp(prefix="svg-grid-r42-"), "panel.svg")
    with open(doc, "w", encoding="utf-8") as handle:
        handle.write(
            '<svg width="100" height="80" viewBox="0 0 100 80" xmlns="http://www.w3.org/2000/svg">'
            '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none" stroke="none"/>'
            '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="#3366cc"/></g></svg>'
        )
    done = run([BIN, "convert", "--input", doc, "--check"])
    check(done.returncode == 0, "B1 svg_grid convert --check accepts a tagged panel", done.stderr.strip()[-300:])


def family_c_names():
    help_text = run([BIN, "--help"])
    check(help_text.returncode == 0, "C1 svg_grid --help succeeds")
    check("svg_grid convert --input" in help_text.stdout, "C2 --help documents the convert subcommand")
    check("svg_grid_convert" not in help_text.stdout, "C3 --help never names the retired binary")
    convert_help = run([BIN, "convert", "--help"])
    check(convert_help.returncode == 0, "C4 svg_grid convert --help succeeds")
    check("svg_grid_convert" not in convert_help.stdout + convert_help.stderr, "C5 the help never names the retired binary")


def family_d_prefix():
    done = run([BIN, "convert", "--input", os.path.join(ROOT, "no-such-panel-r42.svg"), "--output", "out.svg"])
    check(done.returncode != 0, "D1 a missing input fails closed")
    check(
        done.stderr.startswith("svg_grid convert: "),
        "D2 the diagnostic is prefixed with the subcommand path",
        done.stderr.strip()[:200],
    )
    check("svg_grid_convert:" not in done.stderr, "D3 no diagnostic uses the retired prefix")


ALLOWED_FILES = {
    "Cargo.toml",
    "PROGRESS.md",
    "docs/HANDOFF.md",
    "install.sh",
}
ALLOWED_PREFIXES = ("docs/PITFALLS/", "ARCHITECTURE/")


def family_e_residue():
    done = run(["git", "grep", "-n", "svg_grid_convert"])
    if done.returncode != 0:
        check(False, "E1 git grep works (run inside the repository)", done.stderr.strip())
        return
    hits = []
    for line in done.stdout.splitlines():
        path, _, rest = line.partition(":")
        _, _, text = rest.partition(":")
        hits.append((path, text))
    bad = []
    for path, text in hits:
        if path in ALLOWED_FILES or path.startswith(ALLOWED_PREFIXES):
            continue
        if path.endswith(".stderr.log"):
            continue
        if text.lstrip().startswith(("//", "/*", "*", "#")):
            # Doc comments and comments explain the history; they cannot execute.
            continue
        bad.append(f"{path}: {text.strip()[:80]}")
    check(not bad, "E1 every surviving reference is historical text or stale cleanup", "\n      ".join(bad[:8]))
    check(len(hits) > 0, "E2 the grep still finds the historical text it is supposed to keep", str(hits[:3]))


def main():
    if not os.path.isfile(BIN):
        print(f"FAIL  binary not found: {BIN} (run `cargo build --offline`)")
        return 1
    family_a_layout()
    family_b_subcommand()
    family_c_names()
    family_d_prefix()
    family_e_residue()
    print()
    if FAILS:
        print(f"== {len(FAILS)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
