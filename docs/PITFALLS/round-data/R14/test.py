#!/usr/bin/env python3
"""R14 的可跑证据 —— **CLI 缺陷批次：零值 panic / 越界 assert / 错误拍平 / 坏 `--help` / 编译期字体路径**。

本轮把六族"用户输入变成 panic 或变成错误码 1"的缺陷钉死（契约见 `ARCHITECTURE/cli.md`）：

1. **零值不再 panic**：`--ncol 0` / `--nrow 0` ⇒ 退出码 **2**（不是 101），且 stderr 说明是哪个 flag。
2. **越界不再 panic**：`--avif-quality 0` / `101` / `-1` ⇒ 退出码 **2**，不产出 AVIF，也不断言失败。
3. **错误分档**：0 成功 / 1 运行期（IO）/ 2 用法或输入 / 3 配置错（空字体库）——不再是"一律 1"。
4. **`--help` 可用**：`-h` / `--help` / `plot-grid --help` ⇒ **stdout** 打完整帮助、退出 **0**、stderr 空；
   未知 flag 与缺 `--output` ⇒ 一句话 + 指向 `--help`、退出 **2**；`--version` 打版本、退出 **0**。
   帮助文本**覆盖全部 flag**（flag 清单从 `src/cli/parse.rs` 的 `FLAGS` 注册表读，单一来源）。
5. **字体目录在运行时解析**：`--font-dir` > `$SVG_GRID_FONT_DIR` > `$HOME/svg_grid/fontlib`；
   二进制里**没有**构建机的字体路径（`<checkout>fontlib` 为空），把二进制拷到别处、`HOME` 另指也读对目录。
6. **回归**：R4–R13 全绿（含 R13：字体来源唯一仍成立）。

无参数；秒级；离线；只读仓库（fixture 与字体库副本站时目录）。跳过一律打印原因。
"""
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                  # round-data/R14 → PITFALLS → docs → svg_grid
BIN = next((SVG_GRID / rel for rel in ("target/debug/svg_grid", "target/release/svg_grid")
            if (SVG_GRID / rel).exists()), None)
FAILS = []

SVG = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">'
       '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'
       '<rect x="20" y="20" width="60" height="60" fill="#000"/></svg>')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def run(args, env=None, cwd=None, binary=None):
    return subprocess.run([str(binary or BIN), *args], capture_output=True, text=True,
                          env=env, cwd=cwd)


def main():
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r14-rounddata-"))
    panel = tmp / "panel.svg"
    panel.write_text(SVG)
    out = tmp / "out.svg"

    # ── 字体库材料：挑一个真有字体的候选目录，副本放进临时目录（不动仓库）
    candidates = []
    if os.environ.get("SVG_GRID_FONT_DIR"):
        candidates.append(pathlib.Path(os.environ["SVG_GRID_FONT_DIR"]))
    candidates += [pathlib.Path.home() / "svg_grid" / "fontlib", SVG_GRID / "fontlib"]
    src_lib = next((cand for cand in candidates
                    if cand.is_dir()
                    and (list(cand.glob("*.tt[cf]")) + list(cand.glob("*.otf")))), None)
    good_lib = tmp / "fonts-good"
    empty_lib = tmp / "fonts-empty"
    empty_lib.mkdir()
    if src_lib is not None:
        shutil.copytree(src_lib, good_lib)
        print(f"      字体库材料: {src_lib} → {good_lib}")

    if BIN is None:
        print("SKIP  全部行为断言：没有 svg_grid 二进制（先 cd tools/svg_grid && cargo build --offline）")
        return 1

    # ── 1. 零值不再 panic
    for flag in ("--ncol", "--nrow"):
        p = run(["plot-grid", "--input", str(panel), "--output", str(out), flag, "0"])
        check(p.returncode == 2 and "panicked" not in p.stderr and flag in p.stderr,
              f"零值 {flag} 0 ⇒ 退出 2（不是 101）、stderr 点名该 flag",
              f"rc={p.returncode} stderr={p.stderr[:300]!r}")

    # ── 2. 越界不再 panic（不需要真出 AVIF）
    avif = tmp / "out.avif"
    for value in ("0", "101", "-1", "nan"):
        p = run(["plot-grid", "--input", str(panel), "--output", str(out),
                 "--output-avif", str(avif), "--avif-quality", value])
        check(p.returncode == 2 and not avif.exists() and "panicked" not in p.stderr,
              f"越界 --avif-quality {value} ⇒ 退出 2、不产 AVIF、不 panic",
              f"rc={p.returncode} exists={avif.exists()} stderr={p.stderr[:300]!r}")
        if avif.exists():
            avif.unlink()

    # ── 3. --help / --version
    help_out = ""
    for args in (["--help"], ["-h"], ["plot-grid", "--help"], ["plot-grid", "-h"]):
        p = run(args)
        help_out = help_out or p.stdout
        check(p.returncode == 0 and p.stdout.strip() and p.stderr == "",
              f"`{' '.join(args)}` ⇒ 退出 0、stdout 非空、stderr 空",
              f"rc={p.returncode} stdout={len(p.stdout)}B stderr={p.stderr[:200]!r}")
    p = run(["--version"])
    check(p.returncode == 0 and p.stdout.strip() and p.stderr == "",
          "`--version` ⇒ 退出 0、stdout 打版本", f"rc={p.returncode} {p.stdout[:80]!r}")

    # 帮助文本覆盖解析器认识的每个 flag（清单从 src/cli/parse.rs 的 FLAGS 读）
    parse_rs = (SVG_GRID / "src/cli/parse.rs").read_text()
    block = parse_rs.split("pub(crate) const FLAGS", 1)[1].split("\n];", 1)[0]
    flags = re.findall(r'\(\s*"(--[a-z0-9-]+)"', block)
    missing = [flag for flag in flags if flag not in help_out]
    check(len(flags) >= 25 and not missing,
          f"帮助文本覆盖全部 flag（FLAGS 注册表 {len(flags)} 个）", f"缺: {missing}")
    check("--help" in help_out and "--version" in help_out and "exit codes" in help_out,
          "帮助文本含 `--help` / `--version` / 退出码分档")

    # ── 4. 未知 flag / 缺 --output / 未知子命令
    for args, label in (
        (["plot-grid", "--input", str(panel), "--output", str(out), "--bogus", "1"],
         "未知 flag ⇒ 退出 2 + 指向 --help"),
        (["plot-grid", "--input", str(panel)], "缺 --output ⇒ 退出 2 + 指向 --help"),
        (["no-such-subcommand"], "未知子命令 ⇒ 退出 2 + 指向 --help"),
    ):
        p = run(args)
        check(p.returncode == 2 and "--help" in p.stderr and len(p.stderr.strip().splitlines()) <= 2,
              label, f"rc={p.returncode} stderr={p.stderr[:300]!r}")

    # ── 5. 退出码分档 0/1/2/3
    p = run(["plot-grid", "--input", str(panel), "--output", str(out)])
    check(p.returncode == 0 and out.exists(), "分档 0：正常组图", f"rc={p.returncode} {p.stderr[:200]!r}")
    p = run(["plot-grid", "--input", str(tmp / "no-such.svg"), "--output", str(out)])
    check(p.returncode == 1, "分档 1：读不到输入文件（IO）", f"rc={p.returncode} {p.stderr[:200]!r}")
    bad = tmp / "no-tag.svg"
    bad.write_text('<svg width="10" height="10"><rect x="0" y="0" width="10" height="10"/></svg>')
    p = run(["plot-grid", "--input", str(bad), "--output", str(out)])
    check(p.returncode == 2, "分档 2：输入 SVG 结构坏（没有 data-panel-box）",
          f"rc={p.returncode} {p.stderr[:200]!r}")
    p = run(["plot-grid", "--input", str(panel), "--output", str(out),
             "--output-png", str(tmp / "o.png"), "--font-dir", str(empty_lib)])
    check(p.returncode == 3 and "font library" in p.stderr and not (tmp / "o.png").exists(),
          "分档 3：空字体库 + 要出 PNG ⇒ 退出 3（配置错），不产 PNG",
          f"rc={p.returncode} stderr={p.stderr[:300]!r}")

    # ── 6. 字体目录解析顺序 + 无编译期路径
    if src_lib is None:
        print("SKIP  字体目录解析断言：本机没有可复制的字体库（跑一次 fontlib/setup.sh）—— 原因已打印")
    else:
        base_env = {k: v for k, v in os.environ.items() if k != "SVG_GRID_FONT_DIR"}

        env_lib = dict(base_env, SVG_GRID_FONT_DIR=str(good_lib))
        p = run(["plot-grid", "--list-fonts"], env=env_lib, cwd="/tmp")
        check(p.returncode == 0 and p.stdout.strip(),
              "$SVG_GRID_FONT_DIR 生效：`--list-fonts` 列出该目录里的族（cwd=/tmp）",
              f"rc={p.returncode} stdout={p.stdout[:200]!r} stderr={p.stderr[:200]!r}")

        env_empty = dict(base_env, SVG_GRID_FONT_DIR=str(empty_lib))
        p = run(["plot-grid", "--list-fonts"], env=env_empty, cwd="/tmp")
        check(p.returncode == 3, "$SVG_GRID_FONT_DIR 指向空目录 ⇒ 退出 3（真的读了这个变量）",
              f"rc={p.returncode} stderr={p.stderr[:200]!r}")

        p = run(["plot-grid", "--list-fonts", "--font-dir", str(good_lib)], env=env_empty, cwd="/tmp")
        check(p.returncode == 0 and p.stdout.strip(),
              "--font-dir 优先于 SVG_GRID_FONT_DIR（env 空、flag 好 ⇒ 成功）",
              f"rc={p.returncode} stdout={p.stdout[:200]!r} stderr={p.stderr[:200]!r}")
        p = run(["plot-grid", "--list-fonts", "--font-dir", str(empty_lib)], env=env_lib, cwd="/tmp")
        check(p.returncode == 3,
              "--font-dir 优先于 SVG_GRID_FONT_DIR（env 好、flag 空 ⇒ 退出 3）",
              f"rc={p.returncode} stderr={p.stderr[:200]!r}")

        # 数据根：换 HOME、不设 env、二进制拷到别处，仍读 $HOME/svg_grid/fontlib
        copydir = tmp / "elsewhere"
        copydir.mkdir()
        copied = copydir / "svg_grid"
        shutil.copy2(BIN, copied)
        home = tmp / "home"
        (home / "svg_grid").mkdir(parents=True)
        shutil.copytree(good_lib, home / "svg_grid" / "fontlib")
        p = run(["plot-grid", "--list-fonts"], env=dict(base_env, HOME=str(home)), cwd="/tmp",
                binary=copied)
        check(p.returncode == 0 and p.stdout.strip(),
              "数据根生效：二进制拷到别处 + HOME 另指 ⇒ 读 $HOME/svg_grid/fontlib",
              f"rc={p.returncode} stdout={p.stdout[:200]!r} stderr={p.stderr[:300]!r}")
        home_empty = tmp / "home-empty"
        (home_empty / "svg_grid" / "fontlib").mkdir(parents=True)
        p = run(["plot-grid", "--list-fonts"], env=dict(base_env, HOME=str(home_empty)), cwd="/tmp",
                binary=copied)
        check(p.returncode == 3 and "font library" in p.stderr,
              "数据根为空 ⇒ 退出 3，且报文里给的是那个数据根路径",
              f"rc={p.returncode} stderr={p.stderr[:300]!r}")

    # 编译期路径：源码里没有 env!("CARGO_MANIFEST_DIR")，二进制里没有 <checkout>fontlib
    src = "\n".join(p.read_text() for p in sorted((SVG_GRID / "src").rglob("*.rs")))
    check("CARGO_MANIFEST_DIR" not in src,
          "源码 不再有 `CARGO_MANIFEST_DIR`（字体目录只能运行时解析）")
    if shutil.which("strings"):
        baked = f"{SVG_GRID}fontlib"
        found = [line for line in
                 subprocess.run(["strings", str(BIN)], capture_output=True, text=True).stdout.splitlines()
                 if baked in line]
        check(not found, f"二进制 没有把构建机的字体目录烧进去（`strings` 里没有 `{baked}`）", found[:3])
        stripped = tmp / "svg_grid-stripped"
        shutil.copy2(BIN, stripped)
        if shutil.which("strip"):
            subprocess.run(["strip", "-s", str(stripped)], capture_output=True)
            left = [line for line in
                    subprocess.run(["strings", str(stripped)], capture_output=True, text=True)
                    .stdout.splitlines() if str(SVG_GRID) in line]
            check(not left, "二进制（strip 后）没有任何本 checkout 的绝对路径", left[:3])
        else:
            print("SKIP  strip 后的路径断言：没有 `strip`（binutils）")
    else:
        print("SKIP  二进制路径断言：没有 `strings`（binutils）")

    # ── 7. CLI 解析器的单测（结构 + 真跑）
    main_rs = (SVG_GRID / "src/main.rs").read_text()
    check(len(main_rs.splitlines()) <= 12 and "mod cli;" in main_rs and "cli::main()" in main_rs,
          "结构 main.rs 只做 `main()` 与委托（帮助文本/解析在 src/cli.rs）", main_rs[:300])
    cli_tests = (SVG_GRID / "src/cli/tests.rs").read_text()
    n_tests = len(re.findall(r"^#\[test\]", cli_tests, re.M))
    check(n_tests >= 8 and "exit_code" in cli_tests and "help_text_covers_every_flag" in cli_tests,
          f"结构 src/cli/tests.rs 有解析器单测（{n_tests} 个 #[test]，含帮助覆盖与退出码分档）")

    # ── 8. 回归 R4–R13
    for r in ("R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R13"):
        t = SVG_GRID / "docs/PITFALLS/round-data" / r / "test.py"
        if not t.exists():
            print(f"SKIP  回归 round-data/{r}：没有 test.py")
            continue
        p = subprocess.run([sys.executable, str(t)], capture_output=True, text=True)
        check(p.returncode == 0, f"回归 round-data/{r} 仍全绿", (p.stdout[-300:] + p.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for f in FAILS:
            print("  · " + f)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
