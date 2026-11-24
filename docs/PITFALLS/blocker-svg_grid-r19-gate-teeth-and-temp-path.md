# R57：R19 计数门禁补回真牙 + temp_path 并发假红 + R55 口径更正

派单：`/tmp/r57-dispatch.md`（任务 A–E）。本条只记 B/C/D 与回归；A（R56 入账）在 commit `140ed0a`。

## 结论

| 项 | 修前 | 修后（本轮实测） |
|---|---|---|
| R19 用例计数判据 | 下限 `190 = 60 + 57 + 73`，删 388/579 条仍绿 | 精确 `598 = 306 + 59 + 233`，任何一张合成表都红（除精确快照） |
| `temp_path()` | `pid + name`，pid 复用 ⇒ 并发互相覆盖 | `pid + 纳秒 + name`，8 个同 pid 进程 8/8 绿 |
| R55 账本口径 | 语料 220、「文件全 ≤20000 B」 | 语料 214、体积口径限定到 R8/R9 门禁集合、登记两条残余 |

## P1（R55 评审 P1-1）：R19 计数门禁从下限改回精确相等

`docs/PITFALLS/round-data/R19/test.py:46-53`。R55 把「恒假的等式」换成了下限，等于把门禁变成恒真：R55 评审的
`gate_demo.py` 实测删掉 579 条里的 388 条（剩 191）仍然全绿。本轮改成：

```python
BASELINE = 598   # `cargo test --offline` 的用例总数（精确相等）
COUNTS = {"unittests src/lib.rs": 306, "unittests src/main.rs": 59,
          "tests/selftest/main.rs": 233}
```

`family_two()` 的三处断言（总数、每目标、selftest）全部 `==`。注释里写明：这是上一轮的精确计数，任何一轮增删
测试都必须在同一轮更新这里，否则本条立刻红；并点名 R55 的下限事故与 R55 评审 P1-1。**这带来一个有意的副作用**：
每轮都要动一次快照——这次改动本身就是「我知道用例数变了」的签名，而不是靠人记住。

### 有牙演示（`docs/PITFALLS/round-data/R57/gate_demo.py`，不改仓库）

做法：`importlib` 载入 `R19/test.py`，把 `cargo_test_targets` 换成合成计数表（`FakeRun(returncode=0)`），清空
`FAILS` 后调 `family_two()`，数它报了几条红。实测输出：

```
# R19 snapshot: BASELINE=598 COUNTS={'unittests src/lib.rs': 306, 'unittests src/main.rs': 59, 'tests/selftest/main.rs': 233}

row                                    total  verdict  red assertions
exact snapshot                           598  GREEN    0
total -1 (597)                           597  RED      2
total +1 (599)                           599  RED      2
lib -1 (305)                             597  RED      2
lib +1 (307)                             599  RED      2
main -1 (58)                             597  RED      2
main +1 (60)                             599  RED      2
selftest -1 (232)                        597  RED      3
selftest +1 (234)                        599  RED      3
R55 lower-bound snapshot (60/57/73)      190  RED      5
all tests deleted                          0  RED      5

PASS  gate has teeth: only the exact snapshot 598 = 306 + 59 + 233 is GREEN; 10 synthetic tables are RED
```

（每张 RED 表下面 demo 会列出红断言名，例如 `用例总数 == 598（精确相等）`、`` `unittests src/lib.rs` == 306 条
（精确相等）``、`` `unittests src/main.rs` == 59 条（精确相等）``、`` `tests/selftest/main.rs` == 233 条（精确相等）``、
`` `selftest` == 233 条（精确相等）``。`lib -1` 那行的 597 是总数 306-1+59+233=597 的合计，不是笔误。）

关键点：R55 自己的形状（`60/57/73`，总数 190）现在是红的（5 条红），「全部删除」（0 条）也是红的（5 条红）。
也就是说，如果有人把本文件再改回下限，这个 demo 立刻会红。

## P2：`temp_path()` 的并发假红（`tests/selftest/common/mod.rs:10-32`）

修前 `temp_path(name)` 用 `std::process::id()` + name。pid 会被 OS 复用，8 个进程/多次运行撞在一起时，一个测试
删掉或覆盖另一个测试的临时文件，`labels::visibility` 就偶发 `display: an accepted call writes the output`。
修法：新增 `process_tag()`（`OnceLock<String>`，内容 `format!("{}_{}", pid, nanos)`），临时文件名变成
`svg_grid_test_<pid>_<nanos>_<name>`。只用 std（`OnceLock` + `SystemTime`），没有新依赖。

**真证伪**（不是推理）：

- 用 `LD_PRELOAD` 的 C 垫片 `pid_t getpid(void) { return 4242; }` 把所有进程钉成同一个 pid；8 个进程同时跑
  `display_none_cannot_be_restored_by_a_descendant`，共享一个 TMPDIR。
- 修前二进制：由 `git archive 04042cb` 在 `/tmp/r57old` 单独构建（仓库未动）——
  `/tmp/r57old/target/debug/deps/selftest-f1a3992dfd88e812`；结果 **7/8 红**，
  `rcs=[101, 101, 0, 101, 101, 101, 101, 101]`，panic 在 `tests/selftest/labels/visibility.rs:40`：
  `read /tmp/r57oldtmp/svg_grid_test_4242_r48_vis_display_base.png: No such file or directory (os error 2)`，
  失败测试名 `labels::visibility::display_none_cannot_be_restored_by_a_descendant`。
- 修后二进制：同样 8 进程 **8/8 绿**（`rcs` 全 0，每份输出 `1 passed; 0 failed`）。

这条也进了 `R57/test.py` 的 C 族：默认跑 8 进程；`R57_PRE_FIX_SELFTEST=<修前 selftest>` 时额外跑一遍修前
二进制并断言它必须红（实测 `7/8 red rcs=[101, 101, 0, 101, 101, 101, 101, 101]`）。

## P3（R55 评审 P3）：R55 账本口径更正（`docs/PITFALLS/round-data/R55/meta.toml`）

1. 语料 220 → **214**：`git ls-files 'tests/**/*.svg'` = 214；`find` 数到 220 是因为混进了未跟踪的 scratch SVG。
   门禁跑的是跟踪集合，所以写 214，并注明 220 的来源。
2. 「文件全 ≤20000 B」→ 限定为「受 R8/R9 门禁的集合（`src/`、`tests/`、`scripts/` 的 `.rs`/`.py`）每个文件
   ≤ 20000 B」，并注明 `docs/PITFALLS/round-data/*/test.py` 不在该门禁内（R55/test.py 21693 B、R19/test.py
   20373 B、R38/test.py 23058 B 超限属既有口径，本轮不改）。
3. 末尾登记两条已知残余（只登记、本轮不修）：① 各向异性下纯 `scale(2 3)` 被 `approx_equals(total)` 早退、
   原样保留，残余 xor 1656/28（与 base 相同）；`scale(2 3) translate(3 4)` → `matrix(2 0 0 3 24 60)`，残余
   924/28（base 12762/255）。② href 基元素上的 transform 渲染器不认（三种取值渲染相同），R55 仍共轭它 ⇒
   「href 继承」用例只在属性层自洽，未被像素验证。

## 证据脚本：`docs/PITFALLS/round-data/R57/test.py`

四族，全 fail-closed（缺二进制/渲染器/编译器就红，除非 `--allow-skip`）：

| 族 | 断言 | 实测 |
|---|---|---|
| A | 读 R19 的 `BASELINE`/`COUNTS`，与真实 `cargo test --offline` 对比 | 598 = 306+59+233，逐目标相等；`== BASELINE` 在、`>= BASELINE` 不在、`190` 不在 |
| B | 跑 `gate_demo.py`，逐行断言 11 张表的 RED/GREEN | 只有 `exact snapshot` GREEN，其余 10 张 RED |
| C | 8 个同 pid 进程写同名文件必须互不覆盖；`R57_PRE_FIX_SELFTEST` 时修前必须红 | 修后 8/8 rc=0；修前 7/8 红，panic 指向 `tests/selftest/labels/visibility.rs` |
| D | grep R55 账本：214、无 220、体积口径限定、两条残余在 | 全绿 |

实测：
- 默认：`== 37 passed, 0 failed, 0 skipped ==`，rc=0。
- `R57_PRE_FIX_SELFTEST=/tmp/r57old/target/debug/deps/selftest-f1a3992dfd88e812`：`37 passed` 之外多两条 C 断言，rc=0。
- `SVG_GRID_BIN=/nope`：**rc=1**，红的是 `A/real suite count` 与 `C/concurrent temp_path`，理由
  `refusing to skip: missing release binary /nope (pass --allow-skip)`。
- `SVG_GRID_BIN=/nope ... --allow-skip`：rc=0，`== 28 passed, 0 failed, 2 skipped ==`。

（R56 P2.3 的教训是 `sys.exit(False)` 变 exit 0 的 fail-open；这里缺依赖一律 `check(False, …)` 并最终
`return 1`，`--allow-skip` 才降级成 skipped。）

## 回归（全部实测，本轮）

| 项 | 结果 |
|---|---|
| `cargo test --offline` ×3 | 每次 306 lib + 59 main + 233 selftest = **598**，0 failed（三次都是） |
| `cargo fmt --all --check` | 干净 |
| `cargo clippy --offline --all-targets -- -D warnings` | 干净 |
| `./install.sh` 后 sha256 | `aad3b2f44a822a140fe01187071097078174bb290df812d4726f17bcc163e2ae` 两侧一致（`target/release/svg_grid` / `~/.local/bin/svg_grid`） |
| `svg_grid doctor` | **57 pass / 0 fail / 1 skip**（skip：matplotlib/mathtext 旋转标签，已登记） |
| 语料 convert 对 `/tmp/r50base`（`e0c6033`，sha `75def0b8…`） | 214 文件，**diffs=0**（both-failed=39） |
| 语料 plot-grid 对同一 base | 58 fixture，**diffs=0**（both-failed=1） |
| 旧证据 `R49/run_old_evidence.sh` | **53 / 53 轮绿**（R3..R57，含本轮 R57；`ran.txt` 末行 `R57 OK`） |
| `pitfalls-manager check --repo .` | rc=0（R57/meta.toml 6 条 closures 解析通过） |

`sha256` 与 R56 相同（`aad3b2f4…`）是预期的：本轮只改测试与证据，`src/` 一个字节都没动。

## 偏离派单的地方（含理由）

1. **D2 的「文件全 ≤20000 B」在 R55 账本里根本没有这句**。我核对了 `R55/meta.toml` 全文与已被删掉的
   `docs/PITFALLS/paint-transform-lengths-not-scaled.md`（`git show 1db03ac^:…`），都没有这句。所以派单要求的
   更正被我实现为**新增一段正确口径**：只主张受 R8/R9 门禁的集合（`src/`、`tests/`、`scripts/` 的 `.rs`/`.py`）
   每个文件 ≤ 20000 B，并列出 `round-data/*/test.py` 的超限文件名与字节数。没有去改一句不存在的原文。
2. **R55 的两条残余数字（1656/28、924/28、12762/255）直接采用评审 P3 的值，本轮没有重跑渲染器复测**——派单说
   「只登记不修」，重测要拉 d1render 与 base 两套对照，属下一轮的活。报告与账本都写明了这是「R55 评审 P3 登记」。
3. **R19 的 `BASELINE` 用了 598 而不是 579**：R55 评审时的 579 是当时的值，本轮真实是 598（306+59+233）。
   派单给的正是 598/306/59/233，与实测一致。
4. **R57 的 finding 数定为 6 条**（派单只说「覆盖 B/C/D」）：F1 R19 精确快照、F2 牙齿演示、F3 `process_tag()`、
   F4 R55 语料 214 + 体积口径、F5 两条残余登记、F6 R57 证据脚本。把「口径更正」和「残余登记」拆成两条是因为
   它们是两次独立的判断，合成一条会让下一个人看不出哪条是被迫修的、哪条是主动补的。
5. **R57/test.py 的 C 族在默认运行里没有「修前必须红」这条**：修前二进制要单独构建，默认路径不该依赖 `/tmp`
   里的历史产物。所以它写成可选 env（`R57_PRE_FIX_SELFTEST`），默认不跑、设了就跑并断言红。这与「有牙」要求
   不冲突——牙在演示里，也在 A/B 的精确断言里。
