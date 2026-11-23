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

## 为什么一直没被发现：`resolve_reference` 零测试

`grep -rn 'resolve_reference' --include=*.rs src/ tests/` 只命中两个定义处与两个调用处，
**没有任何测试**直接或间接覆盖它；`tests/fixtures/**` 里也 0 个 `&#`。所以这条路径从建立起
就没有被验证过——五个命名实体恰好对，是因为它们与 XML 规范一致；数字实体是唯一的分支，
而它写错了。

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

## 追加实测（2026-11-24，R47 派单前；含两条更正）

### 更正 1：CDATA 不是缺陷

`<text><![CDATA[37&#176;C]]></text>` 经 `svg_grid convert` ⇒ 产物 `37&amp;#176;C`，但**渲染与源图逐像素
相同**（resvg 300×40，diff 0）。CDATA 里 `&#176;` 本来就是字面量，序列化成 `&amp;#176;` 后渲染出同样的
字面量 ⇒ 行为正确，不要"修"。

### 更正 2：`&unknown;` 在文本里静默、在属性里 fail-closed（口径不一致）

| 输入 | 结果 |
|---|---|
| 文本 `a &unknown; b` | rc=0，产物 `a &amp;unknown; b`（静默保留字面量） |
| 属性 `data-x="a&nbsp;b"` | rc=1 `XML error: at 2..6: unrecognized entity `nbsp`` |
| 文本 `a & b`（裸 `&`） | rc=1 `XML error: ill-formed document: entity or character reference not closed` |

两个渲染器对源图里的 `&nbsp;` 都直接拒绝（resvg `unknown entity reference 'nbsp'`、cairosvg
`ParseError undefined entity`）⇒ 含未定义命名实体的源图本来就不可渲染，静默保留只会把"不可渲染"
变成"渲染出字面量"。建议与属性路径一致：**fail-closed**。

### quick-xml 0.39.4 的权威语义（不要手写数字解析）

`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/quick-xml-0.39.4/src/escape.rs`：

- `:2038-2051` `parse_number`：`#x` 前缀走 16 进制、否则 10 进制；`code == 0` ⇒ `IllegalCharacter`；
  `char::from_u32` 失败 ⇒ `InvalidCodepoint`。
- `:2054-2061` `from_str_radix` 拒绝 `+`/`-` 符号（`UnexpectedSign`）。
- `:212-215` `pub fn unescape(raw: &str) -> Result<Cow<'_, str>, EscapeError>` =
  `unescape_with(raw, resolve_predefined_entity)`；`:255-300` `unescape_with` 先处理 `#` 数字引用
  （不可覆盖），再 `resolve_entity`，未识别 ⇒ `EscapeError::UnrecognizedEntity`。

实测（`quick_xml::escape::unescape(&format!("&{};", reference))`）：

```
#65      -> Ok("A")          #x42     -> Ok("B")        #176 -> Ok("°")
amp      -> Ok("&")          nbsp     -> Err(UnrecognizedEntity(1..5, "nbsp"))
#0       -> Err(InvalidCharRef(IllegalCharacter(0)))
#x110000 -> Err(InvalidCharRef(InvalidCodepoint(1114112)))
#55296   -> Err(InvalidCharRef(InvalidCodepoint(55296)))
#        -> Err(InvalidCharRef(InvalidNumber(Empty)))
#x       -> Err(InvalidCharRef(InvalidNumber(Empty)))
#-1      -> Err(InvalidCharRef(UnexpectedSign))   #+65 -> Err(InvalidCharRef(UnexpectedSign))
```

⇒ 修法：`Event::GeneralRef` 分支直接调 `quick_xml::escape::unescape`，`Ok` 用返回值，`Err` 转
`SvgGridError::InvalidInput`（rc=2）或 `Xml`（rc=1），文案点名实体与位置。**不要**自己写
`u32::from_str_radix`（会漏 `0`、代理区、`+`/`-`、空串四类）。

### 语料面（复核）

`tests/fixtures/**/*.svg`：0 个 `&#`、0 个命名实体。`research/**/*.svg`：命名实体只有
`&apos;` 133201、`&quot;` 5520、`&gt;` 264、`&lt;` 68、`&amp;` 45（全是五个预定义），
0 个 `&nbsp;`/`&mdash;`/数字实体。⇒ 修数字实体 + 未定义命名实体 fail-closed 对现有语料零回归。
