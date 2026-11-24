# 阻塞点：`convert --check` 对非有限数判 compatible；计数门看不见被掏空的 `#[test]`

- 轮次：R65
- 日期：2026-11-25
- 代码前：`bfe5e12`（HARNESS 收口；代码同 `571efec`）
- 类别：① 工具缺陷（两处）

## 现象

### 1. 非有限数被放行（R63 评审 P2）

`svg_grid convert --check` 对下列输入全部 `rc=0`、报 `compatible`，写模式还会把非法数值原样写进产物：

| 输入 | 修前 | 修后 |
|---|---|---|
| `stroke-width="1e999"` | rc=0，产物里 `stroke-width="1e999"` | rc=1，`geometry attribute stroke-width="1e999" is not a finite number, so the composer cannot rewrite it` |
| `x="1e999"` | rc=0，产物里 `x="1e999"` | rc=1，同上句式点名 `x` |
| `d="M 1e999 0 L 10 10"` | rc=0，产物被展平成 `points="inf,NaN 10,10"` | rc=1，点名 `d="1e999"` |
| `font-size="1e999"` | rc=0 | rc=1，点名 `font-size` |
| `textLength="1e999"` | rc=0 | rc=1，点名 `textLength` |
| `<stop offset="1e999">` | rc=0 | rc=1，点名 `offset` |
| `points="1e999,10 20,20"` | rc=0 | rc=1，点名 `points` |
| `stroke-width="NaN"` / `"-inf"` | rc=0 | rc=1，点名属性与值 |

复现：

```bash
cat > /tmp/bad.svg <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect x="10" y="10" width="20" height="20" fill="#000" stroke-width="1e999"/>
</svg>
EOF
svg_grid convert --input /tmp/bad.svg --check          # 修前 rc=0 "compatible"
svg_grid convert --input /tmp/bad.svg --output /tmp/o.svg   # 修前 rc=0，产物里留着 1e999
```

`plot-grid` 事后只会打 W7（「值被保原文」），所以 `--check` 的 compatible 承诺不成立：产物里是非法数值，
`d` 那条更糟——composer 自己写出 `inf`/`NaN`。

### 2. 计数门看不见被掏空的测试（R19 的洞）

R19 的 `test.py` 只钉「每个 target 有几条 `#[test]`」。把任意一条测试的函数体换成 `{}`，计数不变，
`cargo test` 依旧全绿，R19 也依旧全绿——「同名掏空」因此不可见。

## 根因（已证）

1. `src/convert/scan.rs::non_numeric_geometry` 的判据是 `value.trim().parse::<f32>().is_err()`。
   `1e999` 解析**成功**得 `inf`，`NaN` 解析**成功**得 `NaN`，`is_err()` 因此为假 ⇒ 漏判。
   判据只问「能不能 parse」，没问「parse 出来是不是有限数」。
2. 覆盖也不全：只查了部分图元的几何属性，`path` 的 `d`、`points`、`<stop offset>`、fe* 的数值、
   内联 `style` 里的长度 token 都没查；根 `<svg>` 的 width/height 与 viewBox 也没查（composer 在
   viewBox 决定画布时仍把根 width/height 原样抄进产物）。
3. R19 的计数门是「数量」而不是「内容」：`#[test]` 的数量与函数体是否断言无关。

## 修法

- 新增 `src/convert/finite.rs`：判据改成「解析失败 **或** `!is_finite()`」，覆盖
  circle/ellipse/text/use/line/rect/image 的几何属性、`polyline|polygon` 的 `points`、
  `path` 的 `d`（逐 token，命令字母与 `Z/z` 不误报）、`stroke-width`、`font-size`、`textLength`、
  `<stop offset>`、fe* 的 `stdDeviation`/`dx`/`dy`/`radius`/`baseFrequency`/`scale`、内联 `style`。
  单位后缀 `px`/`%` 先剥再判；`font-size`/`textLength`/`offset` 只判数字前缀（`40px` 合法）。
- `src/convert/path.rs` 新增 `non_finite_numbers(d)`：复用已有的 `read_number`，畸形文本不报
  （那是 `tokenize` 的职责，既有 `failed to flatten <path d=…>: missing argument for path command 'M'` 文案不变）。
- `src/convert/geom.rs::canvas_box` 加三处 fail-closed：根 width/height 非有限、viewBox 里有非有限、
  无 viewBox 时画布尺寸非有限（第三处实测不可达，已删并留注释说明由第一处兜住）。
- 新增 `docs/PITFALLS/round-data/R65/gate_assertions.py`：逐条读 `src/**/*.rs` 与 `tests/**/*.rs` 里每个
  `#[test]` 的函数体（花括号配对，跳过字符串/raw string/字符字面量/生命周期/注释），要求至少一次断言式调用
  （`assert*`/`debug_assert*`/`panic!`/`unreachable!` 宏，或 `.expect(`/`.unwrap(`/`.unwrap_err(`，或调用一个
  自身含断言宏的树内函数——只跟一跳），并禁止 `#[ignore]`；失败时点名「哪个文件、哪条测试」。
  R19 的 `family_two` 在计数检查之后调用它。

## 为什么这样修不弄坏相邻代码

- composer 的 W7 口径（畸形但**有限**的值保原文）不动；新门只加在 convert 的 scan 阶段。
- 语料 214 个 `convert`（含 `--report`）与 58 个 fixture 的 `plot-grid --width 600 --ncol 1`（含 `--plan`）
  对 base `e0c6033` 逐字节 0 差。
- 合法值全部仍合法：`1e-45`、`0`、`-0`、`.5`、`1.`、`+1`、`1E3`、`5px`、`50%`、`40px`、`medium`。
- 既有 W7 fixture 用 `x="inf"` 触发「值被保原文」，而 doctor 是**在进程内**调转换器的，新门一上这些 fixture
  在 convert 阶段就 rc=1。断言不动，只把触发值换成「畸形但有限」的 `feMorphology radius="2 3 4"`
  （三个数，要求一或两个）：`tests/selftest/common/harness.rs` 的 `W7_PANEL`、
  `tests/selftest/doctor/masking.rs` 的 `W7_MASKED_PANEL`、R45 的 `FIXTURES["w7"]`、R53 的 `W7_BODY`/`w7clean`、
  R63 的 `W7SVG`（它是唯一走 `auto_convert` 的，原来的 `stroke-width="1e999"` 必须去掉）。

## 证据

`docs/PITFALLS/round-data/R65/test.py`（58 checks，A/B/C/D/E/T 六族，默认 rc=0）：

```bash
cd <repo> && python3 docs/PITFALLS/round-data/R65/test.py
SVG_GRID_PIN_BIN=/tmp/r65_before_bin python3 docs/PITFALLS/round-data/R65/test.py   # A 族 18 条红
```

修前二进制：`git worktree add --detach /tmp/r65base bfe5e12 && cd /tmp/r65base && cargo build --release --offline`
⇒ sha256 `0ff7d7750e3d5c4af6ab0ae6ec9ddcf6e460fc4c8421606d276778eb392f7f03`。

变异（实跑，红名见 `meta.toml`）：把 `classify_strict` 退回 `is_err()` ⇒ 12 条红；把守卫的空函数体分支删掉
⇒ C 族 `empty-body` 红；掏空函数体 / 删断言留 `let _ =` / 加 `#[ignore]` ⇒ 守卫分别点名文件与测试。

## 不在本轮

`tspan` 视觉序、W6 假阳性、`--rel-widths` 0 宽、obb clipPath 的实现（只做一次判定实验：建不出有判别力的
fixture，见 `meta.toml` 的 family 第 6 条）、composer 侧 `src/path/parse.rs` 的同类入口（由 R60/R61 的 W7 口径覆盖）。
