#!/usr/bin/env python3
"""R62b — a definition's `href` edge follows its own content domain.

Follow-up to R62: the first R62 cut (7876c46) copied definitions per content
domain but never switched definition-to-definition `href` edges; cc66d4b fixed
container data-scale inheritance without touching href. This round's evidence
is the three href cases (pattern chain / objectBoundingBox gradient / three
layers) plus their control.

The heavy lifting (PNG decode, renderer, composer) lives in ../R62/test.py and
is imported rather than duplicated; that file is committed next to this one.

rc=0 = all pass. Missing binary/renderer => rc=1 (fail-closed).
"""
import importlib.util
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
R62 = HERE.parent / "R62" / "test.py"


def load_evidence():
    spec = importlib.util.spec_from_file_location("r62ev", R62)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    if not R62.is_file():
        print("FAIL: missing %s (the R62 evidence this round extends)" % R62)
        return 1
    ev = load_evidence()
    failures = 0
    total = 0

    def check(ok, label, detail=""):
        nonlocal failures, total
        total += 1
        print(("PASS " if ok else "FAIL ") + label + ((" — " + detail) if detail and not ok else ""))
        if not ok:
            failures += 1

    if not ev.BIN.is_file():
        print("FAIL: svg_grid binary not found at %s (set SVG_GRID_BIN)" % ev.BIN)
        return 1
    if not ev.RENDER.is_file():
        print("FAIL: renderer not found at %s (set SVG_GRID_RENDER_BIN)" % ev.RENDER)
        return 1

    print("== href out-edge follows the referrer's own content domain ==")
    cases = [
        ("e1_pattern_href", ev.CASE_E1,
         lambda s: ev.widths(s, "pattern", "width") == ["10", "20", "40"]
         and "q--svg-grid-verbatim" in ev.hrefs(s)),
        ("e3b_gradient_href", ev.CASE_E3B,
         lambda s: ev.widths(s, "linearGradient", "x2") == ["0.5", "1"]
         and "g--svg-grid-verbatim" in ev.hrefs(s)),
        ("e4_three_layers", ev.CASE_E4,
         lambda s: ev.widths(s, "pattern", "width") == ["16", "4", "40", "8", "8"]
         and ev.hrefs(s) == ["q--svg-grid-verbatim", "r", "r--svg-grid-verbatim"]),
    ]
    for name, svg, policy in cases:
        text, diff = ev.pixel_case("r62b-" + name, svg, ev.BIN)
        check(text is not None and diff == (0, 0), name + " pixel-matches the source at s=2",
              "xor=%s" % (diff,))
        check(text is not None and policy(text), name + " switches the href to the verbatim copy")

    print("== control: a definition referenced only from a verbatim body ==")
    text, diff = ev.pixel_case("r62b-e2", ev.CASE_E2, ev.BIN)
    check(text is not None and diff == (0, 0), "e2_only_verbatim control still pixel-clean",
          "xor=%s" % (diff,))
    check(text is not None and "g--svg-grid-verbatim" in ev.hrefs(text),
          "e2_only_verbatim control still points at the verbatim gradient")

    print("== teeth: the pre-fix build (cc66d4b) must be red on the same cases ==")
    prev = Path(os.environ.get("SVG_GRID_PREV_BIN", "/tmp/r62_cc66_bin"))
    if not prev.is_file():
        print("SKIP teeth: %s is missing (set SVG_GRID_PREV_BIN)" % prev)
    else:
        for name, svg, _ in cases:
            text, diff = ev.pixel_case("r62b-tooth-" + name, svg, prev)
            check(text is None or diff != (0, 0), "tooth %s is red before the fix" % name,
                  "xor=%s" % (diff,))

    print("\n%d checked, %d failed" % (total, failures))
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
