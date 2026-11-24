#!/usr/bin/env python3
"""R52 evidence: diagnostics and ledger hygiene.

R50 left five review items: (P2-1) R50's metadata claimed 220 corpus files and a
stale base; (P2-2) `modify R47 --by R50` rewrote four R47 line numbers using the
`1fbc3c3` coordinate system instead of R47's own anchor `8aca0d1`; (P3-1) an
unresolved named entity still printed `quick_xml`'s synthetic byte range
(`at 1..5: `), which indexes the `&name;` fragment it parses, never the document;
(P3-2) R50's own findings were off by 1-2 lines.  R52 strips the synthetic range
through one shared helper, appends ledger findings instead of rewriting history,
and moves `normalize_label` to `src/label_letters.rs` to stay under the 20000 B
file red line.

Families
  A  text path: `<text>a&nbsp;b</text>` -> rc=1, exact stderr, no `at N..M`.
  B  attribute path: `data-x="a&foo;b"` and the unterminated `data-x="a&nbsp"`
     -> same shape with the attribute prefix, no synthetic range in any spelling.
  C  R50 contract unchanged: the illegal-codepoint messages (which never carried
     a range) stay verbatim on both paths.
  D  (SVG_GRID_OLD_BIN) the pre-R52 binary must be RED on A/B: its message has
     the synthetic range.  At least three checks must flip.  Skipped when unset.
  E  R50/meta.toml matches the measured corpus: 214 `tests/**/*.svg` (217 in the
     whole repo), base `/tmp/r50base ... @e0c6033`, and no stale 220/r46base.
  F  `normalize_label` still answers through `svg_grid::normalize_label` after
     the move: ` A ` collides with `A` (rc=2, stderr names "A"), while
     `A` / `A&#160;` stay distinct (rc=0).

Env
  SVG_GRID_BIN      binary under test (default: target/debug/svg_grid)
  SVG_GRID_OLD_BIN  pre-fix binary; supplies family D (skipped when unset)
"""

import os
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[4]
BIN = pathlib.Path(os.environ.get("SVG_GRID_BIN") or ROOT / "target" / "debug" / "svg_grid")
OLD_BIN = pathlib.Path(os.environ["SVG_GRID_OLD_BIN"]) if os.environ.get("SVG_GRID_OLD_BIN") else None

RANGE = re.compile(r"at \d+\.\.\d+")

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
LEAF_PANEL = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">'
    '<rect data-panel-box="main" x="0" y="0" width="100" height="80" fill="none"/>'
    '<text data-svg-grid-label="0" x="4" y="14" font-size="12" font-family="Arial">{BODY}</text></svg>'
)

EXPECTED = {
    "text": "svg_grid convert: XML error: reference &nbsp; cannot be resolved: unrecognized entity `nbsp`",
    "attr": (
        "svg_grid convert: XML error: attribute `data-x`: reference &foo; cannot be resolved: "
        "unrecognized entity `foo`"
    ),
    "unterminated_attr": (
        "svg_grid convert: XML error: attribute `data-x`: "
        "Error while escaping character: Cannot find ';' after '&'"
    ),
    "bogus": "svg_grid convert: XML error: reference &bogus; cannot be resolved: unrecognized entity `bogus`",
    "plot": "svg_grid: SVG XML error: reference &nbsp; cannot be resolved: unrecognized entity `nbsp`",
    "illegal_text": (
        "svg_grid convert: XML error: character reference &#1; decodes to U+0001, "
        "which is not a legal XML 1.0 character"
    ),
    "illegal_attr": (
        "svg_grid convert: XML error: attribute `data-x`: character reference &#1; "
        "decodes to U+0001, which is not a legal XML 1.0 character"
    ),
}

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


def write(directory, name, template, body):
    path = directory / name
    path.write_text(template.replace("{BODY}", body), encoding="utf-8")
    return path


def convert(binary, inp, out):
    if out.exists():
        out.unlink()
    result = subprocess.run(
        [str(binary), "convert", "--input", str(inp), "--output", str(out)],
        capture_output=True,
    )
    return result, out.exists()


def plot_grid(binary, inputs, out, extra=()):
    if out.exists():
        out.unlink()
    args = [str(binary), "plot-grid"]
    for inp in inputs:
        args += ["--input", str(inp)]
    args += ["--output", str(out)]
    args += list(extra)
    result = subprocess.run(args, capture_output=True)
    return result, out.exists()


def stderr_of(result):
    return result.stderr.decode("utf-8", "replace").rstrip("\n")


def corpus(pattern):
    """Count files tracked by git: the corpus must not include ignored scratch
    SVGs that `doctor`/earlier runs leave in the working tree (they once made R50
    count 220 while only 214 tests files are tracked)."""
    result = subprocess.run(
        ["git", "ls-files", pattern], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return len(result.stdout.splitlines())


def message_case(tmp, binary, label, template, body, expected):
    inp = write(tmp, f"{label}.svg", template, body)
    out = tmp / f"{label}.out.svg"
    result, wrote = convert(binary, inp, out)
    err = stderr_of(result)
    check(
        result.returncode == 1
        and not wrote
        and err == expected
        and not RANGE.search(err)
        and "at range " not in err,
        label,
        f"rc={result.returncode} wrote={wrote} stderr={err!r}",
    )
    return err


def main():
    if not BIN.is_file():
        print(f"FAIL  binary under test does not exist: {BIN}")
        return 1
    print(f"binary under test: {BIN}")
    if OLD_BIN:
        print(f"pre-fix binary:    {OLD_BIN}")

    with tempfile.TemporaryDirectory(prefix="r52-") as tmp:
        tmp = pathlib.Path(tmp)

        # ── A: text path ──────────────────────────────────────────────────────
        message_case(tmp, BIN, "a_text", PLAIN, "a&nbsp;b", EXPECTED["text"])
        message_case(tmp, BIN, "a_other_name", PLAIN, "a&bogus;b", EXPECTED["bogus"])
        # plot-grid parses a panel SVG with its own error prefix.
        inp = write(tmp, "a_plot.svg", PANEL, "a&nbsp;b")
        out = tmp / "a_plot.out.svg"
        result, wrote = plot_grid(BIN, [inp], out)
        err = stderr_of(result)
        check(
            result.returncode == 1
            and not wrote
            and err == EXPECTED["plot"]
            and not RANGE.search(err),
            "A/plot-grid text message is exact",
            f"rc={result.returncode} wrote={wrote} stderr={err!r}",
        )

        # ── B: attribute path ─────────────────────────────────────────────────
        message_case(tmp, BIN, "b_attr", PLAIN_ATTR, "a&foo;b", EXPECTED["attr"])
        message_case(
            tmp, BIN, "b_unterminated", PLAIN_ATTR, "a&nbsp", EXPECTED["unterminated_attr"]
        )

        # ── C: the R50 messages keep their exact bytes ────────────────────────
        message_case(tmp, BIN, "c_illegal_text", PLAIN, "a&#1;b", EXPECTED["illegal_text"])
        message_case(tmp, BIN, "c_illegal_attr", PLAIN_ATTR, "a&#1;b", EXPECTED["illegal_attr"])

        # ── D: the pre-R52 binary must be red ─────────────────────────────────
        if OLD_BIN is None:
            skip("D/pre-fix binary", "SVG_GRID_OLD_BIN is unset")
        elif not OLD_BIN.is_file():
            check(False, "D/pre-fix binary exists", str(OLD_BIN))
        else:
            for label, template, body, expected in (
                ("D/text", PLAIN, "a&nbsp;b", EXPECTED["text"]),
                ("D/attr", PLAIN_ATTR, "a&foo;b", EXPECTED["attr"]),
                ("D/bogus", PLAIN, "a&bogus;b", EXPECTED["bogus"]),
            ):
                inp = write(tmp, f"{label.replace('/', '_')}.svg", template, body)
                out = tmp / f"{label.replace('/', '_')}.out.svg"
                result, wrote = convert(OLD_BIN, inp, out)
                err = stderr_of(result)
                check(
                    result.returncode == 1 and not wrote and err != expected and bool(RANGE.search(err)),
                    f"{label} pre-fix is red (synthetic range)",
                    f"rc={result.returncode} wrote={wrote} stderr={err!r}",
                )

        # ── E: R50 metadata matches the anchors it cites ──────────────────────
        meta_path = ROOT / "docs" / "PITFALLS" / "round-data" / "R50" / "meta.toml"
        meta = meta_path.read_text(encoding="utf-8")
        tests_svgs = corpus("tests/**/*.svg")
        repo_svgs = corpus("*.svg")
        check(tests_svgs == 214, "E/tests corpus is 214 svg", f"measured {tests_svgs}")
        check(repo_svgs == 217, "E/whole repo is 217 svg", f"measured {repo_svgs}")
        check(
            "214 个 `tests/**/*.svg`" in meta,
            "E/meta says 214",
            "missing `214 个 `tests/**/*.svg``",
        )
        check("220 个" not in meta, "E/meta has no stale 220", "220 still present")
        check(
            "/tmp/r50base/target/release/svg_grid @e0c6033" in meta,
            "E/meta names the base really used",
            "missing /tmp/r50base @e0c6033",
        )
        check("r46base" not in meta, "E/meta has no stale r46base", "r46base still present")

        # ── F: normalize_label is still reachable through the CLI ─────────────
        plain = write(tmp, "f_plain.svg", LEAF_PANEL, "A")
        padded = write(tmp, "f_padded.svg", LEAF_PANEL, " A ")
        result, _ = plot_grid(BIN, [plain, padded], tmp / "f_dup.svg", ["--width", "400", "--ncol", "2"])
        err = stderr_of(result)
        check(
            result.returncode == 2 and 'duplicate panel letters: "A"' in err,
            "F/` A ` normalizes to `A`",
            f"rc={result.returncode} stderr={err!r}",
        )
        nbsp = write(tmp, "f_nbsp.svg", LEAF_PANEL, "A&#160;")
        result, wrote = plot_grid(BIN, [plain, nbsp], tmp / "f_nbsp_out.svg", ["--width", "400", "--ncol", "2"])
        err = stderr_of(result)
        check(
            result.returncode == 0 and wrote and "duplicate panel letters" not in err,
            "F/U+00A0 stays significant",
            f"rc={result.returncode} wrote={wrote} stderr={err!r}",
        )

        # ── G: no source file may cross the 20000 B red line ──────────────────
        for path in sorted((ROOT / "src").rglob("*.rs")):
            size = path.stat().st_size
            check(size <= 20000, f"G/{path.relative_to(ROOT)} <= 20000 B", f"{size} B")

    print(f"\n== {passed} passed, {failed} failed, {skipped} skipped ==")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
