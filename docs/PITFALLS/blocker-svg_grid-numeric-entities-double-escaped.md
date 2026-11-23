# svg_grid 阻塞点 — 数字实体（`&#65;` / `&#x42;`）被双重转义，文字几乎全部丢失

场景：`tools/svg_grid` HEAD `a053283`（2026-11-24，R44 之后）。`svg_grid plot-grid` 与 `svg_grid convert` 都受影响。

## 现象

`src/dom.rs:151-158` 的 `resolve_reference` 只认五个命名实体：

```rust
fn resolve_reference(reference: &str) -> String {
    match reference {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => format!("&{reference};"),   // ← 数字实体落到这里
    }
}
```

`quick-xml` 把 `&#65;` 作为 `Event::GeneralRef` 交出来，`reference` 是 `"#65"`，于是被还原成字面量
`&#65;`；序列化时 `&` 又被转义成 `&amp;` ⇒ 产物里是 `&amp;#65;`，渲染器把它当**可见文本**画出来，
而不是字符 `A`。

## 复现步骤

```bash
cd /tmp && cat > deg.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="200" height="80">
  <rect data-panel-box="main" x="0" y="0" width="200" height="80" fill="none"/>
  <text x="10" y="40" font-size="20" font-family="Arial">37&#176;C &#215;2 &#177;1</text>
</svg>
SVG
~/.local/bin/svg_grid plot-grid --input deg.svg --width 400 --ncol 1 --output dego.svg
grep -o '<text[^>]*>[^<]*</text>' dego.svg
# <text ...>37&amp;#176;C &amp;#215;2 &amp;#177;1</text>   ← 应为 37°C ×2 ±1
```

渲染对比（cairosvg，白底）：

| 文件 | 尺寸 | 暗像素 |
|---|---|---|
| `deg.svg`（源） | 400×160 | **64000** |
| `dego.svg`（产物） | 400×189 | **995** |

即：文字几乎全部丢失（只剩 `37`、`C`、`2`、`1` 这几个 ASCII 字符），而且画布高度也变了（文字变宽 ⇒
换行/度量不同）。

十进制、十六进制、`&nbsp;`、`&mdash;` 全部中招：

```
dec: &#65; hex: &#x42; deg: &#176;   →   dec: &amp;#65; hex: &amp;#x42; deg: &amp;#176;
nbsp: &#160; mdash: &#8212;          →   nbsp: &amp;#160; mdash: &amp;#8212;
```

五个命名实体（`&amp; &lt; &gt; &quot; &apos;`）正常。

## 两处实现都有（不是一处）

`src/dom.rs:150-158`（主库）与 `src/convert/dom.rs:146-154`（转换器）各有一份**完全相同**的
`resolve_reference`，所以 `svg_grid convert` 也中招：

```bash
~/.local/bin/svg_grid convert --input cent.svg --output cent.out.svg   # rc=0
grep -o '<text[^>]*>[^<]*</text>' cent.out.svg
# <text ...>37&amp;#176;C</text>
```

修的时候两处都要改（`ARCHITECTURE/tag-protocol.md` 已登记"path 双实现防漂移"，DOM 解析是第二对）。

## 范围：只有文本节点中招，属性值正常

属性值走 `quick_xml` 的 `decode_and_unescape_value`（`src/dom.rs:128`），数字实体在那里已被正确解码：

```bash
# <rect x="&#49;0" y="10" width="20" height="20"/>
svg_grid plot-grid --input attr.svg --width 200 --ncol 1 --plot-margin 0 --output o.svg
grep -o '<rect x="[^"]*" y="[^"]*"' o.svg     # x="10"  ← 正确
```

只有 `Event::GeneralRef`（文本节点里的实体）走 `resolve_reference`。所以修的时候只需改
`resolve_reference` 这一处（两个文件各一份），不必动属性解析。

## 影响面

- `tests/fixtures/**/*.svg`：**0 个**含 `&#`（所以 220 fixture 回归抓不到）。
- `research/瘤内菌群组图/Figure {2,3,8}.svg`：37080 个 `&#10;`，但都在 base64 图像数据里（换行），
  不影响渲染。
- 真实风险在**带单位/符号的图注文字**：`37&#176;C`、`&#215;2`、`&#177;1`、`p&#8201;<&#8201;0.05`
  这类是生物医学图里最常见的写法（R/ggplot2 的 `ggsave` 与部分 matplotlib 后端会输出数字实体）。

## 建议

`resolve_reference` 补数字实体解析（`#123` 十进制、`#x1F` 十六进制），按 XML 规范映射到字符；
无法解析的（越界、非法码点）保留原样或 fail-closed 报错。修完补测试：十进制/十六进制/越界/非法，
以及"产物渲染后与源图逐像素一致"的对照（不能只断 markup）。
