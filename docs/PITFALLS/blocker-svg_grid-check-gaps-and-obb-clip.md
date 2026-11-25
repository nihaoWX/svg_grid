# R67：非有限值门漏掉 13 类 composer 真会改写的属性 + obb clipPath body 被缩放

- 轮次：R67（消费 R65 评审的三条 must-fix）
- 基线：`4626f31`（R66 收口）；修复 commit 见账本 `R67.code_after`
- 证据：`docs/PITFALLS/round-data/R67/{test.py,meta.toml}`（82 checks，A/B/C/D/T 五族）

## 1. 现象

1. **A（P1）**：`src/convert/finite.rs::unusable` 只扫几何表（circle/ellipse/text/use/line/rect/image +
   stroke-width），而组图路径还会解析 tspan、渐变、pattern、mask、filter、marker、fe* 的坐标。
   于是 `<text x="10" y="20" font-size="10">a<tspan x="1e999" y="30">b</tspan></text>`
   在 `convert --check` 下 **rc=0 "compatible"**；`plot-grid` 只打一条 W7 告警
   （`1 attribute could not be rewritten, kept verbatim: input #0 <tspan x>: non-finite number "1e999".`）
   就把 `x="1e999"` 原样写进产物——渲染器拿到的是 inf。
2. **B（P1）**：`clipPathUnits="objectBoundingBox"` 的 body 是**分数域**，但
   `src/transform/walk.rs:346-350` 只把 `child_body` 置 false，子元素仍按 panel 的 user-space `policy`
   走 `dispatch.rs::rewrite_rect`/`rewrite_scaled_length`：`<rect width="1" height="0.5">` 变成
   `width="2" height="1"`，`<rect>` 的默认 x/y 还被 materialise 成 `x="0" y="0"`。
   d1render 实测 source ink 16000 vs product 32000，xor 32000/128000（纯色与 pattern 两种 paint 都坏）。
3. **C（P1）**：R65-F12 的 reason 说「cairosvg 不裁剪、ink 8000、0/0 形式不存在」——R65 评审证伪：
   cairosvg 正常裁剪（user rect/path/group/obb/circle 墨 2000/2000/2000/4000/1308 vs 1324），
   「8000」是把透明背景 `(0,0,0,0)` 当成了墨（真实 4000），且 0/0 形式存在（分数 verbatim + 删 x/y）。
4. **D（P2）**：`docs/PITFALLS/round-data/R65/gate_assertions.py` 可被绕过：
   `{ let _ = "assert!(true)"; }`、`{ let _ = r#"assert!(true)"#; }`、`{ // assert!(true) }` 都 PASS；
   `#[cfg_attr(test, ignore)]` 也不拦（`IGNORE_ATTR` 只匹配 `#[ignore`）。
   另外 `convert` 写模式失败时 stderr 只有 `1 problem(s)`，不说是哪条（`--check` 却逐条打印）。

## 2. 复现步骤

```bash
cd <repo>
# A：修前 rc=0（漏洞），修后 rc=1 且点名 x="1e999"
printf '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">\
<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>\
<g data-scale="xy"><text x="10" y="20" font-size="10">a<tspan x="1e999" y="30">b</tspan></text></g></svg>' \
  > /tmp/tspan.svg
svg_grid convert --input /tmp/tspan.svg --check; echo rc=$?

# B：修前产物 clip body 是 width="2" height="1" x="0" y="0"；修后是 width="1" height="0.5"（无 x/y）
#    完整对照见 round-data/R67/test.py 的 B 族（d1render + cairosvg 的 xor/ink）

# D：修前 3 种绕过 PASS、cfg_attr(ignore) 不拦；修后 --self-test 4 条全红
python3 docs/PITFALLS/round-data/R65/gate_assertions.py --self-test; echo rc=$?
```

## 3. 修法

- **A**：`finite.rs` 改成表驱动 `CLASSES`（tag + attrs + units 门 + strict），
  `unusable(element, ids, primitive_user_space)`；units 门按解析后的
  `gradientUnits`/`patternUnits`/`maskUnits`/`filterUnits`/`markerUnits`/`primitiveUnits`
  （含 `href` 继承与环保护）决定是否在类；strict 类用 `classify_strict`（非数/非有限都拒），
  `pattern` 用非有限-only（`tile_number` 对 `50%` 静默保留，见 §6 偏离）。
  `scan.rs` 的 `collect_ids` 改成 `HashMap<String, &Element>` 以支持 href 解析。
  R65 的 10 条旧测试原在 `finite.rs` 内，加表后文件超 20000 B，故原样搬到
  `src/convert/tests_finite.rs`（断言未改，计数不变）。
- **B**：在 `walk.rs` 的 gradient 分支之后加 obb clipPath 的**整棵子树早退**（与 obb pattern body 同策略），
  并删掉旧的 `child_body = false` 块。**不**把 clipPath 加进 `domains::body_is_verbatim`：
  `defs::split` 的 refs 定点会把 body 引用的定义也标成 `Slot::Verbatim`，clip body 里的 userSpace
  pattern 就不再 8→16 缩放，实测 0/0 会退回 10984/21968/255（R64 的旧症状）。
- **C**：账本 append-only、finding 字段不可改 ⇒ 本轮 `record R67` 新增 `R67-F3`，`where` 指向
  `round-data/R65/meta.toml` 的 F12 reason 段，reason 写明证伪与修法；再 `modify R65 --by R67`。
- **D**：`gate_assertions.py` 扫描前 `strip_noise()` 清空注释、字符串/raw-string/char 字面量（保留换行，
  行号与 `#[test]` 定位不变）；`IGNORE_ATTR` 加 `cfg_attr(... ignore ...)` 分支；每种绕过一条自测
  （`--self-test`，4 条必须全红并点名 file+test）。`cli.rs` 的写模式失败分支改为
  `print_incompat_report(err, &findings)`，findings 逐条进 stderr，末尾仍是原来的
  `... is not compatible ...; N problem(s); no output written`。

## 4. 四列对照（修前 / 修后 / 参考 / 证据）

| 项 | 修前 | 修后 | 参考 / 证据 |
|---|---|---|---|
| A：13 类 `--check` | 13/13 rc=0（放行） | 13/13 rc=1，逐条点名 `attr="1e999"` | R65 评审 §2；`R67/test.py` A 族 |
| A：tspan 例子 | rc=0 "compatible" | rc=1，`<tspan> #3 at svg/text/tspan: geometry attribute x="1e999" is not a finite number...` | 派单验收 |
| A：写模式 | rc=0 写产物 | rc=1、无产物、哨兵 `SENTINEL` 未被截断 | `R67/test.py` A 族 |
| A：合法控制 5 条 | rc=0 | rc=0（obb 50% 渐变/pattern、userSpace pattern 50%、marker 默认 strokeWidth、feTile primitiveUnits=obb） | 不误拒 |
| A：href 继承 3 条 | rc=0 | rc=1（gradient/pattern/filter 的 units 在 base 上） | `R67/test.py` A 族 |
| B：clip body | `width="2" height="1" x="0" y="0"` | `width="1" height="0.5"`，无 x/y | 分数 verbatim |
| B：d1render xor | 32000/128000（纯色与 pattern） | 0/0 | `R67/test.py` B 族 |
| B：d1render ink | 16000 vs 32000 | 16000/16000 | 同上 |
| B：cairosvg | xor 32000/128000 | xor 0/0，ink 16000/16000 | 同上 |
| B：userSpace clip | 正常改写 | 仍 `x="20" y="20" width="40" height="40"` | 无回归 |
| B：clip 引用的 pattern | 8→16 | 仍 8→16（`width="16" height="16"`） | 无回归 |
| D：gate 绕过 | 3 种 PASS、cfg_attr 不拦 | `--self-test` 4 条全红；全树 `PASS 772` | `R67/test.py` D 族 |
| D：写模式 stderr | 只有 `1 problem(s)` | findings 逐条 + `1 problem(s)` | `R67/test.py` D 族 |
| 测试总数 | 747 = 452 + 62 + 233 | **772 = 477 + 62 + 233** | `cargo test --all-targets -- --list` |
| R19 快照 | `BASELINE=747` | `BASELINE=772`，COUNTS 477/62/233 | `R19/test.py` rc=0 |
| R57 快照 | 747 = 452 + 62 + 233 | 772 = 477 + 62 + 233 | `R57/test.py` rc=0 |
| R64b 快照 | 432 / 59 / 233 | 477 / 62 / 233 | `R64b/test.py` rc=0 |
| 214 convert | — | 对 `/tmp/r50base`（@`e0c6033`）rc+产物逐字节 0 差 | `R67/test.py` C 族 |
| 58 plot-grid | — | 同上 0 差 | 同上 |
| doctor | 57/0/1 | 57/0/1 | `svg_grid doctor` |
| fmt / clippy | — | `cargo fmt --all --check` rc=0；`clippy -D warnings` rc=0 | 本轮 |
| install sha | — | `target/release` 与 `~/.local/bin` 同为 `9d9a7b30…` | 本轮 |

## 5. 属性 → 改写点（判据来源，不是抄派单）

| 类 | 属性 | composer 的改写点 |
|---|---|---|
| `tspan` | x, y | `src/transform/dispatch.rs:80-91`（`rewrite_axes`） |
| `linearGradient` | x1, y1, x2, y2 | `src/transform/dispatch.rs:169-186`（`gradientUnits=userSpaceOnUse` 门） |
| `radialGradient` | cx, cy, fx, fy | `src/transform/dispatch.rs:201-215` |
| `radialGradient` | r, fr | `src/transform/dispatch.rs:216-218`（`rewrite_scaled_length` Uniform） |
| `pattern` | x, y, width, height | `src/transform/dispatch.rs:241-250` → `src/transform/patterns.rs:260`（`rewrite_pattern_tile`） |
| `mask` | x, y, width, height | `src/transform/dispatch.rs:276-281` → `src/transform/primitives.rs:371`（`rewrite_region`） |
| `filter` | x, y, width, height | `src/transform/dispatch.rs:291-296` → `src/transform/primitives.rs:371` |
| `marker` | markerWidth, markerHeight, refX, refY | `src/transform/walk.rs:238-252` → `src/transform/primitives.rs:412`（`rewrite_marker_box`） |
| `fe*` | x, y, width, height | `src/transform/filters.rs:233-250`（`primitiveUnits=userSpaceOnUse` 门） |

**不在类**（composer 不解析或只做非数值语义，语义不变）：`rect` rx/ry、`text`/`tspan` dx/dy、
`stroke-dasharray`/`stroke-dashoffset` 的属性形式、`textPath` startOffset、`pathLength`、CSS 规则、
`feComposite` k1/k4、`feConvolveMatrix` order/divisor/bias/targetX/kernelMatrix、
`feDistantLight` azimuth/elevation、`feSpotLight` limitingConeAngle/specularExponent、
`feDiffuseLighting` surfaceScale/diffuseConstant、`feSpecularLighting` specularConstant、
`feComponentTransfer` slope/intercept/amplitude/exponent、`feTurbulence` seed。
已登记进 `ARCHITECTURE/convert.md` §6 与 `round-data/R67/meta.toml` 的 F8。

## 6. 变异（≥4，实跑，列红名）

| # | 变异 | 实跑 | 结果 |
|---|---|---|---|
| M1 | `finite.rs` 的 `for class in CLASSES` → `&CLASSES[..0]` | `cargo test --offline --lib tests_r67` | **14 failed**（13 类 + 1 条 feTile width） |
| M2 | `walk.rs` 的 obb clipPath 早退加 `false &&` | 同上 | **3 failed**（obb clip body verbatim、font-size、pattern 仍缩放） |
| M3 | `gate_assertions.py` 的 `strip_noise` 直接 `return text` | `--self-test` | **3 条自测红**（string-literal / raw-string / comment） |
| M4 | `IGNORE_ATTR` 退回只匹配 `#[ignore` | `--self-test` | **1 条自测红**（cfg-attr-ignore） |
| M5 | `cli.rs` 写模式不打印 findings | `convert` 写模式 | stderr 只剩 `1 problem(s)`，无 `x="1e999"` 行 |

## 7. 回归面

- 214 convert（含 `--report`）+ 58 plot-grid（含 `--plan`）对 `/tmp/r50base/target/release/svg_grid`
  （@`e0c6033`）rc + 产物逐字节 0 差；唯一差异是 2 个本来就不兼容的语料
  （`tests/fixtures/matplotlib/mathtext.svg`、`tests/mpl_dialect/failclosed_rotated_mathtext.svg`）
  的 stderr 多了 D 的 findings 块，且修后 stderr 以修前 stderr 结尾（只前置、不改旧行）。
- 旧证据 R3…R66 逐轮 rc=0（R19 的 ⑤ 内部逐轮跑 R4–R17；R20–R66 由本轮循环逐个跑，见 `ran.txt`）。
  其中 6 轮的期望值被本轮**有意**改写，均已同轮更新并在 `R67/meta.toml` 的 `touched` 登记：
  R59（E4 由「像素不变」改为 `after == source@2`，因为 obb clip 不再为空）、
  R62（C/m4 登记值 `(16000, 255)` → `(0, 0)`，R67-B 收敛）、
  R63（fail-closed 断言改为逐行找 `svg_grid convert: ` 前缀）、
  R64b（第三处写死计数 432/59/233 → 477/62/233）、
  R65/R66（214 convert 的 stderr 比较改为「修后以修前结尾」）。
  这 6 处都是断言口径随行为/输出变化，不是放宽：R59 反而新增了像素 oracle，R62 是把「没收敛」改成「收敛」。
- 测试只增不减：747 → 772（+25：`tests_r67.rs` 21 条 + transform 4 条）。
- `cargo fmt --all --check` rc=0；`cargo clippy --offline --all-targets -- -D warnings` rc=0；
  `./install.sh` 后两侧 sha256 同为 `9d9a7b30cef8c9782a06de1af2d145c6c7c2a70452e327feaa26d141ee2fa9d5`；
  `doctor` 57 pass / 0 fail / 1 skip。
- 单文件 ≤20000 B：`R67/test.py` 19991 B、`R67/meta.toml` 9019 B、`ARCHITECTURE/convert.md` 约 10.4 KB；
  `finite.rs` 加表后一度 20619 B（R8 的体积检查抓到），把 R65 的 10 条测试拆到
  `src/convert/tests_finite.rs`（5408 B）后 `finite.rs` 14964 B，断言一条未改。

## 8. 账本

- `record R67`：`R67-F1`（A 13 类）、`F2`（B obb clip）、`F3`（C 更正 R65-F12）、`F4`（D gate 绕过）、
  `F5`（D 写模式 stderr）、`F6`（R19/R57/R64b 快照）、`F7`（R67 证据）、`F8`（不在类登记）。
- `modify R19 --by R67`、`R57`、`R59`、`R62`、`R63`、`R64b`、`R65`、`R66`（期望值被本轮有意改写的 8 轮）。
- `check --repo .` 空输出、rc=0。

## 9. 自述偏离

1. **A 的判据选「units 门控的类表」而不是「所有数值属性一律拒」**：合法的 obb 百分比
   （`<radialGradient r="50%">`、`<pattern x="50%">`）是 composer 的合法输入，一律拒会误杀；
   只有解析后的 units 让 composer 真的 `parse_f32` 时，非数/非有限才必然坏。非有限值在任何 units
   下都不是合法 SVG 数，所以 strict 类对 `1e999` 一律拒。零误拒证据见 §4（214+58 0 差 + 5 条合法控制）。
2. **B 用 walk 层早退，不用 defs 层 `Slot::Verbatim`**：理由见 §3（refs 定点会把被引用的 pattern 也标
   Verbatim，0/0 反而坏掉）。
3. **`pattern` 的 strict=false**：`tile_number` 对 `50%` 解析失败就静默保留，只有 `1e999`→inf 在产物里
   非法；若按 strict 拒 `50%` 会误杀合法输入。
4. **改了 `ARCHITECTURE/convert.md`**（加 §6）：派单 A 明确要求登记「已知不在类」；HARNESS 其余文件
   （`AGENTS.md`/`INSTRUCTIONS.md`/`PROGRESS.md`）未动。
5. **`--report` 单独使用仍报 "missing required --output"**：这是既有行为，不在本轮范围，未改。
