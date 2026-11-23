"""R47 — a character reference in text is a character, not a literal.

`quick_xml` hands a text-node reference over as `Event::GeneralRef` without its
`&`/`;`. Both DOMs resolved only the five predefined names and echoed everything
else verbatim, so `&#176;` became the literal `&#176;`; the serializer then
escaped the `&` again ⇒ `&amp;#176;`, which the renderer draws as visible text
(ink 1701 instead of 502 on the `37°C ×2 ±1` panel).

The fix routes both branches through `quick_xml::escape::unescape`, so the text
path now has exactly the semantics the attribute path always had:

  A  numeric references decode (decimal, hex, leading zeros, astral, whitespace)
  B  an undefined name or a malformed number fails closed on both paths
  C  CDATA stays character data — the literal is not a reference and must not move
  D  the attribute path is a control: it already behaved this way and must not move
  E  the same checks against `SVG_GRID_OLD_BIN` must be red (A) / green (C, D)

Every text assertion is character-for-character, and A1 is additionally compared
pixel by pixel against a hand-written oracle: this repository has been wrong
about entity/unit semantics three times already (round-data/R38, R39, R40).

Run:
  cargo build --offline
  python3 docs/PITFALLS/round-data/R47/test.py

Env:
  SVG_GRID_BIN        binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN    base binary; supplies family E (skipped when unset)
  SVG_GRID_RENDER_BIN reference renderer (default: the D1 resvg build)
"""

import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
FONT_DIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="svg-grid-r47-"))

PASSED: list[str] = []
FAILED: list[str] = []

CANVAS_W = 400.0
CANVAS_H = 80.0

_COUNTER = [0]


def check(ok: bool, label: str, detail: str = "") -> bool:
    (PASSED if ok else FAILED).append(label)
    print(f"{'PASS  ' if ok else 'FAIL  '}{label}")
    if not ok and detail:
        print(f"      {detail}")
    return ok


def skip(label: str, why: str) -> None:
    print(f"SKIP  {label}  （{why}）")


def write(name: str, text: str) -> pathlib.Path:
    _COUNTER[0] += 1
    path = TMP / f"{_COUNTER[0]:03d}-{name.replace('/', '_')}"
    path.write_text(text, encoding="utf-8")
    return path


def run(binary: pathlib.Path, args: list) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(binary), *[str(a) for a in args]], capture_output=True, text=True, timeout=300
    )


# ---------------------------------------------------------------- pixel helpers


def read_png(path):
    data = pathlib.Path(path).read_bytes()
    position = 8
    width = height = channels = None
    rows = []
    while position + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[position : position + 8])
        payload = data[position + 8 : position + 8 + length]
        position += 12 + length
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", payload[:10])
            if depth != 8:
                raise ValueError("only 8-bit PNGs are read here")
            channels = {0: 1, 2: 3, 4: 2, 6: 4}[colour]
        elif kind == b"IDAT":
            rows.append(payload)
        elif kind == b"IEND":
            break
    raw = zlib.decompress(b"".join(rows))
    stride = width * channels
    out = []
    previous = bytearray(stride)
    cursor = 0
    for _ in range(height):
        filter_kind = raw[cursor]
        cursor += 1
        line = bytearray(raw[cursor : cursor + stride])
        cursor += stride
        for index in range(stride):
            left = line[index - channels] if index >= channels else 0
            up = previous[index]
            corner = previous[index - channels] if index >= channels else 0
            if filter_kind == 0:
                value = line[index]
            elif filter_kind == 1:
                value = line[index] + left
            elif filter_kind == 2:
                value = line[index] + up
            elif filter_kind == 3:
                value = line[index] + (left + up) // 2
            elif filter_kind == 4:
                estimate = left + up - corner
                pa, pb, pc = (
                    abs(estimate - left),
                    abs(estimate - up),
                    abs(estimate - corner),
                )
                predictor = left if pa <= pb and pa <= pc else (up if pb <= pc else corner)
                value = line[index] + predictor
            else:
                raise ValueError(f"unknown PNG filter {filter_kind}")
            line[index] = value & 0xFF
        out.append(bytes(line))
        previous = line
    return width, height, channels, out


def ink(path) -> int:
    width, height, channels, rows = read_png(path)
    count = 0
    for row in rows:
        for x in range(width):
            if row[x * channels : x * channels + 3] != b"\xff\xff\xff":
                count += 1
    return count


def diff(left, right):
    lw, lh, lc, lrows = read_png(left)
    rw, rh, rc, rrows = read_png(right)
    if (lw, lh) != (rw, rh):
        return 10**9, 255
    xor = 0
    worst = 0
    for y in range(lh):
        lrow, rrow = lrows[y], rrows[y]
        for x in range(lw):
            lp = lrow[x * lc : x * lc + 3]
            rp = rrow[x * rc : x * rc + 3]
            if lp != rp:
                xor += 1
                worst = max(worst, max(abs(a - b) for a, b in zip(lp, rp)))
    return xor, worst


def render(svg: pathlib.Path, tag: str, scale: float = 1.0) -> pathlib.Path:
    png = svg.with_name(f"{svg.stem}.{tag}.png")
    environment = dict(os.environ)
    if FONT_DIR.is_dir():
        environment["D1_FONT_DIR"] = str(FONT_DIR)
    result = subprocess.run(
        [str(RENDER), str(svg), str(png), str(scale)],
        capture_output=True,
        text=True,
        timeout=300,
        env=environment,
    )
    if result.returncode != 0:
        raise RuntimeError(f"d1render failed on {svg}: {result.stderr.strip()}")
    return png


# --------------------------------------------------------------- svg scaffolding


def panel(inner: str) -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">\n'
        '<rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>\n'
        f"{inner}\n</svg>\n"
    )


def plain(inner: str) -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">\n'
        f"{inner}\n</svg>\n"
    )


def text(body: str) -> str:
    return f'<text x="10" y="50" font-size="20" font-family="Arial">{body}</text>'


def oracle(body: str) -> pathlib.Path:
    return write(
        "oracle.svg",
        f'<svg width="{CANVAS_W}" height="{CANVAS_H}" viewBox="0 0 {CANVAS_W} {CANVAS_H}" '
        'xmlns="http://www.w3.org/2000/svg">'
        f'<rect x="0" y="0" width="{CANVAS_W}" height="{CANVAS_H}" fill="#FFFFFF" stroke="none"/>'
        f"{body}</svg>",
    )


def read(path: pathlib.Path) -> str:
    """Read without newline translation: `&#13;` must stay a carriage return."""
    return path.read_bytes().decode("utf-8")


def compose(binary: pathlib.Path, source: pathlib.Path, tag: str) -> tuple:
    out = source.with_name(f"{source.stem}.{tag}.svg")
    if out.exists():
        out.unlink()
    result = run(
        binary,
        [
            "plot-grid",
            "--input",
            source,
            "--output",
            out,
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--width",
            CANVAS_W,
        ],
    )
    return result, (read(out) if out.exists() else "")


def convert(binary: pathlib.Path, source: pathlib.Path, tag: str) -> tuple:
    out = source.with_name(f"{source.stem}.{tag}.svg")
    if out.exists():
        out.unlink()
    result = run(binary, ["convert", "--input", source, "--output", out])
    return result, (read(out) if out.exists() else "")


def text_nodes(svg: str) -> list:
    """The character data of every `<text>` element, in document order.

    The serializer escapes `& < > " '` on the way out, so the five predefined
    references are turned back into the characters they stand for here: what is
    under test is the DOM text, not the escaping the writer has always done.
    """
    nodes = []
    for chunk in svg.split("<text")[1:]:
        start = chunk.index(">") + 1
        end = chunk.index("</text>", start)
        node = chunk[start:end]
        for reference, character in (
            ("&amp;", "&"),
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&quot;", '"'),
            ("&apos;", "'"),
        ):
            node = node.replace(reference, character)
        nodes.append(node)
    return nodes


# ---------------------------------------------------------------- the families

# A: what a reference means. `&#065;` and `&#x0041;` are the same character as
# `&#65;`; `&#x1F600;` is outside the BMP; the whitespace references are legal
# and must survive as the characters they name.
DECODE = {
    "A1_decimal": ("37&#176;C &#215;2 &#177;1", "37°C ×2 ±1"),
    "A2_hex": ("&#x42;&#x1F600;", "B😀"),
    "A3_leading_zeros": ("&#065;&#x0041;", "AA"),
    # A control: the five predefined names already resolved before the fix.
    "A4_predefined": ("&amp;&lt;&gt;&quot;&apos;", "&<>\"'"),
    "A5_whitespace": ("a&#9;b&#10;c&#13;d&#32;e", "a\tb\nc\rd e"),
}

# B: fail-closed. The message is quick-xml's own text, verbatim.
REJECT = {
    "B1_undefined_name": ("a&nbsp;b", "unrecognized entity"),
    "B2_zero": ("a&#0;b", "invalid character reference"),
    "B3_astral_overflow": ("a&#x110000;b", "invalid character reference"),
    "B4_surrogate": ("a&#55296;b", "invalid character reference"),
    "B5_empty": ("a&#;b", "invalid character reference"),
    "B6_sign": ("a&#-1;b", "invalid character reference"),
    "B7_upper_x": ("a&#X41;b", "invalid character reference"),
}


def decode_case(binary: pathlib.Path, label: str, body: str, expected: str, path: str) -> tuple:
    source = write(f"{label}.src.svg", (panel if path == "plot" else plain)(text(body)))
    result, out = (compose if path == "plot" else convert)(binary, source, "got")
    if result.returncode != 0:
        return False, f"rc={result.returncode}: {result.stderr.strip()}"
    nodes = text_nodes(out)
    if nodes != [expected]:
        return False, f"text nodes {nodes!r} != {[expected]!r}"
    return True, f"text={expected!r}"


def reject_case(binary: pathlib.Path, label: str, body: str, phrase: str, path: str) -> tuple:
    source = write(f"{label}.src.svg", (panel if path == "plot" else plain)(text(body)))
    result, out = (compose if path == "plot" else convert)(binary, source, "got")
    if result.returncode != 1:
        return False, f"rc={result.returncode} (want 1): {result.stderr.strip()}"
    if phrase not in result.stderr:
        return False, f"stderr lacks {phrase!r}: {result.stderr.strip()}"
    if out:
        return False, "an output file was written on failure"
    return True, result.stderr.strip().splitlines()[-1]


def render_case(binary: pathlib.Path, label: str, body: str, expected: str) -> tuple:
    """The decoded panel must be pixel-identical to a hand-written oracle."""
    source = write(f"{label}.src.svg", panel(text(body)))
    result, out = compose(binary, source, "got")
    if result.returncode != 0:
        return False, f"rc={result.returncode}: {result.stderr.strip()}"
    got = source.with_name(f"{source.stem}.got.svg")
    reference = oracle(text(expected))
    xor, worst = diff(render(reference, "ref"), render(got, "got"))
    if (xor, worst) != (0, 0):
        return False, f"render xor={xor} maxdiff={worst} ink={ink(render(got, 'got'))}"
    return True, f"xor=0 ink={ink(render(reference, 'ref'))}"


def main() -> int:
    if not RENDER.exists():
        print(f"FAIL  reference renderer missing: {RENDER}")
        return 1

    print("== A: a character reference in text decodes to its character ==")
    for label, (body, expected) in DECODE.items():
        for path in ("plot", "convert"):
            ok, detail = decode_case(BIN, f"{label}/{path}", body, expected, path)
            check(ok, f"{label}/{path}", detail)

    print("== A': the decoded panel is pixel-identical to the oracle ==")
    ok, detail = render_case(BIN, "A1_render", *DECODE["A1_decimal"])
    check(ok, "A1_render/plot", detail)

    print("== B: an undefined name or a malformed number fails closed ==")
    for label, (body, phrase) in REJECT.items():
        for path in ("plot", "convert"):
            ok, detail = reject_case(BIN, f"{label}/{path}", body, phrase, path)
            check(ok, f"{label}/{path}", detail)

    print("== C: CDATA is character data, not a reference ==")
    cdata_body = "<![CDATA[37&#176;C]]>"
    source = write("C1.src.svg", panel(text(cdata_body)))
    result, out = compose(BIN, source, "got")
    if result.returncode != 0:
        check(False, "C1_cdata/plot", f"rc={result.returncode}: {result.stderr.strip()}")
    else:
        # The literal survives: the writer escapes its `&` on the way out, and
        # `text_nodes` turns that escape back into the character it stands for.
        nodes = text_nodes(out)
        check(nodes == ["37&#176;C"], "C1_cdata/plot", f"text nodes {nodes!r}")
        if OLD_BIN is not None:
            old_result, old_out = compose(OLD_BIN, source, "old")
            check(
                old_result.returncode == 0 and old_out == out,
                "C1_cdata/plot is byte-identical to the base binary",
                f"rc={old_result.returncode}",
            )
        else:
            skip("C1_cdata/plot is byte-identical to the base binary", "SVG_GRID_OLD_BIN is not set")

    print("== D: the attribute path is a control and must not move ==")
    attribute = write(
        "D1.src.svg",
        panel(
            '<rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none" '
            'data-x="a&#176;b"/>'
        ),
    )
    result, out = compose(BIN, attribute, "got")
    check(
        result.returncode == 0 and 'data-x="a°b"' in out,
        "D1_attribute_numeric_still_decodes",
        f"rc={result.returncode}: {result.stderr.strip()}",
    )
    attribute = write(
        "D2.src.svg",
        panel(
            '<rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none" '
            'data-x="a&nbsp;b"/>'
        ),
    )
    result, _ = compose(BIN, attribute, "got")
    check(
        result.returncode == 1 and "unrecognized entity" in result.stderr,
        "D2_attribute_undefined_name_still_fails",
        f"rc={result.returncode}: {result.stderr.strip()}",
    )

    print("== E: the base binary must be red on A and green on C/D ==")
    if OLD_BIN is None:
        for label in DECODE:
            skip(f"E/{label}", "SVG_GRID_OLD_BIN is not set")
        skip("E/A1_render", "SVG_GRID_OLD_BIN is not set")
    else:
        for label, (body, expected) in DECODE.items():
            for path in ("plot", "convert"):
                ok, detail = decode_case(OLD_BIN, f"old-{label}/{path}", body, expected, path)
                if label == "A4_predefined":
                    check(ok, f"E/{label}/{path} is a control and stays green", detail)
                else:
                    check(
                        not ok,
                        f"E/{label}/{path} is red on the base binary",
                        f"still green: {detail}",
                    )
        ok, detail = render_case(OLD_BIN, "old-A1_render", *DECODE["A1_decimal"])
        check(not ok, "E/A1_render is red on the base binary", f"still green: {detail}")

    print()
    if FAILED:
        print(f"== 失败 {len(FAILED)} 条（成立 {len(PASSED)} 条）==")
        return 1
    print(f"== 全部成立（{len(PASSED)} 条）==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
