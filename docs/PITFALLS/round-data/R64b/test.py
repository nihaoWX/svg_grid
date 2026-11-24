#!/usr/bin/env python3
"""R64b 的可跑证据 —— 两条控制必须是**精确序列化**，且 walk 的守卫变异必须让它们变红。

背景：R64 把 `<use>` 引用的定义接进引用者域，同时留了两条“字节不变”的对照（u6 的
`<symbol viewBox>` 经 `<use>`、u15 的 `viewBox` pattern body 里的 `<use>`）。R64 评审
P1-1 指出：那两条对照在 `tests_r64.rs` 里只断言属性（symbol body 的 width、pattern tile
的 width），删掉 `src/transform/walk.rs` 里 `&& defs::is_definition(element)` 这一守卫
（下称 m4）后 `cargo test` 仍全绿——只有本目录旁边的 R64/test.py 的字节比对能抓。

所以本轮的证据有两层，缺一不可：

① 两条测试**在名单里**且**用例数精确**（测试被改名/删掉、计数被悄悄改，本条立刻红）；
② **m4 变异下那条测试必须变红**（默认就跑；`--skip-mutation` 只跳过这一族并打印原因）。
   变异在 `git worktree add` 出来的临时树里做，跑完 `finally` 清理，并断言原仓库的
   `src/transform/walk.rs` 字节未变——绝不在工作树里改源码。

三族都在契约内：无参默认全跑；rc=0=全过、rc=1=有红、rc=2=环境不可用（不是 SKIP）。
离线；除 `cargo` 写 `target/` 与临时目录外不写仓库。
"""
import argparse
import hashlib
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]                      # R64b → round-data → PITFALLS → docs → svg_grid
WALK = ROOT / "src" / "transform" / "walk.rs"
GUARD = "if slot.is_some_and(Slot::is_verbatim) && defs::is_definition(element) {"
MUTANT = "if slot.is_some_and(Slot::is_verbatim) {"
RED_TEST = "a_use_inside_a_viewbox_pattern_body_stays_verbatim"
CONTROLS = [
    "transform::tests_r64::a_symbol_with_a_viewbox_keeps_its_body_verbatim",
    "transform::tests_r64::a_use_inside_a_viewbox_pattern_body_stays_verbatim",
]
# 上一轮的精确计数：任何一轮增删测试都必须同轮更新这里（与 R19/R57 同一口径）。
BASELINE = {"unittests src/lib.rs": 419, "unittests src/main.rs": 59,
            "tests/selftest/main.rs": 233}
RUNNING_RE = re.compile(r"^\s+(?:Running|Doc-tests) (.+?)(?: \([^)]*\))?\s*$")
RESULT_RE = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed;")

FAILS = []


def check(ok, label, detail=""):
    print(("PASS  " if ok else "FAIL  ") + label)
    if not ok:
        print("      " + str(detail).replace("\n", "\n      ")[:1200])
        FAILS.append(label)


def info(text):
    print(f"INFO  {text}")


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args, cwd=None, env=None, timeout=900):
    try:
        return subprocess.run([str(a) for a in args], capture_output=True, text=True,
                              cwd=str(cwd or ROOT), env=env, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        return subprocess.CompletedProcess([str(a) for a in args], 124,
                                           (exc.stdout or "") if isinstance(exc.stdout, str) else "",
                                           f"timeout after {timeout}s")


def family_names():
    print("\n== ① 两条控制仍在测试名单里 ==")
    result = run(["cargo", "test", "--offline", "--lib", "--", "--list"], timeout=900)
    listed = set()
    for line in result.stdout.splitlines():
        name = line.rsplit(":", 1)[0].strip()
        if line.strip().endswith(": test"):
            listed.add(name)
    check(result.returncode == 0, "cargo test --lib -- --list rc=0",
          result.stdout[-800:] + result.stderr[-800:])
    for name in CONTROLS:
        check(name in listed, f"listed: {name}", f"missing from {len(listed)} listed tests")
    if result.returncode == 0 and all(n in listed for n in CONTROLS):
        info("两条控制都在名单里（改名/删除会让本条立刻红）")


def family_counts():
    print("\n== ② cargo test 三个 target 的用例数精确等于快照 ==")
    result = run(["cargo", "test", "--offline"], timeout=1800)
    names = [m.group(1) for m in
             (RUNNING_RE.match(line) for line in result.stderr.splitlines()) if m]
    results = [m.groups() for m in
               (RESULT_RE.match(line) for line in result.stdout.splitlines()) if m]
    check(result.returncode == 0, "cargo test --offline rc=0",
          result.stdout[-1200:] + result.stderr[-800:])
    if len(names) < len(BASELINE) or len(results) < len(BASELINE):
        check(False, "能解析出三个测试目标", f"names={names} results={len(results)}")
        return
    got = {name: (int(passed), int(failed))
           for name, (_, passed, failed) in zip(names, results)}
    for name, expected in BASELINE.items():
        actual = got.get(name)
        check(actual == (expected, 0) if actual else False,
              f"{name} = {expected} passed / 0 failed", f"got {actual}")
    total = sum(actual[0] for actual in got.values())
    check(total == sum(BASELINE.values()), f"总数 {sum(BASELINE.values())}",
          f"got {total}: {got}")


def family_mutation():
    print("\n== ③ m4 变异（walk 的 is_definition 守卫）必须让 u15 那条测试变红 ==")
    started = time.time()
    source = WALK.read_text()
    if source.count(GUARD) != 1:
        check(False, "walk.rs 里守卫恰好一处", f"count={source.count(GUARD)} of {GUARD!r}")
        return
    before = sha256(WALK)
    worktree = pathlib.Path(tempfile.mkdtemp(prefix="r64b-mut-"))
    target = pathlib.Path(tempfile.mkdtemp(prefix="r64b-target-"))
    removed = False
    try:
        added = run(["git", "-C", ROOT, "worktree", "add", "--detach", worktree, "HEAD"],
                    timeout=300)
        check(added.returncode == 0, "临时 worktree 建好",
              added.stdout[-400:] + added.stderr[-800:])
        if added.returncode != 0:
            return
        mutant = worktree / "src" / "transform" / "walk.rs"
        text = mutant.read_text()
        if text.count(GUARD) != 1:
            check(False, "临时树的 walk.rs 守卫恰好一处", f"count={text.count(GUARD)}")
            return
        mutant.write_text(text.replace(GUARD, MUTANT))
        # 独立 target：与主仓库共享 target 会让 cargo 复用临时树的产物（实测会把
        # 未变异的测试跑成变异体），所以变异族一律用一次性 target 目录。
        env = dict(os.environ, CARGO_TARGET_DIR=str(target))
        run_result = run(["cargo", "test", "--offline", "--lib", RED_TEST],
                         cwd=worktree, env=env, timeout=1800)
        output = run_result.stdout + run_result.stderr
        check(run_result.returncode != 0,
              "变异下 cargo test --lib 必须 rc≠0", output[-1200:])
        check(RED_TEST in output and re.search(r"1 failed", output) is not None,
              f"红名出现在失败里：{RED_TEST}", output[-1200:])
        info(f"变异：{GUARD}  ->  {MUTANT}")
    finally:
        cleaned = run(["git", "-C", ROOT, "worktree", "remove", "--force", worktree],
                      timeout=300)
        if cleaned.returncode != 0 and worktree.exists():
            shutil.rmtree(worktree, ignore_errors=True)
        run(["git", "-C", ROOT, "worktree", "prune"], timeout=120)
        removed = not worktree.exists()
        check(removed, "临时 worktree 已清理", f"still at {worktree}")
        shutil.rmtree(target, ignore_errors=True)
        check(sha256(WALK) == before, "原仓库 walk.rs 字节未变", "sha changed during mutation")
        info(f"③ 用时 {time.time() - started:.1f}s")


def main():
    parser = argparse.ArgumentParser(description="R64b evidence: exact serializations + m4 teeth")
    parser.add_argument("--skip-mutation", action="store_true",
                        help="只跳过 ③（变异族）；① ② 仍跑")
    args = parser.parse_args()

    for tool in ("cargo", "git"):
        if not shutil.which(tool):
            print(f"ENV   need `{tool}` on PATH")
            return 2
    if not (ROOT / "Cargo.toml").is_file():
        print(f"ENV   {ROOT} is not the svg_grid repo root")
        return 2

    info(f"repo {ROOT}")
    family_names()
    family_counts()
    if args.skip_mutation:
        print("\nSKIP  ③ m4 变异 — 传了 --skip-mutation（默认必须跑，见 meta.toml）")
    else:
        family_mutation()

    print(f"\nSUMMARY {len(FAILS)} failed")
    if FAILS:
        for label in FAILS:
            print(f"FAILED  {label}")
        return 1
    print("RESULT OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
