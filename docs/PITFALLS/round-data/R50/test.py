#!/usr/bin/env python3
"""R50 evidence: XML 1.0 illegal characters must never reach the output.

R47 decoded character references with `quick_xml::escape::unescape`, which
accepts references such as `&#1;` / `&#xFFFE;` that are not legal XML 1.0
characters.  The decoded control character was written into the product, so
`convert` returned 0 while the product was no longer well-formed.  The
attribute path had the same hole before R47.  R50 adds one shared `Char`
predicate and fails closed on both paths.

Families
  A  legal references (10) on the text and attribute paths: rc=0, output
     written, output re-parses as XML 1.0 and contains no illegal character.
  B  illegal references (12) on both paths: rc=1, stderr names the reference
     as written and the codepoint, no output written.
  C  strong invariant: a batch of inputs (legal refs, illegal refs, predefined
     entities, CDATA, literal control characters) — every rc=0 output
     re-parses and contains no illegal character; every rc=1 writes nothing.
  D  (SVG_GRID_OLD_BIN) the pre-fix binary must be red on the seven references
     it used to accept (rc=0 + non-well-formed output) and green on the legal
     ones.  This is the "the assertions have teeth" half.
  E  (SVG_GRID_RENDER_BIN) the reference renderer on the ORIGINAL inputs:
     legal references render (rc=0); the nine references resvg rejects must
     fail (rc=3).  resvg is lenient on surrogates / overflow, so those three
     are skipped with a note (cairosvg/libxml2 rejects them; see the report).

Env
  SVG_GRID_BIN        binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN    pre-fix binary; supplies family D (skipped when unset)
  SVG_GRID_RENDER_BIN reference renderer (default: the D1 resvg build)
"""

import os
import pathlib
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None
RENDER = pathlib.Path(
    os.environ.get("SVG_GRID_RENDER_BIN") or "/tmp/d1_eval/render/target/release/d1render"
)
FONT_DIR = ROOT / "fontlib"

LEGAL = [
    "&#9;", "&#10;", "&#13;", "&#32;", "&#65;",
    "&#x1F600;", "&#xFFFD;", "&#x10FFFF;", "&#xE000;", "&#xD7FF;",
]
ILLEGAL = [
    ("&#0;", "U+0000"), ("&#x0;", "U+0000"), ("&#1;", "U+0001"),
    ("&#8;", "U+0008"), ("&#11;", "U+000B"), ("&#12;", "U+000C"),
    ("&#31;", "U+001F"), ("&#xFFFE;", "U+FFFE"), ("&#xFFFF;", "U+FFFF"),
    ("&#55296;", "U+D800"), ("&#xDFFF;", "U+DFFF"), ("&#x110000;", "U+110000"),
]
# The seven references the pre-fix binary decoded into a bare control character.
OLD_ACCEPTED = ["&#1;", "&#8;", "&#11;", "&#12;", "&#31;", "&#xFFFE;", "&#xFFFF;"]
# resvg rejects these nine; it is lenient on the three surrogate/overflow ones.
RENDER_REJECTED = ["&#0;", "&#x0;", "&#1;", "&#8;", "&#11;", "&#12;", "&#31;", "&#xFFFE;", "&#xFFFF;"]

PLAIN = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">'
    '<text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text></svg>'
)
PLAIN_ATTR = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">'
    '<text x="10" y="50" font-size="20" font-family="Arial" data-x="{BODY}">x</text></svg>'
)
PANEL = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">'
    '<rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>'
    '<text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text></svg>'
)
PANEL_ATTR = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">'
    '<rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>'
    '<text x="10" y="50" font-size="20" font-family="Arial" data-x="{BODY}">x</text></svg>'
)

passed = 0
failed = 0
skipped = 0


def check(ok, label, detail=""):
    global passed, failed
    if ok:
        passed += 1
        print(f"PASS  {label}")
    else:
        failed += 1
        print(f"FAIL  {label}")
        if detail:
            for line in str(detail).splitlines():
                print(f"        {line}")


def skip(label, why):
    global skipped
    skipped += 1
    print(f"SKIP  {label}  ({why})")


def is_xml_char(c):
    return (
        c in "\t\n\r"
        or "\u0020" <= c <= "\ud7ff"
        or "\ue000" <= c <= "\ufffd"
        or "\U00010000" <= c <= "\U0010ffff"
    )


def illegal_chars(data):
    text = data.decode("utf-8", "replace")
    return [(c, i) for i, c in enumerate(text) if not is_xml_char(c)]


def wellformed(data):
    try:
        ET.fromstring(data)
        return True
    except ET.ParseError:
        return False


def run(binary, args):
    return subprocess.run([str(binary)] + args, capture_output=True)


def convert(binary, inp, out):
    if out.exists():
        out.unlink()
    result = run(binary, ["convert", "--input", str(inp), "--output", str(out)])
    data = out.read_bytes() if out.exists() else None
    return result, data


def plot_grid(binary, inp, out):
    if out.exists():
        out.unlink()
    result = run(binary, ["plot-grid", "--input", str(inp), "--output", str(out)])
    data = out.read_bytes() if out.exists() else None
    return result, data


def write(directory, name, template, body):
    path = directory / name
    path.write_text(template.replace("{BODY}", body), encoding="utf-8")
    return path


def render(inp, out):
    if out.exists():
        out.unlink()
    env = dict(os.environ)
    if FONT_DIR.is_dir():
        env["D1_FONT_DIR"] = str(FONT_DIR)
    return subprocess.run([str(RENDER), str(inp), str(out), "1"], capture_output=True, env=env).returncode


def main():
    if not BIN.is_file():
        print(f"FAIL  binary under test does not exist: {BIN}")
        return 1
    print(f"binary under test: {BIN}")
    if OLD_BIN:
        print(f"pre-fix binary:    {OLD_BIN}")
    if RENDER.is_file():
        print(f"reference renderer: {RENDER}")
    else:
        print("reference renderer: missing (family E skipped)")

    with tempfile.TemporaryDirectory(prefix="r50-") as tmp:
        tmp = pathlib.Path(tmp)

        # ── A: the legal surface stays accepted and well-formed ────────────────
        for index, ref in enumerate(LEGAL):
            for kind, template in (("text", PLAIN), ("attr", PLAIN_ATTR)):
                inp = write(tmp, f"a_{kind}_{index}.svg", template, f"a{ref}b")
                for command, runner in (("convert", convert), ("plot-grid", plot_grid)):
                    out = tmp / f"a_{kind}_{index}_{command}.svg"
                    result, data = runner(BIN, inp, out)
                    label = f"A/{kind}/{command} {ref}"
                    if result.returncode != 0:
                        check(False, label, f"rc={result.returncode} stderr={result.stderr.decode('utf-8', 'replace')}")
                        continue
                    if data is None:
                        check(False, label, "rc=0 but no output")
                        continue
                    bad = illegal_chars(data)
                    check(not bad and wellformed(data), label,
                          f"illegal={bad[:3]} wellformed={wellformed(data)}")

        # ── B: the illegal surface fails closed on both paths ──────────────────
        for index, (ref, codepoint) in enumerate(ILLEGAL):
            for kind, template in (("text", PLAIN), ("attr", PLAIN_ATTR)):
                inp = write(tmp, f"b_{kind}_{index}.svg", template, f"a{ref}b")
                for command, runner in (("convert", convert), ("plot-grid", plot_grid)):
                    out = tmp / f"b_{kind}_{index}_{command}.svg"
                    result, data = runner(BIN, inp, out)
                    stderr = result.stderr.decode("utf-8", "replace")
                    label = f"B/{kind}/{command} {ref}"
                    ok = (
                        result.returncode == 1
                        and data is None
                        and ref in stderr
                        and codepoint in stderr
                    )
                    check(ok, label,
                          f"rc={result.returncode} output={data is not None} "
                          f"ref={ref in stderr} codepoint={codepoint in stderr} stderr={stderr.strip()}")

        # ── C: the strong invariant over a mixed batch ─────────────────────────
        text_cases = [
            ("plain", "hello", True),
            ("predefined", "&amp;&lt;&gt;&quot;&apos;", True),
            ("decimal", "37&#176;C", True),
            ("hex", "&#x1F600;", True),
            ("cdata", "<![CDATA[37&#176;C]]>", True),
            ("tab", "a&#9;b", True),
            ("replacement", "a&#xFFFD;b", True),
            ("literal_control", "a\x01b", False),
            ("illegal_1", "a&#1;b", False),
            ("illegal_fffe", "a&#xFFFE;b", False),
            ("illegal_0", "a&#0;b", False),
            ("illegal_surrogate", "a&#55296;b", False),
            ("illegal_overflow", "a&#x110000;b", False),
        ]
        attr_cases = [case for case in text_cases if case[0] != "cdata"]
        accepted = 0
        rejected = 0
        for kind, template, cases in (
            ("text", PLAIN, text_cases),
            ("attr", PLAIN_ATTR, attr_cases),
        ):
            for index, (name, body, must_accept) in enumerate(cases):
                inp = write(tmp, f"c_{kind}_{index}.svg", template, body)
                for command, runner in (("convert", convert), ("plot-grid", plot_grid)):
                    out = tmp / f"c_{kind}_{index}_{command}.svg"
                    result, data = runner(BIN, inp, out)
                    label = f"C/{kind}/{command} {name}"
                    if result.returncode == 0:
                        accepted += 1
                        ok = data is not None and not illegal_chars(data) and wellformed(data)
                        check(ok, label,
                              f"rc=0 output={data is not None} "
                              f"illegal={illegal_chars(data)[:3] if data else None} "
                              f"wellformed={wellformed(data) if data else None}")
                    else:
                        rejected += 1
                        check(result.returncode == 1 and data is None, label,
                              f"rc={result.returncode} output={data is not None}")
                    if must_accept:
                        check(result.returncode == 0, f"C/{kind}/{command} {name} must be accepted",
                              f"rc={result.returncode}")
                    else:
                        check(result.returncode == 1, f"C/{kind}/{command} {name} must be rejected",
                              f"rc={result.returncode}")
        check(accepted > 0 and rejected > 0, "C/batch has both accepted and rejected cases",
              f"accepted={accepted} rejected={rejected}")

        # ── D: the pre-fix binary must be red (the assertions have teeth) ──────
        if OLD_BIN is None:
            skip("D/pre-fix binary", "SVG_GRID_OLD_BIN is not set")
        elif not OLD_BIN.is_file():
            check(False, "D/pre-fix binary exists", f"not found: {OLD_BIN}")
        else:
            for index, ref in enumerate(OLD_ACCEPTED):
                for kind, template in (("text", PLAIN), ("attr", PLAIN_ATTR)):
                    inp = write(tmp, f"d_{kind}_{index}.svg", template, f"a{ref}b")
                    out = tmp / f"d_{kind}_{index}.svg.out"
                    result, data = convert(OLD_BIN, inp, out)
                    label = f"D/{kind} pre-fix accepts {ref} but the product is not well-formed"
                    check(
                        result.returncode == 0
                        and data is not None
                        and (illegal_chars(data) or not wellformed(data)),
                        label,
                        f"rc={result.returncode} output={data is not None} "
                        f"illegal={illegal_chars(data)[:3] if data else None} "
                        f"wellformed={wellformed(data) if data else None}",
                    )
            for index, ref in enumerate(LEGAL):
                inp = write(tmp, f"d_legal_{index}.svg", PLAIN, f"a{ref}b")
                out = tmp / f"d_legal_{index}.svg.out"
                result, data = convert(OLD_BIN, inp, out)
                check(
                    result.returncode == 0 and data is not None and wellformed(data),
                    f"D/text pre-fix stays green on legal {ref}",
                    f"rc={result.returncode} wellformed={wellformed(data) if data else None}",
                )

        # ── E: the reference renderer on the ORIGINAL inputs ───────────────────
        if not RENDER.is_file():
            skip("E/reference renderer", f"not found: {RENDER}")
        else:
            for index, ref in enumerate(LEGAL):
                inp = write(tmp, f"e_legal_{index}.svg", PLAIN, f"a{ref}b")
                code = render(inp, tmp / f"e_legal_{index}.png")
                check(code == 0, f"E/renderer accepts legal {ref}", f"rc={code}")
            for index, ref in enumerate(RENDER_REJECTED):
                inp = write(tmp, f"e_bad_{index}.svg", PLAIN, f"a{ref}b")
                code = render(inp, tmp / f"e_bad_{index}.png")
                check(code == 3, f"E/renderer rejects illegal {ref}", f"rc={code}")
            for ref in ("&#55296;", "&#xDFFF;", "&#x110000;"):
                skip(f"E/renderer {ref}", "resvg is lenient on surrogates/overflow; cairosvg rejects")

    print(f"\n== {passed} passed, {failed} failed, {skipped} skipped ==")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
