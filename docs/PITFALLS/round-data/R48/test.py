"""R48 — composite-figure panel letters: catch the split ones, ignore the invisible ones.

The duplicate check used to read only the *direct* text children of a
`data-svg-grid-label` marker, compare the raw string, and walk into subtrees the
renderer never draws.  Three silent duplicates (rc=0) and five false positives
(rc=2 with an unchanged figure) followed.

  A  a letter split into a `<tspan>`, wrapped in a `<g>`, or padded with
     whitespace is the same letter (rc=2; the base binary says rc=0)
  B  normalization is trim + collapse of internal whitespace, on both the
     inherited and the explicit `--labels` path; an empty marker is not a letter
  C  `defs` / `symbol` / `display:none` / `visibility:hidden` / `opacity:0`
     subtrees are not rendered and do not collide (rc=0; the base says rc=2);
     `visibility` can be restored by a descendant, `display:none` cannot
  D  the message keeps every site (or an explicit `+N more`), reports the real
     cell, and tells the user something they can actually do
  E  a spacer draws no letter of its own, but an explicit `--labels` entry on
     that cell *does* render (d1render: 44 ink pixels), so it still collides
  F  40000 distinct letters stay linear (release, target <= 0.2 s)

Every A/B/C/D/F case is also run against `SVG_GRID_OLD_BIN`; the old binary must
show the opposite result (the "teeth").  B5 and C8 compare rendered PNGs, not
SVG bytes: the marker is copied into the output, so the SVG text differs even
when the figure is identical.

Run:
  cargo build --offline
  python3 docs/PITFALLS/round-data/R48/test.py

Env:
  SVG_GRID_BIN          binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN      base binary; supplies the red half of every case
  SVG_GRID_RELEASE_BIN  release binary for the F timing (default: target/release/svg_grid)
  SVG_GRID_RENDER_BIN   reference renderer (default: the D1 resvg build)
"""

import os
import pathlib
import struct
import subprocess
import sys
import tempfile
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None
RELEASE_BIN = pathlib.Path(
    os.environ.get("SVG_GRID_RELEASE_BIN") or ROOT / "target" / "release" / "svg_grid"
)
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
FONT_DIR = ROOT / "fontlib"
TMP = pathlib.Path(tempfile.mkdtemp(prefix="svg-grid-r48-"))

PASSED: list[str] = []
FAILED: list[str] = []
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
        [str(binary), *[str(a) for a in args]], capture_output=True, text=True, timeout=600
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
                pa, pb, pc = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
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


def render(svg: pathlib.Path, tag: str) -> pathlib.Path:
    png = TMP / f"{tag}.png"
    environment = dict(os.environ)
    if FONT_DIR.is_dir():
        environment["D1_FONT_DIR"] = str(FONT_DIR)
    result = subprocess.run(
        [str(RENDER), str(svg), str(png), "1"],
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
        '<svg width="100" height="80" viewBox="0 0 100 80" '
        'xmlns="http://www.w3.org/2000/svg">\n'
        '<rect data-panel-box="main" x="0" y="0" width="100" height="80" '
        'fill="none" stroke="none"/>\n'
        f"{inner}\n</svg>\n"
    )


def letter(text: str, marker: str = "0", attrs: str = "") -> str:
    return (
        f'<text data-svg-grid-label="{marker}" x="4" y="14" font-size="12" '
        f'font-family="Arial" {attrs}>{text}</text>'
    )


def compose(binary: pathlib.Path, sources: list, tag: str, extra=(), width: int = 400, plan: bool = False):
    out = TMP / f"{tag}.svg"
    if out.exists():
        out.unlink()
    args = ["plot-grid"]
    for source in sources:
        args += ["--input", source]
    args += ["--width", str(width), "--ncol", str(len(sources))]
    args += ["--plan"] if plan else ["--output", out]
    args += list(extra)
    return run(binary, args), out


def case(label, sources, extra=(), width=400, rc=2, fragments=(), old_rc=None,
         old_fragments=(), old_absent=()):
    """Run one case on the binary under test and, when given, on the base binary."""
    result, out = compose(BIN, sources, f"{label}-cur", extra, width)
    stderr = result.stderr.strip()
    ok = result.returncode == rc and all(fragment in stderr for fragment in fragments)
    check(ok, f"{label}: current rc={rc}", f"rc={result.returncode} stderr={stderr[:220]}")
    if rc == 2:
        check(not out.exists(), f"{label}: rc=2 writes no output", f"left {out}")
    if OLD_BIN is not None and old_rc is not None:
        old_result, old_out = compose(OLD_BIN, sources, f"{label}-old", extra, width)
        old_stderr = old_result.stderr.strip()
        ok_old = (
            old_result.returncode == old_rc
            and all(fragment in old_stderr for fragment in old_fragments)
            and all(fragment not in old_stderr for fragment in old_absent)
        )
        check(
            ok_old,
            f"{label}: base binary rc={old_rc} (the bug)",
            f"rc={old_result.returncode} stderr={old_stderr[:220]}",
        )
        if old_rc == 2:
            check(not old_out.exists(), f"{label}: base rc=2 writes no output", f"left {old_out}")


# --------------------------------------------------------------------- family A


def family_a():
    tspan = write("tspan.svg", panel(letter("<tspan>A</tspan>")))
    tspan2 = write("tspan2.svg", panel(letter("<tspan>A</tspan>")))
    case("A1 tspan", [tspan, tspan2], rc=2, fragments=('"A"', "#0", "#1"), old_rc=0)

    group = write("group.svg", panel('<g data-svg-grid-label="0"><text x="4" y="14" '
                                     'font-size="12" font-family="Arial">A</text></g>'))
    group2 = write("group2.svg", panel('<g data-svg-grid-label="0"><text x="4" y="14" '
                                       'font-size="12" font-family="Arial">A</text></g>'))
    case("A2 group marker", [group, group2], rc=2, fragments=('"A"',), old_rc=0)

    inline = write("inline.svg", panel(letter("A<tspan>B</tspan>")))
    nested = write("nested.svg", panel(letter("<tspan>A</tspan><tspan>B</tspan>")))
    case("A3 document order", [inline, nested], rc=2, fragments=('"AB"',), old_rc=0)

    entity = write("entity.svg", panel(letter("&#65;")))
    plain = write("plain.svg", panel(letter("A")))
    case("A4 entity control", [entity, plain], rc=2, fragments=('"A"',), old_rc=2)


# --------------------------------------------------------------------- family B


def family_b():
    padded = write("padded.svg", panel(letter(" A ")))
    plain = write("plain-b.svg", panel(letter("A")))
    case("B1 padded", [padded, plain], rc=2, fragments=('"A"',), old_rc=0)

    trailing = write("trailing.svg", panel(letter("A ")))
    case("B2 trailing", [trailing, plain], rc=2, fragments=('"A"',), old_rc=0)

    spaced = write("spaced.svg", panel(letter("A  B")))
    blank = write("blank-b.svg", panel(""))
    case("B3 internal whitespace", [spaced, blank], extra=("--labels", ",A B"),
         rc=2, fragments=('"A B"',), old_rc=0)

    empty = write("empty.svg", panel(letter("   ")))
    case("B4 empty marker", [empty, plain], rc=0, old_rc=0)

    if not RENDER.exists():
        skip("B5 render control", f"{RENDER} is missing")
        return
    left, left_out = compose(BIN, [plain], "B5-a")
    right, right_out = compose(BIN, [padded], "B5-pad")
    if left.returncode != 0 or right.returncode != 0:
        check(False, "B5 render control: both inputs compose", left.stderr + right.stderr)
        return
    xor, worst = diff(render(left_out, "B5-a"), render(right_out, "B5-pad"))
    check((xor, worst) == (0, 0), "B5 render control: A and ' A ' are pixel-identical",
          f"diff=({xor},{worst})")


# --------------------------------------------------------------------- family C


def family_c():
    visible = write("visible.svg", panel(letter("A")))
    forms = {
        "C1 defs": '<defs>' + letter("A") + "</defs>",
        "C2 symbol": "<symbol>" + letter("A") + "</symbol>",
        "C3 display": '<g display="none">' + letter("A") + "</g>",
        "C4 visibility": '<g visibility="hidden">' + letter("A") + "</g>",
        "C5 opacity": '<g opacity="0">' + letter("A") + "</g>",
    }
    for label, body in forms.items():
        hidden = write(f"{label.split()[0]}.svg", panel(body))
        case(label, [hidden, visible], rc=0, old_rc=2)

    restored = write("restored.svg", panel(
        '<g visibility="hidden">' + letter("A", attrs='visibility="visible"') + "</g>"))
    case("C6 visibility restored", [restored, visible], rc=2, fragments=('"A"',), old_rc=2)

    display_child = write("display-child.svg", panel(
        '<g display="none">' + letter("A", attrs='display="inline"') + "</g>"))
    case("C7 display child", [display_child, visible], rc=0, old_rc=2)

    if not RENDER.exists():
        skip("C8 render identity", f"{RENDER} is missing")
        return
    blank = write("blank-c.svg", panel(""))
    baseline, baseline_out = compose(BIN, [blank, visible], "C8-base")
    if baseline.returncode != 0:
        check(False, "C8 render identity: baseline composes", baseline.stderr)
        return
    baseline_png = render(baseline_out, "C8-base")
    for label, body in forms.items():
        hidden = write(f"{label.split()[0]}-render.svg", panel(body))
        result, out = compose(BIN, [hidden, visible], f"C8-{label.split()[0]}")
        if result.returncode != 0:
            check(False, f"C8 {label}: composes", result.stderr)
            continue
        xor, worst = diff(render(out, f"C8-{label.split()[0]}"), baseline_png)
        check((xor, worst) == (0, 0), f"C8 {label}: rendered figure is unchanged",
              f"diff=({xor},{worst})")


# --------------------------------------------------------------------- family D


def family_d():
    six = [write(f"six-{i}.svg", panel(letter("A"))) for i in range(6)]
    result, out = compose(BIN, six, "D1-cur", width=1200)
    stderr = result.stderr.strip()
    check(result.returncode == 2, "D1 six sites: rc=2", stderr[:220])
    check(
        "+3 more" in stderr or "(6 places)" in stderr,
        "D1 six sites: the omitted sites are counted",
        stderr[:220],
    )
    check(len(stderr) <= 200, "D1 six sites: one 200-char line", f"{len(stderr)} chars")
    if OLD_BIN is not None:
        old_result, _ = compose(OLD_BIN, six, "D1-old", width=1200)
        old_stderr = old_result.stderr.strip()
        check(
            old_result.returncode == 2 and "+3 more" not in old_stderr and "(6 places)" not in old_stderr,
            "D1 six sites: base collapses to a bare letter",
            old_stderr[:220],
        )

    p1 = write("cell-p1.svg", panel(""))
    p2 = write("cell-p2.svg", panel(""))
    inner, inner_out = compose(BIN, [p1, p2], "D2-inner", extra=("--labels", ",A"))
    if inner.returncode != 0:
        check(False, "D2 cell: inner composite composes", inner.stderr)
        return
    a = write("cell-a.svg", panel(letter("A")))
    result, out = compose(BIN, [inner_out, a], "D2-cur")
    stderr = result.stderr.strip()
    check(result.returncode == 2, "D2 cell: rc=2", stderr[:220])
    check("#0 (cell 1)" in stderr and "#1 (cell 0)" in stderr,
          "D2 cell: the real cell is reported", stderr[:220])
    if OLD_BIN is not None:
        old_result, _ = compose(OLD_BIN, [inner_out, a], "D2-old")
        old_stderr = old_result.stderr.strip()
        check(
            old_result.returncode == 2 and "#0 (cell 1)" not in old_stderr,
            "D2 cell: base reports the input index as the cell",
            old_stderr[:220],
        )

    # Direct text, not a tspan: the base binary must reach its own fix text.
    fix_a = write("fix.svg", panel(letter("A")))
    fix_b = write("fix2.svg", panel(letter("A")))
    result, out = compose(BIN, [fix_a, fix_b], "D3-cur")
    stderr = result.stderr.strip()
    check("drop the inputs' data-svg-grid-label markers" in stderr,
          "D3 fix text: names the marker to remove", stderr[:220])
    check("pass unique --labels" not in stderr,
          "D3 fix text: no impossible --labels advice", stderr[:220])
    if OLD_BIN is not None:
        old_result, _ = compose(OLD_BIN, [fix_a, fix_b], "D3-old")
        old_stderr = old_result.stderr.strip()
        check("pass unique --labels here" in old_stderr,
              "D3 fix text: base gives the impossible advice", old_stderr[:220])

    sentinel = TMP / "sentinel.svg"
    sentinel.write_text("SENTINEL\n")
    result = run(BIN, ["plot-grid", "--input", fix_a, "--input", fix_b,
                       "--width", "400", "--ncol", "2", "--output", sentinel])
    check(result.returncode == 2 and sentinel.read_text() == "SENTINEL\n",
          "D4 rc=2 leaves an existing output untouched", result.stderr.strip()[:220])

    result = run(BIN, ["plot-grid", "--input", fix_a, "--input", fix_b,
                       "--width", "400", "--ncol", "2", "--plan"])
    check(result.returncode == 2 and result.stdout == "",
          "D5 --plan fails without printing a plan", result.stderr.strip()[:220])


# --------------------------------------------------------------------- family E


def family_e():
    a = write("spacer-a.svg", panel(letter("A")))
    result, out = compose(BIN, [a, "none"], "E1-cur")
    check(result.returncode == 0, "E1 spacer alone: rc=0", result.stderr.strip()[:220])
    if OLD_BIN is not None:
        old_result, _ = compose(OLD_BIN, [a, "none"], "E1-old")
        check(old_result.returncode == 0, "E1 spacer alone: base rc=0 (control)",
              old_result.stderr.strip()[:220])

    result, out = compose(BIN, [a, "none"], "E2-cur", extra=("--labels", ",A"))
    check(result.returncode == 2 and '"A"' in result.stderr,
          "E2 explicit spacer letter: rc=2", result.stderr.strip()[:220])
    if OLD_BIN is not None:
        old_result, _ = compose(OLD_BIN, [a, "none"], "E2-old", extra=("--labels", ",A"))
        check(old_result.returncode == 2, "E2 explicit spacer letter: base rc=2 (control)",
              old_result.stderr.strip()[:220])

    if not RENDER.exists():
        skip("E3 spacer render", f"{RENDER} is missing")
        return
    result, out = compose(BIN, ["none", "none"], "E3-cur", extra=("--labels", ",A"))
    if result.returncode != 0:
        check(False, "E3 spacer render: composes", result.stderr)
        return
    pixels = ink(render(out, "E3-cur"))
    check(pixels > 0, "E3 spacer render: the explicit letter is drawn", f"ink={pixels}")


# --------------------------------------------------------------------- family F


def family_f():
    if not RELEASE_BIN.exists():
        skip("F1 40000 letters", f"{RELEASE_BIN} is missing")
        return
    body = "".join(letter(f"L{i}", marker=str(i)) for i in range(40000))
    big = write("perf.svg", panel(body))
    args = ["plot-grid", "--input", big, "--width", "400", "--ncol", "1", "--plan"]

    def best(binary):
        times = []
        for _ in range(3):
            start = time.perf_counter()
            result = run(binary, args)
            times.append(time.perf_counter() - start)
            if result.returncode != 0:
                return None, result.stderr.strip()[:220]
        return min(times), ""

    current, error = best(RELEASE_BIN)
    if current is None:
        check(False, "F1 40000 letters: current release runs", error)
        return
    check(current <= 0.2, "F1 40000 letters: current release <= 0.2 s", f"{current:.3f} s")
    print(f"      current release: {current:.3f} s")
    if OLD_BIN is not None:
        old, error = best(OLD_BIN)
        if old is None:
            check(False, "F1 40000 letters: base release runs", error)
        else:
            check(old > 0.5, "F1 40000 letters: base release is quadratic", f"{old:.3f} s")
            print(f"      base release: {old:.3f} s")


def main() -> int:
    print(f"binary under test: {BIN}")
    print(f"base binary:       {OLD_BIN or '(not set)'}")
    print(f"release binary:    {RELEASE_BIN}")
    print(f"renderer:          {RENDER}")
    print()
    family_a()
    family_b()
    family_c()
    family_d()
    family_e()
    family_f()
    print()
    if FAILED:
        print(f"== 失败 {len(FAILED)} 条（成立 {len(PASSED)} 条）==")
        return 1
    print(f"== 全部成立（{len(PASSED)} 条）==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
