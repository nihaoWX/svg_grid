"""R46 — a definition is drawn in the user space of whoever references it.

Before this round the composer walked `<defs>` like any other subtree, so a
definition written outside the `data-scale` group inherited the root's `none`
while the shapes pointing at it scaled: the pattern tile kept `width="20"`, the
body became `M 0 49.600 …`, the body fell outside its own tile window and the
fill vanished (ink fell from 4608 to 0). Three neighbouring faces of the same
mistake are fixed here too:

  A  a definition follows the policy of its referrers (and is copied when two
     referrers disagree; the copy carries its own copies of what it references)
  B  a `<marker>` with a `viewBox` scales its viewport, not its body
  C  `gradientUnits`/`filterUnits` are resolved along `href`, and gradient
     coordinates are rewritten one attribute at a time
  D  `--normalize-input-max-side` leaves `objectBoundingBox` fractions alone
  E  the same checks against `SVG_GRID_OLD_BIN` must be red

Every check asserts rendered pixels (xor / maxdiff against a hand-written
oracle), not just markup: the same place has been "fixed" wrongly three times
before (round-data/R38, R39, R40).

Run:
  cargo build --offline
  python3 docs/PITFALLS/round-data/R46/test.py

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
TMP = pathlib.Path(tempfile.mkdtemp(prefix="svg-grid-r46-"))

PASSED: list[str] = []
FAILED: list[str] = []

CANVAS_W = 248.0
CANVAS_H = 198.4

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
    path = TMP / f"{_COUNTER[0]:03d}-{name}"
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
    result = run(RENDER, [svg, png, scale])
    if result.returncode != 0:
        raise RuntimeError(f"d1render failed on {svg}: {result.stderr.strip()}")
    return png


# --------------------------------------------------------------- svg scaffolding


def panel(inner: str) -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>\n'
        f"{inner}\n</svg>\n"
    )


def oracle(body: str) -> pathlib.Path:
    return write(
        "oracle.svg",
        f'<svg width="{CANVAS_W}" height="{CANVAS_H}" viewBox="0 0 {CANVAS_W} {CANVAS_H}" '
        'xmlns="http://www.w3.org/2000/svg">'
        f'<rect x="0" y="0" width="{CANVAS_W}" height="{CANVAS_H}" fill="#FFFFFF"/>'
        f"{body}</svg>",
    )


def compose(binary: pathlib.Path, source: pathlib.Path, width: float, tag: str, *extra) -> tuple:
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
            width,
            *extra,
        ],
    )
    return result, (out.read_text(encoding="utf-8") if out.exists() else "")


# ---------------------------------------------------------------- the families

PATTERN = (
    '<defs><pattern id="h" patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20">'
    "{body}</pattern></defs>"
)

FIXTURES = {
    # A: the pattern tile and body belong to the referrer's user space. `none`
    # (nothing scales) keeps every number; the round's defect made the tile stay
    # 20 while the body became 49.6 and fell outside it.
    "A1_none": {
        "src": panel(PATTERN.format(body='<path d="M 0,20 L 20,0" stroke="#000" stroke-width="2"/>')
                     + '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/>'),
        "marks": [
            'patternUnits="userSpaceOnUse" x="0" y="0" width="20" height="20"',
            'd="M 0,20 L 20,0" stroke="#000" stroke-width="2"',
        ],
        "body": PATTERN.format(body='<path d="M 0,20 L 20,0" stroke="#000" stroke-width="2"/>')
                + '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/>',
    },
    # A: under `x` only the horizontal numbers scale — the body is 14.880x6, not
    # 14.880x14.880 (the defect scaled both axes).
    "A2_x": {
        "src": panel(PATTERN.format(body='<rect x="0" y="0" width="6" height="6" fill="#000"/>')
                     + '<g data-scale="x"><rect x="10" y="10" width="80" height="60" fill="url(#h)"/></g>'),
        "marks": [
            'x="0" y="0" width="49.600" height="20"',
            '<rect x="0" y="0" width="14.880" height="6" fill="#000"/>',
        ],
        "body": PATTERN.format(body='<rect x="0" y="0" width="14.880" height="6" fill="#000"/>')
                .replace('width="20" height="20"', 'width="49.600" height="20"')
                + '<rect x="24.800" y="10" width="198.400" height="60" fill="url(#h)"/>',
    },
    # A: `position` never changes a size, even for a definition written inside it.
    "A3_position": {
        "src": panel(
            '<g data-scale="position">'
            + PATTERN.format(body='<rect x="0" y="0" width="6" height="6" fill="#000"/>')
            + '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/></g>'
        ),
        "marks": [
            'x="0" y="0" width="20" height="20"',
            '<rect x="0" y="0" width="6" height="6" fill="#000"/>',
            '<rect x="24.800" y="24.800" width="80" height="60"',
        ],
        "body": PATTERN.format(body='<rect x="0" y="0" width="6" height="6" fill="#000"/>')
                + '<rect x="24.800" y="24.800" width="80" height="60" fill="url(#h)"/>',
    },
    # A: two referrers disagree, so the definition is copied and each referrer
    # points at the copy for its own policy.
    "A4_copy": {
        "src": panel(
            PATTERN.format(body='<rect x="0" y="0" width="6" height="6" fill="#000"/>')
            + '<g data-scale="xy"><rect x="10" y="10" width="40" height="30" fill="url(#h)"/></g>'
            + '<g data-scale="x"><rect x="10" y="50" width="40" height="30" fill="url(#h)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-h" patternUnits="userSpaceOnUse" x="0" y="0" '
            'width="49.600" height="49.600">',
            '<pattern id="svg-grid-cell0-h--svg-grid-x" patternUnits="userSpaceOnUse" x="0" y="0" '
            'width="49.600" height="20">',
            'fill="url(#svg-grid-cell0-h)"/>',
            'fill="url(#svg-grid-cell0-h--svg-grid-x)"/>',
        ],
        "body": (
            '<defs><pattern id="a" patternUnits="userSpaceOnUse" x="0" y="0" width="49.600" height="49.600">'
            '<rect x="0" y="0" width="14.880" height="14.880" fill="#000"/></pattern>'
            '<pattern id="b" patternUnits="userSpaceOnUse" x="0" y="0" width="49.600" height="20">'
            '<rect x="0" y="0" width="14.880" height="6" fill="#000"/></pattern></defs>'
            '<rect x="24.800" y="24.800" width="99.200" height="74.400" fill="url(#a)"/>'
            '<rect x="24.800" y="50" width="99.200" height="30" fill="url(#b)"/>'
        ),
    },
    # B: with a viewBox the renderer already maps the body onto the viewport, so
    # only the box and the reference point grow. The defect scaled the body too.
    "B1_marker_viewbox": {
        "src": panel(
            '<g data-scale="xy"><defs><marker id="a" markerUnits="userSpaceOnUse" markerWidth="10" '
            'markerHeight="10" refX="5" refY="5" viewBox="0 0 10 10" orient="auto">'
            '<path d="M 0,0 L 10,5 L 0,10 Z" fill="#000"/></marker></defs>'
            '<polyline points="10,40 90,40" stroke="#000" stroke-width="2" marker-end="url(#a)"/></g>'
        ),
        "marks": [
            'markerWidth="24.800" markerHeight="24.800" refX="12.400" refY="12.400"',
            'viewBox="0 0 10 10"',
            'd="M 0,0 L 10,5 L 0,10 Z"',
        ],
        "body": (
            '<defs><marker id="a" markerUnits="userSpaceOnUse" markerWidth="24.800" markerHeight="24.800" '
            'refX="12.400" refY="12.400" viewBox="0 0 10 10" orient="auto">'
            '<path d="M 0,0 L 10,5 L 0,10 Z" fill="#000"/></marker></defs>'
            '<polyline points="24.800,99.200 223.200,99.200" stroke="#000" stroke-width="4.960" '
            'fill="none" marker-end="url(#a)"/>'
        ),
    },
    # C: `gradientUnits` lives on `base` and is inherited by `d`; `d`'s own lone
    # `x1` must still be rewritten (attribute by attribute, not as a pair).
    "C1_href_units": {
        "src": panel(
            '<defs>'
            '<linearGradient id="base" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="100" y2="0">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient>'
            '<linearGradient id="d" href="#base" x1="10">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient>'
            "</defs>"
            '<g data-scale="xy"><rect x="10" y="10" width="80" height="60" fill="url(#d)"/></g>'
        ),
        "marks": [
            'gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="248" y2="0"',
            'x1="24.800"',
            'fill="url(#svg-grid-cell0-d)"',
        ],
        "body": (
            '<defs><linearGradient id="base" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="248" y2="0">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient>'
            '<linearGradient id="d" href="#base" x1="24.800">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient></defs>'
            '<rect x="24.800" y="24.800" width="198.400" height="148.800" fill="url(#d)"/>'
        ),
    },
    # C: a gradient written at the root follows the referrer like a pattern does.
    "C2_gradient_root": {
        "src": panel(
            '<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="100" y2="0">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient></defs>'
            '<g data-scale="xy"><rect x="10" y="10" width="80" height="60" fill="url(#g)"/></g>'
        ),
        "marks": [
            'gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="248" y2="0"',
            'fill="url(#svg-grid-cell0-g)"',
        ],
        "body": (
            '<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="248" y2="0">'
            '<stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#ffffff"/></linearGradient></defs>'
            '<rect x="24.800" y="24.800" width="198.400" height="148.800" fill="url(#g)"/>'
        ),
    },
}


# --------------------------------------------------------- D: normalization


def normalize_case(binary: pathlib.Path, name: str, source: str, marks: list) -> tuple:
    """Rendering a normalized input must look exactly like the unnormalized one.

    The panel fills the canvas in both runs (`data-scale="xy"`), so normalization's
    shrink is compensated by the panel scaling and any difference left over is a
    number the normalizer should not have touched.
    """
    src = write(f"{name}.src.svg", source)
    plain, _ = compose(binary, src, 400.0, "plain")
    if plain.returncode != 0:
        return False, f"compose failed rc={plain.returncode}: {plain.stderr.strip()}"
    normalised, text = compose(binary, src, 400.0, "norm", "--normalize-input-max-side", "50")
    if normalised.returncode != 0:
        return False, f"normalized compose failed rc={normalised.returncode}: {normalised.stderr.strip()}"
    for mark in marks:
        if mark not in text:
            return False, f"missing {mark!r} after normalization: {text[-600:]}"
    plain_png = render(src.with_name(f"{src.stem}.plain.svg"), "plain")
    normalised_png = render(src.with_name(f"{src.stem}.norm.svg"), "norm")
    xor, worst = diff(plain_png, normalised_png)
    if (xor, worst) != (0, 0):
        return False, (
            f"normalizing changed the picture: xor={xor} maxdiff={worst} "
            f"ink {ink(plain_png)} -> {ink(normalised_png)}"
        )
    return True, f"xor=0 ink={ink(plain_png)}"


NORMALIZE_FIXTURES = {
    # `objectBoundingBox` fractions describe a box that already scales: the defect
    # halved them a second time (0.25 -> 0.125, body 0.1 -> 0.05).
    "D1_obb_pattern": {
        "src": panel(
            '<g data-scale="xy"><defs><pattern id="h" width="0.25" height="0.25" '
            'patternContentUnits="objectBoundingBox"><rect x="0" y="0" width="0.1" height="0.1" '
            'fill="#000"/></pattern></defs>'
            '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/></g>'
        ),
        "marks": ['width="0.25" height="0.25"', 'width="0.1" height="0.1"'],
    },
    # A marker box in `strokeWidth` units is relative to a stroke that already
    # shrank: the defect halved it too (10 -> 5, ink 4880 -> 2600).
    "D2_marker_stroke_units": {
        "src": panel(
            '<g data-scale="xy"><defs><marker id="a" markerWidth="10" markerHeight="10" '
            'refX="5" refY="5" orient="auto"><path d="M 0,0 L 10,5 L 0,10 Z" fill="#000"/></marker></defs>'
            '<polyline points="20,40 80,40" stroke="#000" stroke-width="2" marker-end="url(#a)"/></g>'
        ),
        "marks": ['markerWidth="10" markerHeight="10"', 'refX="5" refY="5"'],
    },
    # Control: a `userSpaceOnUse` marker box *is* user geometry and must scale.
    "D3_marker_user_space": {
        "src": panel(
            '<g data-scale="xy"><defs><marker id="a" markerUnits="userSpaceOnUse" markerWidth="10" '
            'markerHeight="10" refX="5" refY="5" orient="auto">'
            '<path d="M 0,0 L 10,5 L 0,10 Z" fill="#000"/></marker></defs>'
            '<polyline points="20,40 80,40" stroke="#000" stroke-width="2" marker-end="url(#a)"/></g>'
        ),
        "marks": ['markerUnits="userSpaceOnUse" markerWidth="40" markerHeight="40"'],
    },
}


def fixture_case(binary: pathlib.Path, label: str, fixture: dict) -> tuple:
    src = write(f"{label}.src.svg", fixture["src"])
    result, text = compose(binary, src, CANVAS_W, "got")
    if result.returncode != 0:
        return False, f"compose failed rc={result.returncode}: {result.stderr.strip()}"
    for mark in fixture["marks"]:
        if mark not in text:
            return False, f"missing {mark!r}: {text[-600:]}"
    reference = oracle(fixture["body"])
    xor, worst = diff(render(reference, "ref"), render(src.with_name(f"{src.stem}.got.svg"), "got"))
    if (xor, worst) != (0, 0):
        return False, f"render xor={xor} maxdiff={worst}"
    return True, f"xor=0 ink={ink(render(reference, 'ref'))}"


def main() -> int:
    if not RENDER.exists():
        print(f"FAIL  reference renderer missing: {RENDER}")
        return 1

    print("== A/B/C: a definition is drawn where it is referenced ==")
    for label, fixture in FIXTURES.items():
        ok, detail = fixture_case(BIN, label, fixture)
        check(ok, label, detail)

    print("== D: normalization keeps bbox fractions and stroke boxes ==")
    for label, fixture in NORMALIZE_FIXTURES.items():
        ok, detail = normalize_case(BIN, label, fixture["src"], fixture["marks"])
        check(ok, label, detail)

    print("== E: the base binary must be red on the same cases ==")
    if OLD_BIN is None:
        for label in FIXTURES:
            skip(f"E/{label}", "SVG_GRID_OLD_BIN is not set")
        for label in NORMALIZE_FIXTURES:
            skip(f"E/{label}", "SVG_GRID_OLD_BIN is not set")
    else:
        # Only D3 is a control: a `userSpaceOnUse` marker box already scaled
        # correctly at the base commit, so it must stay green there.
        red_expected = [l for l in FIXTURES if l != "D3_marker_user_space"]
        red_expected += ["D1_obb_pattern", "D2_marker_stroke_units"]
        for label in red_expected:
            if label in FIXTURES:
                ok, detail = fixture_case(OLD_BIN, f"old-{label}", FIXTURES[label])
            else:
                fixture = NORMALIZE_FIXTURES[label]
                ok, detail = normalize_case(OLD_BIN, f"old-{label}", fixture["src"], fixture["marks"])
            check(not ok, f"E/{label} is red on the base binary", f"still green: {detail}")
        for label in ["D3_marker_user_space"]:
            if label in FIXTURES:
                ok, detail = fixture_case(OLD_BIN, f"old-{label}", FIXTURES[label])
            else:
                fixture = NORMALIZE_FIXTURES[label]
                ok, detail = normalize_case(OLD_BIN, f"old-{label}", fixture["src"], fixture["marks"])
            check(ok, f"E/{label} is a control and stays green", detail)

    print()
    if FAILED:
        print(f"== 失败 {len(FAILED)} 条（成立 {len(PASSED)} 条）==")
        return 1
    print(f"== 全部成立（{len(PASSED)} 条）==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
