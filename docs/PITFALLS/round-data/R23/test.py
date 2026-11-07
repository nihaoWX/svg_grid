#!/usr/bin/env python3
"""R23 的 round-data —— Windows 的安装面：数据根跨平台解析 + 一份可照做的手动安装指引。

判据（都可机械跑）：
  ① 数据根 `HOME` → `USERPROFILE`：用**真实二进制**在三种环境里验（只有 USERPROFILE / 只有 HOME / 都没有）；
  ② 只有一份实现：`src/home.rs` 里有 `pick()` 的单测（`cargo test home::` 至少 3 条过）；
  ③ `docs/install-windows.md` 存在、≤ 20 KB，且七个关键步骤都在；
  ④ README 同时给出 Linux/macOS 与 Windows 两条路径，且链接的文档真实存在；
  ⑤ 面向 agent 的文案不再写死 `~/`：`--help` 与 `doctor` 的"没有语料"报错都提到 Windows；
  ⑥ 裁决入判据：仓库里没有 `install.ps1` / `install.bat`，也没有入库的 `*.exe`（不预编译 Windows 二进制）。

无参数；约 2 分钟（③④⑤⑥ 秒级；①要跑二进制；②要 `cargo test`）。只读：不写仓库、不建目录。
"""
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
REPO = HERE.parents[5]
BIN = SVG_GRID / "target/debug/svg_grid"
DOC = SVG_GRID / "docs/install-windows.md"
README = SVG_GRID / "README.md"
HOME_RS = SVG_GRID / "src/home.rs"

fails = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"  [{detail}]"))
    if not ok:
        fails.append(label)
    return ok


def run(args, env):
    return subprocess.run([str(BIN), *args], capture_output=True, text=True, env=env)


def env_of(**overrides):
    base = {"PATH": "/usr/bin:/bin", "LANG": "C"}
    base.update(overrides)
    return base


def where_fontlib(stderr):
    """从报错里抠出它去找的 fontlib 路径（那就是数据根 + /fontlib）。"""
    m = re.search(r"no fonts in (.+?) — ", stderr)
    return m.group(1) if m else None


def main():
    print("== ① 数据根：HOME → USERPROFILE（真实二进制）==")
    win = run(["plot-grid", "--list-fonts"], env_of(USERPROFILE="/tmp/r23-win"))
    got = where_fontlib(win.stderr)
    check(got == "/tmp/r23-win/svg_grid/fontlib",
          "只有 USERPROFILE ⇒ 数据根 = <USERPROFILE>/svg_grid/fontlib（Windows 路径）", win.stderr[:200])

    unix = run(["plot-grid", "--list-fonts"], env_of(HOME="/tmp/r23-unix"))
    got = where_fontlib(unix.stderr)
    check(got == "/tmp/r23-unix/svg_grid/fontlib",
          "只有 HOME ⇒ 数据根 = <HOME>/svg_grid/fontlib（Linux/macOS，行为未变）", unix.stderr[:200])

    both = run(["plot-grid", "--list-fonts"], env_of(HOME="/tmp/r23-home", USERPROFILE="/tmp/r23-profile"))
    got = where_fontlib(both.stderr)
    check(got == "/tmp/r23-home/svg_grid/fontlib",
          "两者都有 ⇒ 取 HOME（Git Bash 与原生 shell 不会各读各的）", both.stderr[:200])

    none = run(["plot-grid", "--list-fonts"], env_of())
    got = where_fontlib(none.stderr)
    check(got == "svg_grid/fontlib",
          "两个都没有 ⇒ 相对 svg_grid/fontlib（不会退化成 /svg_grid）", none.stderr[:200])

    print("\n== ② 只有一份实现，且顺序可单测 ==")
    check(HOME_RS.is_file(), "src/home.rs 存在（字体与语料共用同一个解析）")
    lib_rs = (SVG_GRID / "src/lib.rs").read_text(encoding="utf-8")
    check("pub mod home;" in lib_rs, "lib.rs 声明了 pub mod home（二进制侧 doctor 也能用）")
    raster = (SVG_GRID / "src/raster.rs").read_text(encoding="utf-8")
    check("crate::home::data_root()" in raster and 'var_os("HOME")' not in raster,
          "raster.rs 走 home 模块，不再自己读 HOME")
    doctor = (SVG_GRID / "src/cli/doctor/mod.rs").read_text(encoding="utf-8")
    check("svg_grid::home::home_dir()" in doctor and 'var_os("HOME")' not in doctor,
          "doctor 走 home 模块，不再自己读 HOME")
    t = subprocess.run(["cargo", "test", "--offline", "--quiet", "home::"],
                       cwd=str(SVG_GRID), capture_output=True, text=True)
    passed = re.search(r"(\d+) passed", t.stdout + t.stderr)
    check(t.returncode == 0 and passed and int(passed.group(1)) >= 3,
          "`cargo test home::` 至少 3 条过（顺序 / Windows 回退 / 空值）",
          (t.stdout + t.stderr)[-400:])

    print("\n== ③ Windows 指引：七个步骤一个不少 ==")
    check(DOC.is_file(), "docs/install-windows.md 存在")
    if DOC.is_file():
        doc = DOC.read_text(encoding="utf-8")
        size = len(DOC.read_bytes())
        check(size <= 20000, f"体积 {size} B（≤ 20000）", f"{size}")
        steps = {
            "在自己机器上 cargo build --release": "cargo build --release",
            "把 exe 放进 PATH": "PATH",
            "数据根 %USERPROFILE%\\svg_grid": r"%USERPROFILE%\svg_grid",
            "复制 Arial 字体": "arial.ttf",
            "复制 Times 字体": "times.ttf",
            "复制语料 tests\\fixtures": r"tests\fixtures",
            "装 skill 到 .commandcode\\skills": r".commandcode\skills",
            "用 doctor 验收": "svg_grid doctor",
            "absence of a windows script is explained": "install.ps1",
        }
        for label, needle in steps.items():
            check(needle in doc, f"指引含：{label}")

    print("\n== ④ README 说清三平台 ==")
    readme = README.read_text(encoding="utf-8")
    check("安装" in readme, "README 有「安装」段落")
    check("install.sh" in readme, "README 给了 Linux/macOS 的一条命令（install.sh）")
    check("install-windows.md" in readme, "README 指向 Windows 指引")
    link = re.search(r"\[docs/install-windows\.md\]\(([^)]+)\)", readme)
    check(link is not None and (SVG_GRID / link.group(1)).is_file(),
          "README 里的相对链接能解析到真实文件", link.group(1) if link else None)
    check("USERPROFILE" in readme, "README 说明 Windows 的数据根是 %USERPROFILE%\\svg_grid")

    print("\n== ⑤ 面向 agent 的文案不写死 `~/` ==")
    help_out = run(["--help"], env_of(HOME="/tmp/r23"))
    text = help_out.stdout
    check("%USERPROFILE%" in text, "`--help` 提到 %USERPROFILE%", text[:0])
    check("$HOME/svg_grid" not in text, "`--help` 不再写死 `$HOME/svg_grid`")
    missing = run(["doctor", "--fixtures", "/nonexistent-r23"], env_of(HOME="/tmp/r23"))
    check(missing.returncode == 3, "`doctor --fixtures <不存在>` ⇒ 退出 3（配置错这一档没变）",
          f"rc={missing.returncode}")
    check("%USERPROFILE%" in missing.stderr and "Windows" in missing.stderr,
          "`doctor` 的「没有语料」报错说明 Windows 怎么放语料", missing.stderr[:300])
    check(missing.stderr.count("\n") == 1, "那条报错仍是**一句话一行**（不摊成多行）",
          missing.stderr[:400])

    print("\n== ⑥ 裁决入判据：无 Windows 脚本、无预编译 exe ==")
    stray = [p.name for p in SVG_GRID.iterdir() if p.suffix.lower() in (".ps1", ".bat", ".cmd")]
    check(not stray, "工具目录里没有 install.ps1 / .bat / .cmd（不维护分叉脚本）", stray)
    tracked = subprocess.run(["git", "ls-files", "*.exe"], cwd=str(REPO),
                             capture_output=True, text=True)
    check(tracked.returncode == 0 and tracked.stdout.strip() == "",
          "仓库里没有入库的 *.exe（不预编译 Windows 二进制）", tracked.stdout[:200])

    print()
    if fails:
        print(f"== {len(fails)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
