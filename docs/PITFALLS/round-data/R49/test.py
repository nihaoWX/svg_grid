"""R49 — nested definitions are copied once per policy, and a definition's own
presentation attributes follow its **content** domain.

B1  `<pattern id="P">` contains `<defs><linearGradient id="G">`; P is referenced
    under two policies. `split_children` recursed into every copy of P and split
    G by its full referrer set, so each copy of P got the full set of G copies:
    the product carried duplicate ids (G twice, G--y three times). The copies
    happened to be byte-identical, but resvg/cairosvg resolve duplicate ids
    last-wins and browsers first-wins, so a variant whose copies differ (probe11)
    renders differently across renderers. The fix narrows the inner wanted set to
    the copy's own policy and hoists a nested definition that is also referenced
    from outside its enclosing definition.

P1  `--normalize-input-max-side` scaled a definition's own `stroke-width` /
    `font-size` before looking at its content units, so an `objectBoundingBox`
    pattern's `stroke-width="0.1"` became `0.050` and the stroke doubled. The gate
    now follows the content domain: pattern `patternContentUnits` (and no
    viewBox), marker `markerUnits`, mask `maskContentUnits`, filter
    `primitiveUnits`, clipPath `clipPathUnits`; a gradient has no content domain
    and keeps the old behaviour.

Families:
  A  B1: nested definitions — unique ids, resolvable refs, render == oracle
  B  P1: content-domain gate, plus a `userSpaceOnUse` control that must scale
  C  the base binary must be red on the same cases

Run:
  cargo build --offline
  python3 docs/PITFALLS/round-data/R49/test.py

Env:
  SVG_GRID_BIN        binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN    base binary; supplies family C (skipped when unset)
  SVG_GRID_RENDER_BIN reference renderer (default: the D1 resvg build)
"""

import collections
import os
import pathlib
import re
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
TMP = pathlib.Path(tempfile.mkdtemp(prefix="svg-grid-r49-"))

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


# ---------------------------------------------------------------- B1: nesting

PATTERN = (
    '<defs><pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
    'x="0" y="0" width="20" height="20">{body}</pattern></defs>'
)

GRADIENT = (
    '<defs><linearGradient id="G" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="20" y2="20">'
    '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs>'
)

NESTED = {
    # A1: P referenced by {xy, y}; G referenced by P's body. Before the fix the
    # product carried G twice and G--y three times.
    "A1_two_policies": {
        "src": panel(
            PATTERN.format(body=GRADIENT + '<rect x="0" y="0" width="20" height="20" fill="url(#G)"/>')
            + '<g data-scale="xy"><rect x="10" y="10" width="60" height="40" fill="url(#P)"/></g>'
            + '<g data-scale="y"><rect x="10" y="60" width="60" height="20" fill="url(#P)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-P"',
            '<pattern id="svg-grid-cell0-P--svg-grid-y"',
            '<linearGradient id="svg-grid-cell0-G"',
            '<linearGradient id="svg-grid-cell0-G--svg-grid-y"',
        ],
        "body": (
            '<defs>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="49.600">'
            '<linearGradient id="G" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="49.600" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="49.600" height="49.600" fill="url(#G)"/></pattern>'
            '<pattern id="P--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="49.600">'
            '<linearGradient id="G--y" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="20" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="20" height="49.600" fill="url(#G--y)"/></pattern>'
            '</defs>'
            '<rect x="24.800" y="24.800" width="148.800" height="99.200" fill="url(#P)"/>'
            '<rect x="10" y="148.800" width="60" height="49.600" fill="url(#P--y)"/>'
        ),
    },
    # A2: three referrers {xy, x, y} — three copies of P and of G, all unique.
    "A2_three_policies": {
        "src": panel(
            PATTERN.format(body=GRADIENT + '<rect x="0" y="0" width="20" height="20" fill="url(#G)"/>')
            + '<g data-scale="xy"><rect x="10" y="10" width="60" height="30" fill="url(#P)"/></g>'
            + '<g data-scale="x"><rect x="10" y="45" width="60" height="15" fill="url(#P)"/></g>'
            + '<g data-scale="y"><rect x="10" y="65" width="60" height="10" fill="url(#P)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-P"',
            '<pattern id="svg-grid-cell0-P--svg-grid-x"',
            '<pattern id="svg-grid-cell0-P--svg-grid-y"',
            '<linearGradient id="svg-grid-cell0-G"',
            '<linearGradient id="svg-grid-cell0-G--svg-grid-x"',
            '<linearGradient id="svg-grid-cell0-G--svg-grid-y"',
        ],
        "body": (
            '<defs>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="49.600">'
            '<linearGradient id="G" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="49.600" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="49.600" height="49.600" fill="url(#G)"/></pattern>'
            '<pattern id="P--x" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="20">'
            '<linearGradient id="G--x" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="49.600" y2="20">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="49.600" height="20" fill="url(#G--x)"/></pattern>'
            '<pattern id="P--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="49.600">'
            '<linearGradient id="G--y" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="20" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="20" height="49.600" fill="url(#G--y)"/></pattern>'
            '</defs>'
            '<rect x="24.800" y="24.800" width="148.800" height="74.400" fill="url(#P)"/>'
            '<rect x="24.800" y="45" width="148.800" height="15" fill="url(#P--x)"/>'
            '<rect x="10" y="161.200" width="60" height="24.800" fill="url(#P--y)"/>'
        ),
    },
    # A3: two nesting levels — P contains G contains H. Each level is copied once
    # per policy, and each copy points at its own child copy.
    "A3_two_levels": {
        "src": panel(
            '<defs><pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="20">'
            '<pattern id="G" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="10" height="10">'
            '<linearGradient id="H" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="10">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="10" height="10" fill="url(#H)"/></pattern>'
            '<rect x="0" y="0" width="20" height="20" fill="url(#G)"/></pattern></defs>'
            '<g data-scale="xy"><rect x="10" y="10" width="60" height="40" fill="url(#P)"/></g>'
            '<g data-scale="y"><rect x="10" y="60" width="60" height="20" fill="url(#P)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-P"',
            '<pattern id="svg-grid-cell0-P--svg-grid-y"',
            '<pattern id="svg-grid-cell0-G"',
            '<pattern id="svg-grid-cell0-G--svg-grid-y"',
            '<linearGradient id="svg-grid-cell0-H"',
            '<linearGradient id="svg-grid-cell0-H--svg-grid-y"',
        ],
        "body": (
            '<defs>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="49.600">'
            '<pattern id="G" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="24.800" height="24.800">'
            '<linearGradient id="H" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="24.800" y2="24.800">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="24.800" height="24.800" fill="url(#H)"/></pattern>'
            '<rect x="0" y="0" width="49.600" height="49.600" fill="url(#G)"/></pattern>'
            '<pattern id="P--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="49.600">'
            '<pattern id="G--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="10" height="24.800">'
            '<linearGradient id="H--y" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="10" y2="24.800">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<rect x="0" y="0" width="10" height="24.800" fill="url(#H--y)"/></pattern>'
            '<rect x="0" y="0" width="20" height="49.600" fill="url(#G--y)"/></pattern>'
            '</defs>'
            '<rect x="24.800" y="24.800" width="148.800" height="99.200" fill="url(#P)"/>'
            '<rect x="10" y="148.800" width="60" height="49.600" fill="url(#P--y)"/>'
        ),
    },
    # A4: a clone references another clone — P is copied for {xy, y}, its body
    # references Q, and Q is also referenced from outside under xy. Q is copied
    # too, and each P copy points at the Q copy for its own policy.
    "A4_clone_references_clone": {
        "src": panel(
            '<defs>'
            '<pattern id="Q" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="10" height="10">'
            '<rect x="0" y="0" width="10" height="10" fill="#000"/></pattern>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="20">'
            '<rect x="0" y="0" width="20" height="20" fill="url(#Q)"/></pattern>'
            '</defs>'
            '<g data-scale="xy"><rect x="10" y="10" width="60" height="40" fill="url(#P)"/></g>'
            '<g data-scale="y"><rect x="10" y="60" width="60" height="20" fill="url(#P)"/></g>'
            '<g data-scale="xy"><rect x="80" y="10" width="10" height="10" fill="url(#Q)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-P"',
            '<pattern id="svg-grid-cell0-P--svg-grid-y"',
            '<pattern id="svg-grid-cell0-Q"',
            '<pattern id="svg-grid-cell0-Q--svg-grid-y"',
            'fill="url(#svg-grid-cell0-Q)"',
            'fill="url(#svg-grid-cell0-Q--svg-grid-y)"',
        ],
        "body": (
            '<defs>'
            '<pattern id="Q" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="24.800" height="24.800">'
            '<rect x="0" y="0" width="24.800" height="24.800" fill="#000"/></pattern>'
            '<pattern id="Q--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="10" height="24.800">'
            '<rect x="0" y="0" width="10" height="24.800" fill="#000"/></pattern>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="49.600">'
            '<rect x="0" y="0" width="49.600" height="49.600" fill="url(#Q)"/></pattern>'
            '<pattern id="P--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="49.600">'
            '<rect x="0" y="0" width="20" height="49.600" fill="url(#Q--y)"/></pattern>'
            '</defs>'
            '<rect x="24.800" y="24.800" width="148.800" height="99.200" fill="url(#P)"/>'
            '<rect x="10" y="148.800" width="60" height="49.600" fill="url(#P--y)"/>'
            '<rect x="198.400" y="24.800" width="24.800" height="24.800" fill="url(#Q)"/>'
        ),
    },
    # A5: a nested definition referenced from outside its enclosing definition
    # under a policy the enclosing one does not have is hoisted to the top level
    # and split once there; the outer copies keep references to their own copy.
    "A5_hoist_outside": {
        "src": panel(
            '<defs><pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="20">'
            '<defs><linearGradient id="G" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="20" y2="20">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs>'
            '<rect x="0" y="0" width="20" height="20" fill="url(#G)"/></pattern></defs>'
            '<g data-scale="xy"><rect x="10" y="10" width="60" height="30" fill="url(#P)"/></g>'
            '<g data-scale="y"><rect x="10" y="45" width="60" height="15" fill="url(#P)"/></g>'
            '<g data-scale="x"><rect x="10" y="65" width="60" height="10" fill="url(#G)"/></g>'
        ),
        "marks": [
            '<pattern id="svg-grid-cell0-P"',
            '<pattern id="svg-grid-cell0-P--svg-grid-y"',
            '<linearGradient id="svg-grid-cell0-G"',
            '<linearGradient id="svg-grid-cell0-G--svg-grid-x"',
            '<linearGradient id="svg-grid-cell0-G--svg-grid-y"',
        ],
        "body": (
            '<defs>'
            '<pattern id="P" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="49.600" height="49.600">'
            '<rect x="0" y="0" width="49.600" height="49.600" fill="url(#G)"/></pattern>'
            '<pattern id="P--y" patternUnits="userSpaceOnUse" patternContentUnits="userSpaceOnUse" '
            'x="0" y="0" width="20" height="49.600">'
            '<rect x="0" y="0" width="20" height="49.600" fill="url(#G--y)"/></pattern>'
            '<linearGradient id="G" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="49.600" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<linearGradient id="G--x" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="49.600" y2="20">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '<linearGradient id="G--y" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="20" y2="49.600">'
            '<stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>'
            '</defs>'
            '<rect x="24.800" y="24.800" width="148.800" height="74.400" fill="url(#P)"/>'
            '<rect x="10" y="111.600" width="60" height="37.200" fill="url(#P--y)"/>'
            '<rect x="24.800" y="65" width="148.800" height="10" fill="url(#G--x)"/>'
        ),
    },
}


def nested_case(binary: pathlib.Path, label: str, fixture: dict) -> tuple:
    src = write(f"{label}.src.svg", fixture["src"])
    result, text = compose(binary, src, CANVAS_W, "got")
    if result.returncode != 0:
        return False, f"compose failed rc={result.returncode}: {result.stderr.strip()}"
    ids = re.findall(r'\bid="([^"]+)"', text)
    counts = collections.Counter(ids)
    duplicates = {name: count for name, count in counts.items() if count > 1}
    if duplicates:
        return False, f"duplicate ids {duplicates}"
    refs = re.findall(r"url\(#([^)]+)\)", text) + re.findall(r'href="#([^"]+)"', text)
    missing = sorted({ref for ref in refs if ref not in counts})
    if missing:
        return False, f"unresolvable refs {missing}"
    for mark in fixture["marks"]:
        if mark not in text:
            return False, f"missing {mark!r}: {text[-600:]}"
    reference = oracle(fixture["body"])
    xor, worst = diff(render(reference, "ref"), render(src.with_name(f"{src.stem}.got.svg"), "got"))
    if (xor, worst) != (0, 0):
        return False, f"render xor={xor} maxdiff={worst}"
    return True, f"ids={len(ids)} unique, xor=0 ink={ink(render(reference, 'ref'))}"


# --------------------------------------------------------- P1: content domain

P1_FIXTURES = {
    # An `objectBoundingBox` pattern's own `stroke-width` is a fraction of the
    # box, so normalization must leave it at 0.1 (the defect wrote 0.050).
    "B1_obb_pattern_stroke": {
        "src": panel(
            '<g data-scale="xy"><defs><pattern id="h" width="0.25" height="0.25" '
            'patternContentUnits="objectBoundingBox" stroke-width="0.1">'
            '<rect x="0" y="0" width="0.1" height="0.1" fill="none" stroke="#000"/></pattern></defs>'
            '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/></g>'
        ),
        "marks": ['stroke-width="0.1"'],
        "forbidden": ['stroke-width="0.050"'],
    },
    # Control: a `userSpaceOnUse` pattern's `stroke-width` is user geometry and
    # must scale (2 -> 1 at the 0.5 normalization factor). Without this the gate
    # could be "never scale" and still pass B1.
    "B2_userspace_pattern_stroke": {
        "src": panel(
            '<g data-scale="xy"><defs><pattern id="h" patternUnits="userSpaceOnUse" '
            'patternContentUnits="userSpaceOnUse" x="0" y="0" width="20" height="20" '
            'stroke-width="2">'
            '<rect x="0" y="0" width="6" height="6" fill="none" stroke="#000"/></pattern></defs>'
            '<rect x="10" y="10" width="80" height="60" fill="url(#h)"/></g>'
        ),
        "marks": ['stroke-width="1"'],
        "forbidden": ['stroke-width="2"'],
    },
    # Control: a `userSpaceOnUse` marker's own `font-size` is user geometry too.
    "B3_userspace_marker_font": {
        "src": panel(
            '<g data-scale="xy"><defs><marker id="m" markerUnits="userSpaceOnUse" markerWidth="10" '
            'markerHeight="10" refX="5" refY="5" orient="auto" font-size="8">'
            '<path d="M 0,0 L 10,5 L 0,10 Z" fill="#000"/></marker></defs>'
            '<polyline points="20,40 80,40" stroke="#000" stroke-width="2" marker-end="url(#m)"/></g>'
        ),
        "marks": ['font-size="4"'],
        "forbidden": ['font-size="8"'],
    },
}


def normalize_case(binary: pathlib.Path, label: str, fixture: dict) -> tuple:
    """Compose plain and normalized at the normalized width (scale 1), so the
    product's numbers are the normalized numbers, and require the same pixels."""
    src = write(f"{label}.src.svg", fixture["src"])
    plain, _ = compose(binary, src, 50.0, "plain")
    if plain.returncode != 0:
        return False, f"compose failed rc={plain.returncode}: {plain.stderr.strip()}"
    normalised, text = compose(binary, src, 50.0, "norm", "--normalize-input-max-side", "50")
    if normalised.returncode != 0:
        return False, f"normalized compose failed rc={normalised.returncode}: {normalised.stderr.strip()}"
    for mark in fixture["marks"]:
        if mark not in text:
            return False, f"missing {mark!r} after normalization: {text[-600:]}"
    for mark in fixture["forbidden"]:
        if mark in text:
            return False, f"unexpected {mark!r} after normalization: {text[-600:]}"
    plain_png = render(src.with_name(f"{src.stem}.plain.svg"), "plain")
    normalised_png = render(src.with_name(f"{src.stem}.norm.svg"), "norm")
    xor, worst = diff(plain_png, normalised_png)
    if (xor, worst) != (0, 0):
        return False, (
            f"normalizing changed the picture: xor={xor} maxdiff={worst} "
            f"ink {ink(plain_png)} -> {ink(normalised_png)}"
        )
    return True, f"xor=0 ink={ink(plain_png)}"


def main() -> int:
    if not RENDER.exists():
        print(f"FAIL  reference renderer missing: {RENDER}")
        return 1

    print("== A: nested definitions are copied once per policy ==")
    for label, fixture in NESTED.items():
        ok, detail = nested_case(BIN, label, fixture)
        check(ok, label, detail)

    print("== B: a definition's own lengths follow its content domain ==")
    for label, fixture in P1_FIXTURES.items():
        ok, detail = normalize_case(BIN, label, fixture)
        check(ok, label, detail)

    print("== C: the base binary must be red on the same cases ==")
    if OLD_BIN is None:
        for label in NESTED:
            skip(f"C/{label}", "SVG_GRID_OLD_BIN is not set")
        for label in P1_FIXTURES:
            skip(f"C/{label}", "SVG_GRID_OLD_BIN is not set")
    else:
        for label, fixture in NESTED.items():
            ok, detail = nested_case(OLD_BIN, f"old-{label}", fixture)
            check(not ok, f"C/{label} is red on the base binary", f"still green: {detail}")
        # B1 is the defect; B2/B3 are controls that already scaled correctly at
        # the base commit, so they must stay green there.
        for label in ["B1_obb_pattern_stroke"]:
            ok, detail = normalize_case(OLD_BIN, f"old-{label}", P1_FIXTURES[label])
            check(not ok, f"C/{label} is red on the base binary", f"still green: {detail}")
        for label in ["B2_userspace_pattern_stroke", "B3_userspace_marker_font"]:
            ok, detail = normalize_case(OLD_BIN, f"old-{label}", P1_FIXTURES[label])
            check(ok, f"C/{label} is a control and stays green", detail)

    print()
    if FAILED:
        print(f"== 失败 {len(FAILED)} 条（成立 {len(PASSED)} 条）==")
        return 1
    print(f"== 全部成立（{len(PASSED)} 条）==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
