#!/usr/bin/env python3
"""R15 的可跑证据 —— **安装**：`install.sh` 编译、装二进制、装字体，可测且幂等。

本轮把"装不上 / 装上但渲染不出字 / 装了不可信"三族钉死（契约见 `ARCHITECTURE/install.md`）：

1. **参数面**：`--help` ⇒ stdout 打用法、退出 0、全英；`--prefix <dir>`、`--no-fonts`、
   `--no-build`、`--dry-run` 都被帮助文本覆盖；未知参数 / `--prefix` 缺值 ⇒ 退出 2。
2. **`--dry-run` 零写入**：退出 0，且数据根（`$HOME/svg_grid/fontlib`）的文件数/mtime 一字未变、
   目标 `<prefix>` 没被创建、`target/release` 的 mtime 没变（不触发 build）；`HOME` 指到空目录时
   `$HOME` 下什么都不建。
3. **真装**：`install.sh` 退出 0、stderr 空、报告正好五行（version / installed / fixtures / fonts / PATH 各一行）；
   装进 `<prefix>` 的 `svg_grid --version` 与 `target/release/svg_grid` **一致且字节相同**；
   R18 起只有**一个**二进制（转换器是 `svg_grid convert` 子命令），故同时断言 `<prefix>` 里没有 `svg_grid_convert`；
   `<prefix>` 不在 PATH ⇒ 一条修法提示、仍退出 0。
4. **从 /tmp 用干净环境可用**：`env -i HOME=… PATH=<prefix>:/usr/bin:/bin` 下
   `svg_grid --version` / `--help`（rc=0）/ `plot-grid --list-fonts` 都正常；默认组图真出一张 PNG。
5. **字体数据根 8 个 face**：`$HOME/svg_grid/fontlib` 里有 Arial / Times New Roman 各 4 个 face，
   `fontlib/setup.sh --dry-run` 复核 0 missing。
6. **幂等**：重复跑 `install.sh` 仍退出 0，报告写 `already up-to-date`，字体目录与二进制内容不变。
7. **无编译期路径**：`strings` release 二进制（以及装好的那份）里没有本 checkout 的绝对路径。
8. **回归**：R4–R14 的 `test.py` 全绿。

无参数；约 3 分钟（回归占绝大部分）；离线；只写临时目录、`<prefix>` 与 `$HOME/svg_grid/`
（这正是本轮的交付），不写仓库。跳过一律打印原因。
"""
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]                  # round-data/R15 → PITFALLS → docs → svg_grid
INSTALL = SVG_GRID / "install.sh"
FONT_SETUP = SVG_GRID / "fontlib" / "setup.sh"
RELEASE = SVG_GRID / "target/release/svg_grid"
RELEASE_CONVERT = SVG_GRID / "convert/target/release/svg_grid_convert"
HOME = pathlib.Path(os.environ.get("HOME") or pathlib.Path.home())
DATA_ROOT = HOME / "svg_grid/fontlib"
DEFAULT_PREFIX = HOME / ".local/bin"
FAILS = []

SVG = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">'
       '<rect data-panel-box="main" x="0" y="0" width="200" height="200" fill="none" stroke="none"/>'
       '<rect x="20" y="20" width="60" height="60" fill="#3366cc"/>'
       '<text x="100" y="150" font-size="16">hello</text></svg>')


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok and detail:
        print("      " + str(detail).replace("\n", "\n      ")[:900])
    if not ok:
        FAILS.append(label)


def skip(label, reason):
    print(f"SKIP  {label} — {reason}")


def run(args, env=None, cwd=None):
    return subprocess.run(args, capture_output=True, text=True, env=env, cwd=cwd)


def install(args, env=None, cwd=None):
    return run(["bash", str(INSTALL), *args], env=env, cwd=cwd)


def clean_env(home=None, path_prefix=None):
    """`env -i` equivalent: only HOME / PATH / LANG, so nothing from the caller leaks in."""
    parts = [p for p in (str(path_prefix), "/usr/local/bin", "/usr/bin", "/bin") if p]
    return {"HOME": str(home or HOME), "PATH": ":".join(parts), "LANG": "C"}


def snapshot(root):
    """Every entry under `root` (name, kind, size, mtime_ns), or None when absent."""
    if not root.exists():
        return None
    entries = [("<root>", "dir", 0, root.stat().st_mtime_ns)]
    for path in sorted(root.rglob("*")):
        stat = path.lstat()
        entries.append((str(path.relative_to(root)), "dir" if path.is_dir() else "file",
                        stat.st_size if path.is_file() else 0, stat.st_mtime_ns))
    return entries


def faces(root):
    if not root.is_dir():
        return []
    return sorted(p for p in root.rglob("*")
                  if p.is_file() and p.suffix.lower() in {".ttf", ".otf", ".ttc"})


def binary_versions(*paths):
    return {str(p): run([str(p), "--version"]).stdout.strip() for p in paths if p.exists()}


def stat_pair(path):
    return (path.stat().st_mtime_ns, path.stat().st_size) if path.exists() else None


def main():
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r15-rounddata-"))
    panel = tmp / "panel.svg"
    panel.write_text(SVG)

    # ── 0. 前置材料：install.sh / fontlib/setup.sh 必须在；本机字体是否齐全（决定是否 SKIP）
    check(INSTALL.is_file() and os.access(INSTALL, os.X_OK),
          "install.sh 存在且可执行（`tools/svg_grid/install.sh`）", f"{INSTALL}")
    if not INSTALL.is_file():
        print("\n== 1 条不成立 ==")
        return 1
    check(FONT_SETUP.is_file(), "fontlib/setup.sh 存在（字体步骤复用它）", f"{FONT_SETUP}")

    probe = run(["bash", str(FONT_SETUP), "--dry-run", "--dir", str(DATA_ROOT)])
    summary = re.search(r"(\d+) copied, (\d+) already up-to-date, (\d+) missing", probe.stdout)
    if summary is None:
        fonts_ok = False
        skip("字体齐全性探测", f"fontlib/setup.sh --dry-run 没给出汇总行（rc={probe.returncode}）: "
                              f"{probe.stderr.strip()[:200]}")
    else:
        fonts_ok = int(summary.group(3)) == 0
        if not fonts_ok:
            skip("依赖字体的断言（真装 / 8 face / 幂等）",
                 f"本机缺 {summary.group(3)} 个 face（setup.sh --dry-run 报的），先装 Arial / Times New Roman")

    # ── 1. 参数面：--help 全英、覆盖全部 flag；未知参数 / 缺值 ⇒ 2
    help_run = install(["--help"])
    check(help_run.returncode == 0 and help_run.stdout.strip() and help_run.stderr == "",
          "`install.sh --help` ⇒ 退出 0、stdout 打用法、stderr 空",
          f"rc={help_run.returncode} stderr={help_run.stderr[:200]!r}")
    missing_flags = [flag for flag in ("--prefix", "--no-fixtures", "--no-fonts",
                                       "--no-build", "--dry-run", "--help")
                     if flag not in help_run.stdout]
    check(not missing_flags, "帮助文本覆盖全部 flag（--prefix/--no-fixtures/--no-fonts/--no-build/--dry-run/--help）",
          f"缺: {missing_flags}")
    check(not re.search(r"[\u3000-\u9fff\uff00-\uffef]", help_run.stdout),
          "帮助文本全英（无 CJK 字符）", help_run.stdout[:300])

    fresh_home = tmp / "fresh-home"
    help_fresh = install(["--help"], env=clean_env(home=fresh_home), cwd="/tmp")
    check(help_fresh.returncode == 0 and not fresh_home.exists(),
          "`--help` 不写任何文件（HOME 指空目录 ⇒ 该目录仍不存在）",
          f"rc={help_fresh.returncode} exists={fresh_home.exists()}")

    bad = install(["--bogus"])
    check(bad.returncode == 2 and "unknown argument" in bad.stderr and "usage:" in bad.stderr,
          "未知参数 ⇒ 退出 2 + stderr 打用法", f"rc={bad.returncode} stderr={bad.stderr[:200]!r}")
    bad = install(["--prefix"])
    check(bad.returncode == 2 and bad.stderr.strip(),
          "`--prefix` 缺值 ⇒ 退出 2 + 说明", f"rc={bad.returncode} stderr={bad.stderr[:200]!r}")

    # ── 2. --dry-run 零写入
    prefix_probe = tmp / "prefix-must-not-exist"
    root_before = snapshot(DATA_ROOT)
    binaries_before = {p: stat_pair(p) for p in (RELEASE, RELEASE_CONVERT)}
    dry = install(["--dry-run", "--prefix", str(prefix_probe)])
    check(dry.returncode == 0, "`install.sh --dry-run` ⇒ 退出 0",
          f"rc={dry.returncode} stderr={dry.stderr[:300]!r}")
    check(not prefix_probe.exists(), "--dry-run 没有创建目标 <prefix>",
          f"{prefix_probe} exists={prefix_probe.exists()}")
    check(snapshot(DATA_ROOT) == root_before,
          "--dry-run 没写字体数据根（前后文件数/mtime 一字未变）",
          f"before={len(root_before or [])} after={len(snapshot(DATA_ROOT) or [])}")
    check({p: stat_pair(p) for p in binaries_before} == binaries_before,
          "--dry-run 不触发 build（target/release 两个二进制 mtime/size 未变）")
    check("would build" in dry.stdout and "would install" in dry.stdout,
          "--dry-run 打印将要做的事（would build / would install）", dry.stdout[:400])

    # ── 3. 真装
    if fonts_ok:
        real = install([])
    else:
        real = install(["--no-fonts"])
    check(real.returncode == 0 and real.stderr == "",
          "`install.sh`（真装）⇒ 退出 0、stderr 空", f"rc={real.returncode} stderr={real.stderr[:400]!r}")
    lines = [line for line in real.stdout.splitlines() if line.strip()]
    kinds = {
        "version": [l for l in lines if re.match(r"^svg_grid \d+\.\d+", l)],
        "installed": [l for l in lines if l.startswith("installed: ")],
        "skill": [l for l in lines if l.startswith("skill: ")],
        "fixtures": [l for l in lines if l.startswith("fixtures: ")],
        "fonts": [l for l in lines if l.startswith("fonts: ")],
        "PATH": [l for l in lines if l.startswith("PATH: ")],
    }
    # R22 起安装器也多装**一件**东西：随工具发布的 skill（general-figure-guide），
    # 故报告的"一行一个事实"从 5 类变 6 类。
    check(all(len(v) == 1 for v in kinds.values()) and len(lines) == 6,
          "报告：一行一个事实（version / installed / skill / fixtures / fonts / PATH 各恰好一行）",
          f"{len(lines)} 行: {lines}")
    check(re.search(r"^fixtures: \d+ files in .+/svg_grid/fixtures$", kinds["fixtures"][0] or ""),
          "报告：语料步骤报了数据根里的 fixtures 条数与位置", kinds["fixtures"][0])
    if fonts_ok:
        check("8 faces" in kinds["fonts"][0] or "8 face" in kinds["fonts"][0],
              "报告：字体步骤报了数据根里的 face 数（8）", kinds["fonts"][0])

    installed = DEFAULT_PREFIX / "svg_grid"
    # R18 起只有**一个**二进制：转换器成了 `svg_grid convert` 子命令（库依赖编进来），
    # `svg_grid_convert` 不再安装。本轮的"装上了吗"断言随之改为「装一个、且旧的没留下」。
    check(installed.is_file() and os.access(installed, os.X_OK),
          "二进制装进 <prefix> 且可执行", f"{DEFAULT_PREFIX}")
    check((DEFAULT_PREFIX / "svg_grid_convert").exists() is False,
          "旧的 `svg_grid_convert` 不在 <prefix> 里（R18：单二进制）", f"{DEFAULT_PREFIX}")
    check(installed.read_bytes() == RELEASE.read_bytes(),
          "装的二进制与 target/release 字节相同（拷贝而非软链）")
    in_versions = binary_versions(installed, RELEASE)
    check(in_versions.get(str(installed)) == in_versions.get(str(RELEASE))
          and in_versions.get(str(installed), "").startswith("svg_grid "),
          "<prefix>/svg_grid --version 与 target/release/svg_grid 一致",
          in_versions)
    check(shutil.which("svg_grid", path=str(DEFAULT_PREFIX)) is not None,
          "`command -v svg_grid` 有输出（<prefix> 在 PATH 上时能解析到）",
          shutil.which("svg_grid", path=str(DEFAULT_PREFIX)))

    # 自定义 prefix + --no-fonts：只装二进制、不碰数据根，且 PATH 提示非致命
    custom = tmp / "custom-prefix"
    font_snapshot = snapshot(DATA_ROOT)
    custom_run = install(["--no-build", "--no-fonts", "--prefix", str(custom)])
    check(custom_run.returncode == 0 and (custom / "svg_grid").is_file()
          and not (custom / "svg_grid_convert").exists(),
          "`--prefix <tmp> --no-build --no-fonts` ⇒ 只把那一个二进制装到那里",
          f"rc={custom_run.returncode} stderr={custom_run.stderr[:300]!r}")
    check("is not on PATH" in custom_run.stdout and "export PATH=" in custom_run.stdout,
          "<prefix> 不在 PATH ⇒ 给一条修法提示（非致命，仍退出 0）", custom_run.stdout[:400])
    check(snapshot(DATA_ROOT) == font_snapshot,
          "`--no-fonts` 真的跳过字体步骤（数据根一字未动）")

    # 全新 HOME（等价于换机器）：装一次 = 从本机复制 8 个 face 进空数据根，装完立刻可用
    if fonts_ok:
        fresh = tmp / "fresh-install-home"
        fresh_run = install(["--no-build"], env=dict(os.environ, HOME=str(fresh)))
        fresh_faces = faces(fresh / "svg_grid/fontlib")
        check(fresh_run.returncode == 0 and len(fresh_faces) == 8,
              "全新 HOME：`install.sh --no-build` ⇒ 8 个 face 从本机复制进空数据根",
              f"rc={fresh_run.returncode} faces={len(fresh_faces)} stderr={fresh_run.stderr[:300]!r}")
        counts = re.search(r"copied (\d+), already up-to-date (\d+)", fresh_run.stdout)
        check(counts is not None and int(counts.group(1)) == 8,
              "全新 HOME：报告说 copied 8（从本机复制，不下载）", fresh_run.stdout[:400])
        fresh_env = clean_env(home=fresh, path_prefix=fresh / ".local/bin")
        fresh_list = run(["svg_grid", "plot-grid", "--list-fonts"], env=fresh_env, cwd="/tmp")
        check(fresh_list.returncode == 0 and "Arial" in fresh_list.stdout,
              "全新 HOME：装完立刻可用（`--list-fonts` 读的是这个 HOME 的数据根）",
              f"rc={fresh_list.returncode} stdout={fresh_list.stdout[:200]!r} "
              f"stderr={fresh_list.stderr[:300]!r}")
    else:
        skip("全新 HOME 装一次", "本机字体不全（见上）")

    # ── 4. 从 /tmp 用干净环境可用
    env = clean_env(path_prefix=DEFAULT_PREFIX)
    version = run(["svg_grid", "--version"], env=env, cwd="/tmp")
    check(version.returncode == 0 and version.stdout.startswith("svg_grid "),
          "干净环境 + cwd=/tmp：`svg_grid --version` ⇒ 退出 0、打版本",
          f"rc={version.returncode} stdout={version.stdout[:80]!r} stderr={version.stderr[:200]!r}")
    help_out = run(["svg_grid", "--help"], env=env, cwd="/tmp")
    check(help_out.returncode == 0 and "--list-fonts" in help_out.stdout and help_out.stderr == "",
          "干净环境 + cwd=/tmp：`svg_grid --help` ⇒ 退出 0、完整帮助（stderr 空）",
          f"rc={help_out.returncode} bytes={len(help_out.stdout)} stderr={help_out.stderr[:200]!r}")
    list_fonts = run(["svg_grid", "plot-grid", "--list-fonts"], env=env, cwd="/tmp")
    check(list_fonts.returncode == 0 and "Arial" in list_fonts.stdout
          and "Times New Roman" in list_fonts.stdout,
          "干净环境 + cwd=/tmp：`plot-grid --list-fonts` ⇒ 退出 0，列出 Arial / Times New Roman",
          f"rc={list_fonts.returncode} stdout={list_fonts.stdout[:200]!r} stderr={list_fonts.stderr[:300]!r}")

    out_png = tmp / "grid.png"
    out_svg = tmp / "grid.svg"
    compose = run(["svg_grid", "plot-grid", "--input", str(panel), "--output", str(out_svg),
                   "--output-png", str(out_png)], env=env, cwd="/tmp")
    check(compose.returncode == 0 and out_png.is_file()
          and out_png.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n" and out_png.stat().st_size > 1000,
          "干净环境 + cwd=/tmp：默认组图真出一张 PNG（rc=0、PNG 魔数、>1 KB）",
          f"rc={compose.returncode} size={out_png.stat().st_size if out_png.exists() else 0} "
          f"stderr={compose.stderr[:300]!r}")

    # ── 5. 字体数据根 8 个 face
    if fonts_ok:
        face_list = faces(DATA_ROOT)
        names = sorted(p.name for p in face_list)
        check(len(face_list) == 8, f"数据根有 8 个 face（{DATA_ROOT}）", names)
        check(any(n.startswith("Arial-") for n in names)
              and any(n.startswith("Times_New_Roman-") for n in names),
              "8 个 face 覆盖 Arial 与 Times New Roman（各 4 个 style）", names)
        check(sum(1 for n in names if n.startswith("Arial-")) == 4
              and sum(1 for n in names if n.startswith("Times_New_Roman-")) == 4,
              "每族 4 个 style（Regular/Bold/Italic/Bold Italic）", names)
        recheck = run(["bash", str(FONT_SETUP), "--dry-run", "--dir", str(DATA_ROOT)])
        check(recheck.returncode == 0 and "0 missing" in recheck.stdout,
              "fontlib/setup.sh 复核：0 missing（装完字体库是完整的）",
              f"rc={recheck.returncode} {recheck.stdout[-300:]!r}")
    else:
        skip("字体数据根 8 个 face", "本机字体不全（见上）")

    # ── 6. 幂等：再跑一次
    if fonts_ok:
        faces_before = [(p.name, p.stat().st_size) for p in faces(DATA_ROOT)]
        bytes_before = installed.read_bytes()
        again = install([])
        check(again.returncode == 0 and again.stderr == "",
              "重复跑 `install.sh` ⇒ 仍退出 0、stderr 空（幂等）",
              f"rc={again.returncode} stderr={again.stderr[:300]!r}")
        check("already up-to-date" in again.stdout,
              "重复跑 ⇒ 报告 already up-to-date（字体逐字节相同就跳过）", again.stdout[:400])
        check([(p.name, p.stat().st_size) for p in faces(DATA_ROOT)] == faces_before,
              "重复跑 ⇒ 字体目录没有多/少文件", f"{faces_before}")
        check(installed.read_bytes() == bytes_before
              and binary_versions(installed, RELEASE)[str(installed)]
              == binary_versions(installed, RELEASE)[str(RELEASE)],
              "重复跑 ⇒ 装的二进制内容与版本不变（新版覆盖旧版也是幂等的）")
    else:
        skip("重复安装幂等", "本机字体不全（见上）")

    # ── 7. 无编译期路径（装到别处仍读 $HOME 数据根）
    checkout = str(SVG_GRID)
    if shutil.which("strings"):
        for binary in (RELEASE, RELEASE_CONVERT, installed):
            if not binary.exists():
                skip(f"`strings {binary}`", "二进制不存在")
                continue
            hits = [line for line in run(["strings", str(binary)]).stdout.splitlines()
                    if checkout in line]
            check(not hits, f"`strings {binary.name}` 里没有本 checkout 路径（{checkout}）", hits[:3])
    else:
        skip("二进制路径断言", "没有 `strings`（binutils）")
    sources = "\n".join(p.read_text() for p in sorted((SVG_GRID / "src").rglob("*.rs")))
    check("CARGO_MANIFEST_DIR" not in sources,
          "源码没有 `CARGO_MANIFEST_DIR`（安装位置与运行目录无关）")

    # ── 8. 回归 R4–R14
    for round_id in ("R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R13", "R14"):
        evidence = HERE.parent / round_id / "test.py"
        if not evidence.exists():
            skip(f"回归 round-data/{round_id}", "没有 test.py")
            continue
        regression = run([sys.executable, str(evidence)])
        check(regression.returncode == 0, f"回归 round-data/{round_id} 仍全绿",
              (regression.stdout[-400:] + regression.stderr[-200:]))

    if FAILS:
        print(f"\n== {len(FAILS)} 条不成立 ==")
        for label in FAILS:
            print("  · " + label)
        return 1
    print("\n== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
