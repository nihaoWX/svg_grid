#!/usr/bin/env python3
"""R22 的 round-data —— `install.sh` 除了二进制，还要把**随工具发布的 skill**装到 harness 认的位置。

判据（都可机械跑）：
  ① 全局装：`~/.commandcode/skills/general-figure-guide/` 存在，文件集合 **只有** 白名单
     （`SKILL.md` + `references/*.md`），与规范源逐字节一致，目录名 == `SKILL.md` 的 `name:`；
  ② 幂等：再跑一次 report `up to date`，装好的目录 mtime 不变；
  ③ `--check` 只读自检：正常 ⇒ rc=0 且三行 OK；
  ④ 隔离的项目级安装（`--project <tmp>`）上做破坏性验证：多塞一个 `ARCHITECTURE.md` ⇒ rc=2 且说"多出"；
     改坏 `SKILL.md` 一字节 ⇒ rc=2 且指名该文件；重装后回到 rc=0；
     ⑤ `--project` 不污染全局那份；
  ⑥ `--dry-run` 不写（装好的目录 mtime 不变）。

无参数；约 1 分钟（每步都要跑 install.sh）；只读全局安装，破坏性验证全在临时目录里做。
"""
import hashlib
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVG_GRID = HERE.parents[3]
GUIDE = SVG_GRID / "docs/general-figure-guide"
INSTALL = SVG_GRID / "install.sh"
HOME = pathlib.Path(os.environ["HOME"])
SKILL_NAME = "general-figure-guide"
GLOBAL_DEST = HOME / ".commandcode/skills" / SKILL_NAME

fails = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label + ("" if ok else f"  [{detail}]"))
    if not ok:
        fails.append(label)
    return ok


def run(args, cwd=None):
    return subprocess.run(["bash", str(INSTALL), *args], cwd=str(cwd or SVG_GRID),
                          capture_output=True, text=True)


def files_of(root):
    return sorted(str(p.relative_to(root)) for p in root.rglob("*") if p.is_file())


def whitelist():
    """规范源里使用侧该有的文件（SKILL.md + references/ 下的全部文件）。"""
    out = ["SKILL.md"]
    out += [f"references/{p.name}" for p in sorted((GUIDE / "references").iterdir()) if p.is_file()]
    return sorted(out)


def tree_mtime(root):
    stamp = 0.0
    for p in [root, *root.rglob("*")]:
        stamp = max(stamp, p.stat().st_mtime)
    return stamp


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    print("== ① 全局安装：位置、白名单、逐字节 ==")
    check(GLOBAL_DEST.is_dir(), f"skill 装到 harness 的全局目录 {GLOBAL_DEST}")
    check(not GLOBAL_DEST.is_symlink(), "装的是**拷贝**，不是软链（外部使用者没有开发树）")
    if GLOBAL_DEST.is_dir():
        want = whitelist()
        got = files_of(GLOBAL_DEST)
        check(got == want, f"装好的目录只有白名单 {len(want)} 个文件", f"差集 want-got={sorted(set(want)-set(got))} got-want={sorted(set(got)-set(want))}")
        bad = [f for f in got if f.split("/")[0] not in ("SKILL.md", "references")]
        check(not bad, "没有混进开发侧文件（ARCHITECTURE*/src/docs/PROGRESS.md）", bad)
        d = subprocess.run(["diff", "-r", str(GUIDE), str(GLOBAL_DEST)], capture_output=True, text=True)
        check(d.returncode == 0, "与规范源逐字节一致（`diff -r` 为空）", d.stdout[:300])
        name = re.search(r"^name:\s*(\S+)", (GUIDE / "SKILL.md").read_text(encoding="utf-8"), re.M)
        check(name is not None and name.group(1) == GLOBAL_DEST.name,
              "目录名 == `SKILL.md` 的 `name:`（否则 harness 静默跳过）",
              f"{name.group(1) if name else None} vs {GLOBAL_DEST.name}")

    print("\n== ② 幂等：再跑一次什么都不动 ==")
    before = tree_mtime(GLOBAL_DEST)
    again = run(["--no-build", "--no-fixtures", "--no-fonts"])
    check(again.returncode == 0, "重跑 install.sh ⇒ rc=0", f"rc={again.returncode} {again.stderr[:200]}")
    check("up to date" in again.stdout, "报告说 skill 已是最新（`up to date`）",
          [l for l in again.stdout.splitlines() if l.startswith("skill: ")])
    check(tree_mtime(GLOBAL_DEST) == before, "装好的 skill 一个文件都没被改写（mtime 不变）")

    print("\n== ③ --check：只读、正常时 rc=0 ==")
    chk = run(["--check"])
    check(chk.returncode == 0, "`--check` ⇒ rc=0", f"rc={chk.returncode} {chk.stdout[:300]}")
    check(len([l for l in chk.stdout.splitlines() if l.startswith("OK: ")]) == 3,
          "`--check` 逐项 OK：binary / skill / location",
          chk.stdout.splitlines())
    check("FAIL" not in chk.stdout, "正常路径不出现 FAIL", chk.stdout[:300])

    print("\n== ④ 破坏性验证：全在隔离的 --project 临时目录里做 ==")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="r22-"))
    try:
        proj = run(["--no-build", "--no-fixtures", "--no-fonts", "--project", str(tmp)])
        dest = tmp / ".commandcode/skills" / SKILL_NAME
        check(proj.returncode == 0 and dest.is_dir(),
              f"`--project` ⇒ skill 落到 <项目>/.commandcode/skills/{SKILL_NAME}",
              f"rc={proj.returncode} {proj.stdout[-200:]}")
        check("project" in proj.stdout, "报告标明作用域是 project")
        check(files_of(dest) == whitelist(), "项目级那份也只含白名单")

        # 塞一个开发侧文件 ⇒ --check 必须报"多出"
        (dest / "ARCHITECTURE.md").write_text("dev-side file\n", encoding="utf-8")
        c1 = run(["--check", "--project", str(tmp)])
        check(c1.returncode == 2 and "must not have" in c1.stdout,
              "多塞 ARCHITECTURE.md ⇒ `--check` rc=2 且指出「多出开发侧文件」",
              f"rc={c1.returncode} {c1.stdout[:400]}")

        # 改坏一个字节 ⇒ --check 必须指名字
        victim = dest / "SKILL.md"
        original = victim.read_bytes()
        victim.write_bytes(original + b"\n")
        c2 = run(["--check", "--project", str(tmp)])
        check(c2.returncode == 2 and "SKILL.md" in c2.stdout,
              "改坏 SKILL.md ⇒ `--check` rc=2 且点名该文件",
              f"rc={c2.returncode} {c2.stdout[:400]}")
        victim.write_bytes(original)

        # 重装 ⇒ 回到 rc=0，且多出的文件被清掉
        fix = run(["--no-build", "--no-fixtures", "--no-fonts", "--project", str(tmp)])
        c3 = run(["--check", "--project", str(tmp)])
        check(fix.returncode == 0 and c3.returncode == 0,
              "重装后 `--check` 回到 rc=0（整目录替换 ⇒ 多出的文件被清掉）",
              f"install rc={fix.returncode} check rc={c3.returncode} {c3.stdout[:300]}")
        check(files_of(dest) == whitelist(), "重装后目录又只剩白名单", files_of(dest))

        print("\n== ⑤ --project 不污染全局那份 ==")
        check(sha(GLOBAL_DEST / "SKILL.md") == sha(GUIDE / "SKILL.md"),
              "全局那份仍是规范源的内容（项目级安装没有改写它）")

        print("\n== ⑥ --dry-run 不写 ==")
        before_dry = tree_mtime(GLOBAL_DEST)
        dry = run(["--dry-run"])
        check(dry.returncode == 0 and "would install skill" in dry.stdout,
              "`--dry-run` 打印「would install skill …」并 rc=0", f"rc={dry.returncode}")
        check(tree_mtime(GLOBAL_DEST) == before_dry, "`--dry-run` 没写任何文件（mtime 不变）")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    if fails:
        print(f"== {len(fails)} 条不成立 ==")
        return 1
    print("== 全部成立 ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
