# svg_grid 阻塞点 — `classify` 取表序第一个命中码，用户内容可把 W9/W7 门禁打成 PASS

场景：`tools/svg_grid` HEAD `54fa2b6`（2026-11-24，R46 之后）。R45 把告警短语表搬进
`src/warnings/codes.rs` 后暴露的**既有**缺陷（base `2d76da3` 同样存在，R45 只是把可遮蔽短语
从 5 条扩到 7 条）。

## 现象

`src/warnings/codes.rs:50-68` 的 `classify` 对每条告警行只取**表序第一个**命中的码：

```rust
if let Some((code, _)) = WARNING_CODES
    .iter()
    .find(|(_, phrases)| phrases.iter().any(|phrase| line.contains(phrase)))
```

而 W9 的行原样打印悬空 id、W7 的行原样打印属性值，两者都是**用户内容**：

```
svg_grid: warning: W9: 1 reference point at an id that nothing in the output defines: url(#<用户 id>)
svg_grid: warning: W7: 1 attribute could not be rewritten, kept verbatim: input #0 <rect x>: <用户值>
```

于是只要用户 id / 属性值里出现任何**表序更靠前**的短语（W1 `scaled by the layout`、
W2 `stretched: `、W4a `--labels has `、W4b `repeats some letters`、W5 两条、W6
`draws ink outside its own canvas box`），`classify` 就返回那个码，W9/W7 消失。

## 复现步骤

```bash
cd /tmp && rm -rf r49probe && mkdir -p r49probe/fx && cd r49probe
cat > fx/w9.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
  <rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>
  <rect x="10" y="10" width="50" height="40" fill="url(#--labels has 3 entries)"/>
</svg>
SVG
svg_grid doctor --fixtures fx
```

- base `2d76da3`：`w9 (step=convert, expect=pass)` **FAIL**，`--json` 的 `warning_codes` 含 `W9`。
- HEAD `54fa2b6`：同一 fixture **PASS**，`--json` 的 `warning_codes` 是 `['W4a']`。

W7 侧同理（`<rect stroke-width="inf">` 干净时 FAIL，同一文档再加
`<path d="M0 0 --labels has 3"/>` 就重新 PASS `['W4a']`）。

## 影响

- doctor 是**唯一**的 `classify` 调用方，它的判据是"fixture 的 stderr 里出现期望的码"。
  码被抢走 ⇒ 该 fixture 从 FAIL 变 PASS ⇒ **门禁 fail-open**。
- 触发需要用户内容恰好含这些英文短语；58 条语料 0 命中，所以不是当前回归，但它是
  "门禁可被输入击穿"这一类问题，与 R45 想修的"死短语导致门禁静默失效"是同一族。
- 净变化：W7 从"表里根本没有"变成"有但可被遮蔽"（改进）；W9 的可遮蔽面从 5 条扩到 7 条（变差）。

## 建议修法

1. `classify` 返回**全部**命中的码（`filter` 而非 `find`），doctor 判"期望码 ∈ 命中集合"。
   这样遮蔽不再可能，代价是同一行可能同时报两个码（可接受，doctor 只做集合包含）。
2. 或者按码的**稳定前缀**判（`W9:` / `W7:` 出现在行首附近），但现有文案没有统一前缀，
   需要同时改所有 `describe_*`，改动面更大。
3. 无论选哪种，加一条守卫测试：构造"W9 行的 id 里含 W4a 短语"的 stderr，断言 `W9` 仍在结果里
   （跨码短语不得互相遮蔽）。

## 同族遗留（同一轮顺手收）

- `src/warnings/rules_tests.rs:236-266` 的 `no_cause_note_repeats_a_warning_phrase` 里手写
  9 条短语，是 `WARNING_CODES` 的**第三份副本**，已与表不一致（副本有 `could not be rewritten`、
  `already carry letters`、`not in the font library`；表里是 `could not be rewritten, kept verbatim`，
  且已无 W3/W8）。变异实测：给 W1 加一条新短语 `the layout rescaled it` ⇒ 419 全绿，副本不跟表走。
- `docs/PITFALLS/round-data/R17/test.py:44-46` 的 `KNOWN_CODE_WORDS` 是死变量（定义了没人用）。
- R45 使旧行号再漂但未记 `modify`：`src/warnings/mod.rs:emit` 198-211→202-215（账本 R31-F3 记
  189-202）、`assemble_within` 257-279→261-283、`src/lib.rs:plot_grid_with_diagnostics`
  154-327→155-328（账本 152-325）。
