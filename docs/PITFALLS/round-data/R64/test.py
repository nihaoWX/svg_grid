#!/usr/bin/env python3
"""R64 evidence — a `<use href>` target is a reference target even when it is not
one of the seven definition tags, and `<foreignObject>`/`<switch>` stop being
silently accepted by `convert`.

Families
  A  the fix: `<rect id>` / `<g id>` in `<defs>` instantiated by `<use>` now take
     the referrer's policy (u2/u3/u5/u17 at xor 0 / ink equal; u3's group children,
     u5's `<use>` inside the definition, and u17's shared copy all covered). u16
     (`data-scale="x"`) and u4 (two policies, one definition → two copies) are
     **attribute** assertions: a `(2, 1)` placement has no uniform `d1render`
     oracle, so pixels cannot decide them.
  A2 the mechanism: take the **pre-fix** product for u2/u3/u5, write only the
     numbers the renderer expects, render it at scale 1 — xor 0 / ink equal. That
     is what makes the defect "the numbers never followed the panel", not "the
     composer drops or duplicates the motif".
  B  the controls that must not move by a byte: `<symbol viewBox>` through
     `<use>` (u6) and a `<use>` inside a `viewBox` pattern body (u15) compare
     byte-for-byte against the pre-fix product.
  C  `<foreignObject>`/`<switch>`: pre-fix `convert --check` said compatible,
     rc=0 and wrote the file; now `--check` is rc=1 with a finding and
     `--output` writes nothing. Measured with `--verbose`: the pre-fix build's
     stderr was 0 bytes.
  D  regression: every tracked `tests/**/*.svg` (214) through `convert` and every
     `tests/fixtures/**/*.svg` (58) through `plot-grid --width 600 --ncol 1` —
     rc and output bytes equal the base build (/tmp/r50base @e0c6033).
  T  teeth: the pre-R64 build at /tmp/r64_before_bin is red on family A; because
     family A runs on `SVG_GRID_PIN_BIN` when it is set, that invocation exits 1.

Registered, not fixed (see meta.toml for the full text)
  C-reg  an obb `clipPath` body that fills with a `pattern` stays at 16000/255
     (default plot margin, 200x160, s=2) before and after this round; with
     `--plot-margin 0` the same case is 12000/255. The review's hand candidate
     (clip body kept at 1x0.5, pattern tile 16/8) does **not** reproduce 0/0 on
     this device: dropping the clip body's x/y, the tile at 16x8 / 16x16, the
     tile's inner rect at 8x8, and their combinations all leave the xor at
     16000/255, and tile 8 vs 16 renders identical pixels. d1render and cairosvg
     both paint the source as 16000 red and the product as 25%-covered 4000 red,
     i.e. neither consumes the pattern coverage inside an obb clip body. What to
     fix is the coordinate system of that body, not a number — and no tracked
     input uses it, so this round only registers it.
  u7-reg  a `<use>` target written in the drawable tree stays put (xor 600/255,
     ink 200 vs 800): the same id is both "drawn in place" and "instantiated", so
     a slot would scale the in-place drawing twice and cloning would draw the body
     twice.

Run:  python3 docs/PITFALLS/round-data/R64/test.py
      SVG_GRID_PIN_BIN=/tmp/r64_before_bin python3 .../R64/test.py   # must be red
Exit: 0 all green, 1 at least one check red, 2 missing binary (fail closed).
Env:  SVG_GRID_BIN (this build), SVG_GRID_PIN_BIN (pre-fix build, default
      /tmp/r64_before_bin; when set, family A/A2/A4/B run on it),
      SVG_GRID_PRE_BIN (parity baseline, default /tmp/r50base/target/release/svg_grid).
"""

import os
import re
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r64_before_bin"))
# Family A is the fix's own face: point it at the pre-fix build and every
# assertion MUST go red (T only records that the build is red, too).
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path("/tmp/d1_eval/render/target/release/d1render")
FONTDIR = ROOT / "fontlib"
TMP = Path(tempfile.mkdtemp(prefix="r64-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
ATTR_RE = re.compile(r'([\w:-]+)\s*=\s*"([^"]*)"')
TAG_RE = re.compile(r'<([A-Za-z][\w:-]*)((?:\s+[\w:-]+\s*=\s*"[^"]*")*)\s*/?>')
PASSED, FAILED = [], []


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + (("  [%s]" % (detail,)) if not ok and detail != "" else ""))


def read_png(path):
    data = Path(path).read_bytes()
    pos, idat, w, h, ch = 8, b"", 0, 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            w, h, depth, color = struct.unpack(">IIBB", body[:10])
            ch = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]
            assert depth == 8, depth
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = w * ch
    rows, prev = [], bytearray(stride)
    for y in range(h):
        base = y * (stride + 1)
        f = raw[base]
        line = bytearray(raw[base + 1:base + 1 + stride])
        for x in range(stride):
            left = line[x - ch] if x >= ch else 0
            up = prev[x]
            ul = prev[x - ch] if x >= ch else 0
            if f == 1:
                line[x] = (line[x] + left) & 0xFF
            elif f == 2:
                line[x] = (line[x] + up) & 0xFF
            elif f == 3:
                line[x] = (line[x] + (left + up) // 2) & 0xFF
            elif f == 4:
                p = left + up - ul
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - ul)
                pred = left if pa <= pb and pa <= pc else (up if pb <= pc else ul)
                line[x] = (line[x] + pred) & 0xFF
        rows.append(line)
        prev = line
    return w, h, ch, rows


def xor(a, b):
    wa, ha, ca, ra = read_png(a)
    wb, hb, cb, rb = read_png(b)
    assert (wa, ha, ca) == (wb, hb, cb), ((wa, ha, ca), (wb, hb, cb))
    count, worst = 0, 0
    for y in range(ha):
        for x in range(wa):
            pa = ra[y][x * ca:x * ca + ca]
            pb = rb[y][x * cb:x * cb + cb]
            d = max(abs(u - v) for u, v in zip(pa, pb))
            if d:
                count += 1
                worst = max(worst, d)
    return count, worst


def ink(path):
    w, h, ch, rows = read_png(path)
    count = 0
    for y in range(h):
        for x in range(w):
            px = rows[y][x * ch:x * ch + ch]
            rgb = px[:3] if ch >= 3 else px * 3
            if any(v != 255 for v in rgb):
                count += 1
    return count


def render(src, out, scale):
    proc = subprocess.run([str(RENDER), str(src), str(out), str(scale)], env=ENV,
                          capture_output=True, text=True)
    if proc.returncode != 0 or not Path(out).exists():
        raise SystemExit("d1render failed: %s %s" % (proc.stdout, proc.stderr))
    return out


def grid(binary, name, sources, extra):
    sources = sources if isinstance(sources, list) else [sources]
    out = TMP / (name + ".svg")
    if out.exists():
        out.unlink()
    args = [str(binary), "plot-grid"]
    for source in sources:
        args += ["--input", str(source)]
    args += ["--output", str(out)] + [str(x) for x in extra]
    proc = subprocess.run(args, capture_output=True)
    return proc.returncode, (out.read_bytes() if out.exists() else b""), proc.stderr


def compose(binary, name, svg, extra=None):
    src = TMP / (name + ".src.svg")
    src.write_text(svg, encoding="utf-8")
    args = ["--width", "200", "--height", "160", "--ncol", "1", "--plot-margin", "0"]
    rc, data, err = grid(binary, name, src, args + (extra or []))
    return src, data, rc, err


def elements(svg, tag):
    return [dict(ATTR_RE.findall(m.group(2))) for m in TAG_RE.finditer(svg)
            if m.group(1) == tag]


def attr_in(svg, tag, attr, id_ends=None):
    for element in elements(svg, tag):
        if id_ends is not None:
            ident = element.get("id", "")
            if not (ident == id_ends or ident.endswith("-" + id_ends)):
                continue
        if attr in element:
            return element[attr]
    return None


HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
        'viewBox="0 0 100 80"><rect data-panel-box="main" x="0" y="0" '
        'width="100" height="80" fill="none" stroke="none"/>')


def doc(body):
    return HEAD + body + "</svg>"


U2 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/></defs>'
         '<g data-scale="xy"><use href="#r" x="10" y="10"/></g>')
U3 = doc('<defs><g id="g"><rect width="10" height="10" fill="#000"/>'
         '<circle cx="15" cy="5" r="3" fill="#000"/></g></defs>'
         '<g data-scale="xy"><use href="#g" x="10" y="10"/></g>')
U5 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/>'
         '<g id="g"><use href="#r" x="2" y="2"/></g></defs>'
         '<g data-scale="xy"><use href="#g" x="10" y="10"/></g>')
U17 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/></defs>'
          '<g data-scale="xy"><use href="#r" x="10" y="10"/>'
          '<use href="#r" x="40" y="10"/></g>')
U16 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/></defs>'
          '<g data-scale="x"><use href="#r" x="10" y="10"/></g>')
U4 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/></defs>'
         '<g data-scale="xy"><use href="#r" x="10" y="10"/></g>'
         '<g data-scale="y"><use href="#r" x="10" y="40"/></g>')
U6 = doc('<defs><symbol id="s" viewBox="0 0 10 10"><rect width="10" height="10" '
         'fill="#000"/></symbol></defs>'
         '<g data-scale="xy"><use href="#s" x="10" y="10" width="20" height="20"/></g>')
U15 = doc('<defs><rect id="r" width="10" height="10" fill="#000"/>'
          '<pattern id="p" patternUnits="userSpaceOnUse" width="10" height="10" '
          'viewBox="0 0 10 10"><use href="#r" x="0" y="0"/></pattern></defs>'
          '<g data-scale="xy"><rect width="100" height="80" fill="url(#p)"/></g>')
U7 = doc('<g id="g"><rect width="10" height="10" fill="#000"/></g>'
         '<g data-scale="xy"><use href="#g" x="10" y="10"/></g>')


def measure(name, svg, binary=VIS):
    src, data, rc, err = compose(binary, name, svg)
    if rc != 0 or not data:
        return None, "FAIL rc=%d %s" % (rc, err[:200])
    cell, ours, ref = TMP / (name + ".cell.svg"), TMP / (name + ".ours.png"), TMP / (name + ".ref.png")
    cell.write_bytes(data)
    render(cell, ours, 1)
    render(src, ref, 2)
    return data.decode("utf-8", "replace"), (xor(ours, ref), ink(ours), ink(ref))


def pixel_case(name, label):
    text, info = measure(name, globals()[name.upper()])
    if text is None:
        check(False, label, info)
        return
    diff, ours, ref = info
    check(diff == (0, 0) and ours == ref, label,
          "xor=%s ink=%d ref=%d" % (diff, ours, ref))


def rewrite_tag(text, tag, match_attrs, updates):
    """Rewrite the attributes of the tags matching `match_attrs`, nothing else."""
    pieces, last = [], 0
    for match in TAG_RE.finditer(text):
        if match.group(1) != tag:
            continue
        attrs = dict(ATTR_RE.findall(match.group(2)))
        if any(attrs.get(k) != v for k, v in match_attrs.items()):
            continue
        span = text[match.start():match.end()]
        for key, value in updates.items():
            span = re.sub(r'(\b%s\s*=\s*")[^"]*(")' % re.escape(key),
                          lambda m: m.group(1) + value + m.group(2), span, count=1)
        pieces.append(text[last:match.start()])
        pieces.append(span)
        last = match.end()
    pieces.append(text[last:])
    return "".join(pieces)


def hand_fixed(name, edits):
    """Pre-fix product + only the expected numbers ⇒ xor 0 / ink equal."""
    src, data, rc, err = compose(PIN, name + "-pin", globals()[name.upper()])
    text = data.decode("utf-8", "replace")
    fixed = text
    for tag, match_attrs, updates in edits:
        fixed = rewrite_tag(fixed, tag, match_attrs, updates)
    path = TMP / (name + "-fixed.svg")
    path.write_text(fixed, encoding="utf-8")
    ours, ref = TMP / (name + "-fixed.png"), TMP / (name + "-fixed-ref.png")
    render(path, ours, 1)
    render(src, ref, 2)
    check(xor(ours, ref) == (0, 0) and ink(ours) == ink(ref),
          "A2/pre-fix product + the expected numbers is exact: " + name,
          "xor=%s ink=%d ref=%d" % (xor(ours, ref), ink(ours), ink(ref)))


def family_a():
    pixel_case("u2", "A/a <use> of a <rect> in <defs> scales: xor 0, ink 400 (=400)")
    pixel_case("u3", "A/a <use> of a <g> in <defs> scales its whole subtree")
    pixel_case("u5", "A/a <use> inside the referenced definition scales too")
    pixel_case("u17", "A/two <use>s of one definition share the scaled copy")
    text, info = measure("u16", U16)
    if text is None:
        check(False, "A2/u16 single-axis attributes", info)
    else:
        ok = (attr_in(text, "rect", "width", "r") == "20"
              and attr_in(text, "rect", "height", "r") == "10"
              and attr_in(text, "use", "x") == "20"
              and attr_in(text, "use", "y") == "10")
        check(ok, "A/u16 data-scale=x scales def width and use x only",
              [attr_in(text, "rect", "width", "r"), attr_in(text, "rect", "height", "r"),
               attr_in(text, "use", "x"), attr_in(text, "use", "y")])
    text, info = measure("u4", U4)
    if text is None:
        check(False, "A2/u4 per-policy copies", info)
    else:
        ok = (attr_in(text, "rect", "width", "r") == "20"
              and attr_in(text, "rect", "height", "r") == "20"
              and attr_in(text, "rect", "width", "r--svg-grid-y") == "10"
              and attr_in(text, "rect", "height", "r--svg-grid-y") == "20")
        check(ok, "A/u4 each per-policy copy scales its own way",
              [attr_in(text, "rect", "width", "r"), attr_in(text, "rect", "height", "r"),
               attr_in(text, "rect", "width", "r--svg-grid-y"),
               attr_in(text, "rect", "height", "r--svg-grid-y")])
    # u7 is registered, not fixed: the target is drawn where it sits as well.
    text, info = measure("u7", U7)
    if text is None:
        check(False, "A/u7 registration", info)
    else:
        body = re.search(r'<g id="[^"]*-g">\s*<rect([^>]*)>', text)
        width = dict(ATTR_RE.findall(body.group(1))).get("width") if body else None
        check(width == "10",
              "A/u7 registered residual: a target in the drawable tree stays put", width)


def family_a2():
    hand_fixed("u2", [("rect", {"width": "10", "height": "10"},
                       {"width": "20", "height": "20"})])
    hand_fixed("u3", [("rect", {"width": "10", "height": "10"},
                       {"width": "20", "height": "20"}),
                      ("circle", {"cx": "15", "cy": "5", "r": "3"},
                       {"cx": "30", "cy": "10", "r": "6"})])
    hand_fixed("u5", [("rect", {"width": "10", "height": "10"},
                       {"width": "20", "height": "20"}),
                      ("use", {"x": "2", "y": "2"}, {"x": "4", "y": "4"})])


def family_b():
    for name, label in (("u6", "B/<symbol viewBox> through <use> is byte-identical"),
                        ("u15", "B/<use> in a viewBox pattern body is byte-identical")):
        _, new, rc, err = compose(BIN, name + "-new", globals()[name.upper()])
        _, old, rc_old, err_old = compose(PIN, name + "-old", globals()[name.upper()])
        check(rc == 0 and rc_old == 0 and new == old, label,
              "rc=%d/%d differ=%s" % (rc, rc_old, new != old))
    text, info = measure("u6", U6, BIN)
    body = None if text is None else re.search(r"<symbol[^>]*>(.*?)</symbol>", text, re.S)
    width = None if body is None else dict(ATTR_RE.findall(body.group(1))).get("width")
    check(width == "10", "B/the symbol body keeps its viewBox numbers", width)
    text, info = measure("u15", U15, BIN)
    ok = (text is not None and attr_in(text, "pattern", "width", "p") == "20"
          and attr_in(text, "rect", "width", "r") == "10")
    check(ok, "B/the pattern tile scales while its verbatim body does not",
          None if text is None else [attr_in(text, "pattern", "width", "p"),
                                     attr_in(text, "rect", "width", "r")])


RAW_SWITCH = """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect x="10" y="10" width="10" height="10" fill="#000"/>
<switch><rect x="30" y="10" width="10" height="10" fill="#f00"/><rect x="50" y="10" width="10" height="10" fill="#0f0"/></switch>
</svg>
"""
RAW_FOREIGN = """<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
<rect x="10" y="10" width="10" height="10" fill="#000"/>
<foreignObject x="30" y="10" width="20" height="20"><rect x="30" y="10" width="20" height="20" fill="#f00"/></foreignObject>
</svg>
"""


def convert_run(binary, name, text, extra):
    src = TMP / (name + ".svg")
    src.write_text(text, encoding="utf-8")
    out = TMP / (name + ".out.svg")
    if out.exists():
        out.unlink()
    args = [str(binary), "convert", "--input", str(src)]
    if "--check" not in extra:
        args += ["--output", str(out)]
    args += extra
    proc = subprocess.run(args, capture_output=True)
    return proc.returncode, proc.stdout, proc.stderr, out.exists()


def family_c():
    for name, text, tag in (("c-switch", RAW_SWITCH, "<switch>"),
                            ("c-foreign", RAW_FOREIGN, "<foreignObject>")):
        rc_old, out_old, err_old, made_old = convert_run(
            PIN, name + "-old", text, ["--check", "--verbose"])
        check(rc_old == 0 and err_old == b"" and b"compatible" in out_old,
              "C/pre-fix %s was accepted silently (rc=0, 0-byte stderr)" % tag,
              (rc_old, err_old[:120]))
        rc_old_w, _, err_old_w, made_old_w = convert_run(
            PIN, name + "-old-write", text, [])
        check(rc_old_w == 0 and made_old_w and err_old_w == b"",
              "C/pre-fix %s conversion wrote the file (rc=0)" % tag,
              (rc_old_w, err_old_w[:120]))
        rc, out, err, made = convert_run(BIN, name + "-new", text, ["--check", "--verbose"])
        check(rc == 1 and b"NOT compatible" in out and tag.encode() in out,
              "C/%s is now a hard finding with rc=1" % tag, (rc, out[:200]))
        rc_w, _, err_w, made_w = convert_run(BIN, name + "-write", text, [])
        check(rc_w == 1 and not made_w and b"no output written" in err_w,
              "C/%s writes no output file" % tag, (rc_w, err_w[:160]))


def family_d():
    listed = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--", "tests"],
                            capture_output=True, check=True)
    tracked = sorted(rel for rel in listed.stdout.decode().split("\n")
                     if rel.endswith(".svg"))
    fixtures = sorted(str(p.relative_to(ROOT)) for p in ROOT.glob("tests/fixtures/**/*.svg"))
    diffs, failing = [], 0
    for rel in tracked:
        rc, data, _, _ = convert_run_bin(BIN, rel)
        rc_old, data_old, _, _ = convert_run_bin(PRE, rel)
        if rc != rc_old or data != data_old:
            diffs.append(rel)
        failing += rc != 0
    print("     (D: convert %d tracked, %d fail-closed by design)" % (len(tracked), failing))
    diffs_grid, failing_grid = [], []
    for rel in fixtures:
        args = ["--width", "600", "--ncol", "1"]
        rc, data, _ = grid(BIN, Path(rel).stem, ROOT / rel, args)
        rc_old, data_old, _ = grid(PRE, Path(rel).stem + "-old", ROOT / rel, args)
        if rc != rc_old or data != data_old:
            diffs_grid.append(rel)
        if rc != 0:
            failing_grid.append(rel)
    print("     (D: plot-grid %d fixtures, %d fail-closed by design)"
          % (len(fixtures), len(failing_grid)))
    check(len(tracked) == 214, "D/214 tracked tests/**/*.svg", len(tracked))
    check(len(fixtures) == 58, "D/58 tracked tests/fixtures/**/*.svg", len(fixtures))
    check(not diffs, "D/convert rc+bytes equal the base build (214 runs)", diffs[:3])
    check(not diffs_grid, "D/plot-grid rc+bytes equal the base build (58 runs)", diffs_grid[:3])


CONVERT_CACHE = {}


def convert_run_bin(binary, rel):
    """Run `convert` (write mode) on a tracked file, caching per binary."""
    key = (str(binary), rel)
    if key in CONVERT_CACHE:
        return CONVERT_CACHE[key]
    out = TMP / ("conv-%d-%d.svg" % (abs(hash(str(binary))) % 10 ** 6,
                                     abs(hash(rel)) % 10 ** 6))
    if out.exists():
        out.unlink()
    proc = subprocess.run([str(binary), "convert", "--input", str(ROOT / rel),
                           "--output", str(out)], capture_output=True)
    data = out.read_bytes() if out.exists() else b""
    result = (proc.returncode, data, proc.stderr, out.exists())
    CONVERT_CACHE[key] = result
    return result


def family_t():
    check(PIN.exists(), "T/pre-fix build exists: %s" % PIN, str(PIN))
    if not PIN.exists():
        return
    text, info = measure("u2", U2, PIN)
    check(text is not None and info[0] != (0, 0),
          "T/pre-fix build is red on u2 (teeth)", info)
    check(attr_in(text or "", "rect", "width", "r") == "10",
          "T/pre-fix build kept the def at 10", attr_in(text or "", "rect", "width", "r"))
    print("     (T: SVG_GRID_PIN_BIN is %s; with it set, family A/A2/A4 must be red)"
          % ("set -> VIS is the pin" if os.environ.get("SVG_GRID_PIN_BIN")
             else "unset, VIS is SVG_GRID_BIN"))


def main():
    for binary, label in ((BIN, "SVG_GRID_BIN"), (VIS, "visible build"), (PRE, "parity baseline"),
                          (RENDER, "d1render")):
        if not binary.exists():
            print("missing %s: %s" % (label, binary), file=sys.stderr)
            return 2
    print("BIN=%s\nVIS=%s\nPRE=%s\nPIN=%s\n" % (BIN, VIS, PRE, PIN))
    family_a()
    family_a2()
    family_b()
    family_c()
    family_d()
    family_t()
    print("\n%d passed, %d failed" % (len(PASSED), len(FAILED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
