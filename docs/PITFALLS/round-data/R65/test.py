#!/usr/bin/env python3
"""R65 evidence — `convert --check` must refuse a non-finite number, and the
test-count gate must notice a gutted `#[test]` body.

Families
  A  the fix, four columns per case: pre-fix rc + message, post-fix rc + message,
     whether `d1render` renders the pre-fix product anyway, and the legal-value
     control. Nine non-finite spellings (`1e999`, `NaN`, `-inf`) across
     stroke-width / x / path d / font-size / textLength / stop offset / points,
     plus two controls that must not move: `stroke-width="abc"` stays rc=1 and
     `stroke-width="5px"` stays rc=0 with byte-identical output.
  B  atomicity: `--force` keeps the old bytes, `--output-png` / `--output-svgz`
     write nothing, `--check` writes nothing.
  C  the gate: `gate_assertions.py` green on the tree, red on three mutants built
     at run time in TMP (empty body, assertions deleted but the call kept,
     `#[ignore]` added) — each names file and test.
  D  parity: 214 tracked `tests/**/*.svg` through `convert` and 58
     `tests/fixtures/**/*.svg` through `plot-grid --width 600 --ncol 1` — rc and
     bytes equal the base build (/tmp/r50base @e0c6033).
  E  the obb `clipPath`-body-references-a-pattern residue: one decisive
     experiment. The pattern inside the clip body changes nothing in either
     renderer, and the two renderers already disagree on a plain clip body, so
     no fixture can discriminate — the residue has no oracle (see meta.toml).
  T  teeth: the pre-fix build at /tmp/r65_before_bin says "compatible" and writes
     the file for every case in A.

Run:  python3 docs/PITFALLS/round-data/R65/test.py
      SVG_GRID_PIN_BIN=/tmp/r65_before_bin python3 .../R65/test.py   # A must be red
Exit: 0 all green, 1 at least one check red, 2 missing binary (fail closed).
Env:  SVG_GRID_BIN, SVG_GRID_PIN_BIN (default /tmp/r65_before_bin),
      SVG_GRID_PRE_BIN (default /tmp/r50base/target/release/svg_grid).
"""

import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
BIN = Path(os.environ.get("SVG_GRID_BIN", str(ROOT / "target/release/svg_grid")))
PIN = Path(os.environ.get("SVG_GRID_PIN_BIN", "/tmp/r65_before_bin"))
# Family A's "after" column runs on VIS: point SVG_GRID_PIN_BIN at the pre-fix
# build and every "after" assertion MUST go red (that is the teeth).
VIS = Path(os.environ.get("SVG_GRID_PIN_BIN", str(BIN)))
PRE = Path(os.environ.get("SVG_GRID_PRE_BIN", "/tmp/r50base/target/release/svg_grid"))
RENDER = Path("/tmp/d1_eval/render/target/release/d1render")
FONTDIR = ROOT / "fontlib"
GUARD = Path(__file__).resolve().parent / "gate_assertions.py"
TMP = Path(tempfile.mkdtemp(prefix="r65-evidence-"))
ENV = dict(os.environ, D1_FONT_DIR=str(FONTDIR))
PASSED, FAILED = [], []

HEAD = ('<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" '
        'viewBox="0 0 100 80">\n')


def doc(body):
    return HEAD + body + "\n</svg>\n"


# name -> (body, attribute the finding must name, value text it must quote)
CASES = {
    "stroke-width=1e999": ('<rect x="10" y="10" width="20" height="20" fill="#000" '
                           'stroke="#000" stroke-width="1e999"/>', "stroke-width", "1e999"),
    "x=1e999": ('<rect x="1e999" y="10" width="20" height="20" fill="#000"/>', "x", "1e999"),
    "d=M 1e999 0 L 10 10": ('<path d="M 1e999 0 L 10 10" fill="none" stroke="#000"/>',
                            "d", "1e999"),
    "font-size=1e999": ('<text x="10" y="20" font-size="1e999">hi</text>',
                        "font-size", "1e999"),
    "textLength=1e999": ('<text x="10" y="20" textLength="1e999">hi</text>',
                         "textLength", "1e999"),
    "offset=1e999": ('<defs><linearGradient id="g"><stop offset="1e999" '
                     'stop-color="#000"/></linearGradient></defs>'
                     '<rect width="100" height="80" fill="url(#g)"/>', "offset", "1e999"),
    "points=1e999,10 20,20": ('<polygon points="1e999,10 20,20" fill="#000"/>',
                              "points", "1e999"),
    "stroke-width=NaN": ('<rect x="10" y="10" width="20" height="20" fill="#000" '
                         'stroke-width="NaN"/>', "stroke-width", "NaN"),
    "stroke-width=-inf": ('<rect x="10" y="10" width="20" height="20" fill="#000" '
                          'stroke-width="-inf"/>', "stroke-width", "-inf"),
}
CONTROLS = {
    "stroke-width=abc": ('<rect x="10" y="10" width="20" height="20" fill="#000" '
                         'stroke-width="abc"/>', 1),
    "stroke-width=5px": ('<rect x="10" y="10" width="20" height="20" fill="#000" '
                         'stroke-width="5px"/>', 0),
}
ILLEGAL = ("1e999", "inf", "NaN", "nan")


def check(ok, label, detail=""):
    (PASSED if ok else FAILED).append(label)
    print(("PASS " if ok else "FAIL ") + label
          + (("  [%s]" % (detail,)) if not ok and detail != "" else ""))


def source(name, body):
    path = TMP / (re.sub(r"[^A-Za-z0-9]+", "-", name) + ".svg")
    path.write_text(doc(body), encoding="utf-8")
    return path


def out_path(name):
    return TMP / (re.sub(r"[^A-Za-z0-9]+", "-", name) + ".out.svg")


def convert(binary, src, name, extra=()):
    out = out_path(name)
    if out.exists():
        out.unlink()
    proc = subprocess.run([str(binary), "convert", "--input", str(src),
                           "--output", str(out)] + list(extra), capture_output=True, text=True)
    return proc, (out.read_bytes() if out.exists() else b"")


def check_run(binary, src):
    proc = subprocess.run([str(binary), "convert", "--input", str(src), "--check"],
                          capture_output=True, text=True)
    return proc.returncode, proc.stdout + proc.stderr


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


def diff(a, b):
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
    return sum(1 for y in range(h) for x in range(w)
               if any(v != 255 for v in (rows[y][x * ch:x * ch + ch][:3]
                                         if ch >= 3 else rows[y][x * ch] * 3)))


def render(src, out):
    proc = subprocess.run([str(RENDER), str(src), str(out), "1"], env=ENV,
                          capture_output=True, text=True)
    return proc.returncode, Path(out).exists()


def family_a():
    print("\n== A 非有限数：修前 · 修后 · d1render · 合法值对照 ==")
    for name, (body, attr, value) in CASES.items():
        src = source(name, body)
        before, before_bytes = convert(PIN, src, name + ".before")
        after, after_bytes = convert(VIS, src, name + ".after")
        check(before.returncode == 0 and before_bytes,
              "A/%s 修前 rc=0 且写出产物（漏洞）" % name,
              "rc=%d wrote=%s" % (before.returncode, bool(before_bytes)))
        check(after.returncode == 1 and not after_bytes,
              "A/%s 修后 rc=1 且不写产物" % name,
              "rc=%d wrote=%s" % (after.returncode, bool(after_bytes)))
        rc, text = check_run(VIS, src)
        check(rc == 1 and attr in text and value in text and "not a finite number" in text,
              "A/%s 修后文案点名属性与值" % name, text.strip()[-300:])
        # d1render 列：修前产物带着非法数值，渲染器照样 rc=0 —— 只有 convert 能拦。
        product = out_path(name + ".before")
        rendered, png = render(product, TMP / (re.sub(r"[^A-Za-z0-9]+", "-", name) + ".png"))
        illegal = [token for token in ILLEGAL if token in product.read_text()]
        check(rendered == 0 and png and illegal,
              "A/%s d1render 对修前产物仍 rc=0（渲染器不是门，%s 留在产物里）"
              % (name, illegal), "rc=%d png=%s" % (rendered, png))
    for name, (body, want) in CONTROLS.items():
        src = source(name, body)
        before, before_bytes = convert(PIN, src, name + ".before")
        after, after_bytes = convert(VIS, src, name + ".after")
        check(before.returncode == want and after.returncode == want,
              "A/%s 对照 rc 不变（%d）" % (name, want),
              "before=%d after=%d" % (before.returncode, after.returncode))
        if want == 0:
            check(before_bytes == after_bytes,
                  "A/%s 对照产物逐字节不变" % name,
                  "%d vs %d bytes" % (len(before_bytes), len(after_bytes)))


def family_b():
    print("\n== B 原子性：失败时不碰既有产物 ==")
    src = source("atomic", CASES["stroke-width=1e999"][0])
    out = TMP / "existing.svg"
    out.write_text("SENTINEL", encoding="utf-8")
    proc = subprocess.run([str(BIN), "convert", "--input", str(src), "--output", str(out),
                           "--force"], capture_output=True, text=True)
    check(proc.returncode == 1 and out.read_text() == "SENTINEL",
          "B/convert --force 不覆盖既有产物", "rc=%d" % proc.returncode)
    for flag, suffix in (("--output-png", ".png"), ("--output-svgz", ".svgz")):
        svg, extra = TMP / ("atomic" + suffix + ".svg"), TMP / ("atomic" + suffix)
        for path in (svg, extra):
            if path.exists():
                path.unlink()
        proc = subprocess.run([str(BIN), "plot-grid", "--input", str(src), "--output", str(svg),
                               flag, str(extra), "--width", "200", "--ncol", "1"],
                              capture_output=True, text=True)
        check(proc.returncode == 2 and not extra.exists() and not svg.exists(),
              "B/plot-grid %s 失败时不写任何文件（rc=2）" % flag, "rc=%d" % proc.returncode)
    rc, _ = check_run(BIN, src)
    check(rc == 1 and not (TMP / "atomic3.svg").exists(), "B/convert --check 不写任何文件", rc)


def family_c():
    print("\n== C 源码级守卫：函数体里必须有断言，且禁止 #[ignore] ==")
    if not GUARD.is_file():
        check(False, "C/守卫脚本存在", str(GUARD))
        return
    proc = subprocess.run([sys.executable, str(GUARD), str(ROOT)],
                          capture_output=True, text=True)
    check(proc.returncode == 0 and "PASS" in proc.stdout,
          "C/树上 724 条 `#[test]` 都有断言、0 条 `#[ignore]`",
          (proc.stdout + proc.stderr).strip()[-400:])
    mutants = {
        "empty-body": ("src/layout.rs", "short_last_row_keeps_the_full_grid_cell_size",
                       "the body is empty"),
        "call-kept": ("src/layout.rs", "short_last_row_keeps_the_full_grid_cell_size",
                      "no assertion in the body"),
        "ignore": ("tests/selftest/doctor/masking.rs",
                   "a_w4a_phrase_inside_a_kept_value_cannot_mask_w7", "#[ignore]"),
    }
    for label, (rel, test, phrase) in mutants.items():
        tree = TMP / ("mut-" + label)
        if tree.exists():
            shutil.rmtree(tree)
        for top in ("src", "tests"):
            for path in (ROOT / top).rglob("*.rs"):
                target = tree / path.relative_to(ROOT)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, target)
        path = tree / rel
        text = path.read_text()
        if label == "empty-body":
            text = re.sub(r"(fn %s\(\)[^{]*\{)(.*?)(\n\})" % test,
                          r"\1\3", text, count=1, flags=re.S)
        elif label == "call-kept":
            text = re.sub(r"(fn %s\(\)[^{]*\{)(.*?)(\n\})" % test,
                          r"\1\n    let _ = fit(5, Some(2), None);\3", text, count=1, flags=re.S)
        else:
            text = text.replace("fn %s() {" % test, "#[ignore]\nfn %s() {" % test, 1)
        path.write_text(text)
        proc = subprocess.run([sys.executable, str(GUARD), str(tree)],
                              capture_output=True, text=True)
        out = proc.stdout + proc.stderr
        check(proc.returncode == 1 and "FAIL" in out and test in out and phrase in out,
              "C/变异 %s ⇒ 红且点名 %s" % (label, test), out.strip()[-300:])


def family_d():
    print("\n== D 语料逐字节不变（对照 /tmp/r50base @e0c6033）==")
    listed = subprocess.run(["git", "-C", str(ROOT), "ls-files", "--", "tests"],
                            capture_output=True, check=True)
    tracked = sorted(rel for rel in listed.stdout.decode().split("\n") if rel.endswith(".svg"))
    fixtures = sorted(str(p.relative_to(ROOT)) for p in ROOT.glob("tests/fixtures/**/*.svg"))
    check(len(tracked) == 214, "D/214 tracked tests/**/*.svg", len(tracked))
    check(len(fixtures) == 58, "D/58 tracked tests/fixtures/**/*.svg", len(fixtures))

    def run(binary, args, out):
        if out.exists():
            out.unlink()
        proc = subprocess.run([str(binary)] + args, capture_output=True)
        return proc.returncode, (out.read_bytes() if out.exists() else b""), proc.stdout, proc.stderr

    diffs = []
    for rel in tracked:
        for mode in ([], ["--report"]):
            out = TMP / "d.svg"
            a = run(PRE, ["convert", "--input", str(ROOT / rel), "--output", str(out)] + mode, out)
            b = run(BIN, ["convert", "--input", str(ROOT / rel), "--output", str(out)] + mode, out)
            if a != b:
                diffs.append((rel, mode, a[0], b[0]))
    check(not diffs, "D/214 个 convert（含 --report）rc+产物逐字节相同", diffs[:3])
    grid_diffs = []
    for rel in fixtures:
        for mode in ([], ["--plan"]):
            out = TMP / "g.svg"
            outs = [run(binary, ["plot-grid", "--input", str(ROOT / rel), "--output", str(out),
                                 "--width", "600", "--ncol", "1"] + mode, out)
                    for binary in (PRE, BIN)]
            if outs[0] != outs[1]:
                grid_diffs.append((rel, mode, outs[0][0], outs[1][0]))
    check(not grid_diffs, "D/58 个 plot-grid（含 --plan）rc+产物逐字节相同", grid_diffs[:3])


def family_e():
    print("\n== E obb clipPath body 引用 pattern：一次判定实验 ==")
    plain = doc('<defs><clipPath id="c" clipPathUnits="objectBoundingBox">'
                '<rect width="1" height="0.5" fill="white"/></clipPath></defs>'
                '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>')
    patterned = doc('<defs><pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8">'
                    '<rect width="4" height="4" fill="white"/></pattern>'
                    '<clipPath id="c" clipPathUnits="objectBoundingBox">'
                    '<rect width="1" height="0.5" fill="url(#p)"/></clipPath></defs>'
                    '<rect width="100" height="80" fill="red" clip-path="url(#c)"/>')
    shots = {}
    for label, text in (("plain", plain), ("pattern", patterned)):
        src = TMP / ("e-%s.svg" % label)
        src.write_text(text, encoding="utf-8")
        d1 = TMP / ("e-%s-d1.png" % label)
        rc, made = render(src, d1)
        cairo = TMP / ("e-%s-cairo.png" % label)
        subprocess.run([sys.executable, "-c",
                        "import cairosvg,sys;cairosvg.svg2png(url=sys.argv[1],"
                        "write_to=sys.argv[2],scale=1)", str(src), str(cairo)],
                       capture_output=True)
        shots[label] = (rc, made, d1, cairo.exists(), cairo)
    rc, made, d1_plain, cairo_ok, cairo_plain = shots["plain"]
    check(rc == 0 and made and cairo_ok, "E/两台渲染器都出图（plain 对照）", shots["plain"])
    if not (made and cairo_ok):
        return
    d1_pattern, cairo_pattern = shots["pattern"][2], shots["pattern"][4]
    check(diff(d1_plain, d1_pattern) == (0, 0),
          "E/d1render：clip body 里的 pattern 不改变任何像素（xor 0/0）",
          diff(d1_plain, d1_pattern))
    plain_diff = diff(d1_plain, cairo_plain)
    check(plain_diff != (0, 0),
          "E/两台渲染器在**不含 pattern** 的 plain 对照上就不一致 ⇒ 无共同 oracle",
          "diff=%s ink d1=%d cairo=%d" % (plain_diff, ink(d1_plain), ink(cairo_plain)))
    check(diff(d1_pattern, cairo_pattern) == plain_diff,
          "E/pattern 版的跨渲染器差异与 plain 相同 ⇒ 该残留无判别力",
          "pattern=%s plain=%s" % (diff(d1_pattern, cairo_pattern), plain_diff))


def family_t():
    print("\n== T 有牙：修前二进制在 A 族上必须红 ==")
    check(PIN.exists(), "T/修前二进制存在：%s" % PIN, str(PIN))
    if not PIN.exists():
        return
    src = source("teeth", CASES["stroke-width=1e999"][0])
    rc, text = check_run(PIN, src)
    check(rc == 0 and "compatible" in text,
          "T/修前 `--check` 对 stroke-width=1e999 判 compatible（rc=0）", text.strip()[-200:])
    proc, data = convert(PIN, src, "teeth")
    check(proc.returncode == 0 and b"1e999" in data,
          "T/修前写出产物且把 1e999 原样留在里面", "rc=%d" % proc.returncode)
    print("     (T: SVG_GRID_PIN_BIN is %s; with it set, family A must be red)"
          % ("set -> A's after column runs on the pin" if os.environ.get("SVG_GRID_PIN_BIN")
             else "unset, A's after column runs on SVG_GRID_BIN"))


def main():
    for binary, label in ((BIN, "SVG_GRID_BIN"), (PIN, "pre-fix build"),
                          (PRE, "parity baseline"), (RENDER, "d1render")):
        if not binary.exists():
            print("missing %s: %s" % (label, binary), file=sys.stderr)
            return 2
    print("BIN=%s\nVIS=%s\nPIN=%s\nPRE=%s\n" % (BIN, VIS, PIN, PRE))
    family_a()
    family_b()
    family_c()
    family_d()
    family_e()
    family_t()
    print("\n%d passed, %d failed" % (len(PASSED), len(FAILED)))
    return 0 if not FAILED else 1


if __name__ == "__main__":
    sys.exit(main())
