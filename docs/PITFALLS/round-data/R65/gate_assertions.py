#!/usr/bin/env python3
"""R65 gate: a `#[test]` fn must *check* something, and `#[ignore]` is forbidden.

Why (R19's hole): the R19 gate pins only the exact per-target test *counts*, so a
body replaced by `{}` keeps the count and stays green. This script reads the
source, finds every `#[test]`, extracts that function's body by brace matching
(strings, raw strings, char literals, comments and lifetimes skipped), and
requires an assertion-style call inside it. Nothing is counted with a regex over
the whole file: the verdict is per *function body*, so the message names the file
and the test.

Assertion-style =
  * the macros `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert*`,
    `panic!`, `unreachable!`;
  * `.expect(`, `.unwrap(`, `.unwrap_err(` — a panic on the wrong value is a
    check in Rust, and the crate's helpers use it to state a positive case;
  * a call to a function defined in the scanned tree whose own body contains an
    assertion macro — **one hop only**, so `usage_error(&args)` and
    `assert_verdict(...)` count through their `assert_eq!`/`panic!`, while
    `let _ = fit(5, Some(2), None);` does not pass just because `fit` reaches an
    `assert!` three calls down.

`#[ignore]` on a test is refused outright: a silently skipped test is exactly the
same hole as a gutted one (currently 0 in the tree).

Usage:  python3 docs/PITFALLS/round-data/R65/gate_assertions.py [ROOT]
Exit:   0 every `#[test]` checks something and no `#[ignore]`, 1 at least one
        problem (each printed as `FAIL <file>:<test>`), 2 no source tree found.
"""

import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE.parents[3]
SOURCES = ("src", "tests")
SKIP_DIRS = {"target", ".git"}

ASSERT_MACRO = re.compile(
    r"\b(?:assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne"
    r"|panic|unreachable)\s*!"
)
PANIC_CALL = re.compile(r"\.(?:expect|unwrap|unwrap_err)\s*\(")
CALL = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
TEST_ATTR = re.compile(r"^[ \t]*#\[test\][ \t]*$", re.M)
IGNORE_ATTR = re.compile(r"^[ \t]*#\[ignore\b[^\]]*\]", re.M)
FN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


def skip_literal(text, index):
    """`index` points at a quote (or the `r`/`b` of a raw/byte string)."""
    if text[index] in "rb":
        j = index + 1
        if j < len(text) and text[j] == '"':
            return skip_quoted(text, j)
        hashes = 0
        while j < len(text) and text[j] == "#":
            hashes += 1
            j += 1
        if j < len(text) and text[j] == '"':
            closing = '"' + "#" * hashes
            end = text.find(closing, j + 1)
            return len(text) if end < 0 else end + len(closing)
        return index + 1
    return skip_quoted(text, index)


def skip_quoted(text, index):
    quote = text[index]
    j = index + 1
    while j < len(text):
        if text[j] == "\\":
            j += 2
            continue
        if text[j] == quote:
            return j + 1
        j += 1
    return len(text)


def body_of(text, open_brace):
    """Return the body between `open_brace` and its matching `}`."""
    depth = 0
    index = open_brace
    while index < len(text):
        char = text[index]
        if char in "rb\"" and (char != "r" or index + 1 >= len(text)
                               or text[index + 1] in '"#'):
            after = skip_literal(text, index)
            if after > index + 1:
                index = after
                continue
        if char == "'":
            # A char literal (`'x'`, `'\n'`) is three-plus bytes; a lifetime (`'a`)
            # is not a string and must not swallow code.
            match = re.match(r"'(\\.|[^\\'])'", text[index:])
            if match:
                index += match.end()
                continue
        if text.startswith("//", index):
            newline = text.find("\n", index)
            index = len(text) if newline < 0 else newline
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index)
            index = len(text) if end < 0 else end + 2
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace + 1:index]
        index += 1
    return text[open_brace + 1:]


def assertions(text):
    return bool(ASSERT_MACRO.search(text) or PANIC_CALL.search(text))


def asserts_macro(text):
    """A helper only counts as an assertion when it *asserts*, not merely unwraps.

    Otherwise `let _ = fit(5, Some(2), None);` would pass through `fit`'s internal
    `.unwrap()` — the exact "assertions deleted, call kept" hole this gate exists
    to close.
    """
    return bool(ASSERT_MACRO.search(text))


def rust_files():
    files = []
    for name in SOURCES:
        root = ROOT / name
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*.rs")):
            if SKIP_DIRS & set(path.parts):
                continue
            files.append(path)
    return files


def function_bodies(files):
    """Map `fn name` -> body for every function in the tree (helper resolution)."""
    bodies = {}
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in FN.finditer(text):
            brace = text.find("{", match.end())
            if brace < 0:
                continue
            # A declaration (`fn f();`) or a call site is not a definition: the body
            # must start before any `;`.
            if ";" in text[match.end():brace]:
                continue
            bodies.setdefault(match.group(1), []).append(body_of(text, brace))
    return bodies


def checks(body, bodies, top=True):
    """True when `body` asserts directly or calls a helper that asserts directly.

    The *test body itself* may state its check with `.expect(...)`/`.unwrap(...)`
    (a panic on the wrong value is a check in Rust). A helper is followed only
    when its own body contains an assertion *macro* — one hop, not transitively:
    `let _ = fit(5, Some(2), None);` must not pass just because `fit` eventually
    reaches an `assert!` three calls down. That is the "assertions deleted, call
    kept" hole this gate exists to close.
    """
    if top and assertions(body):
        return True
    if asserts_macro(body):
        return True
    for name in CALL.findall(body):
        for helper in bodies.get(name, []):
            if asserts_macro(helper):
                return True
    return False


def main():
    files = rust_files()
    if not files:
        print(f"FAIL  no Rust sources under {ROOT}/{{src,tests}}")
        return 2
    bodies = function_bodies(files)
    total = 0
    problems = []
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        relative = path.relative_to(ROOT)
        for match in TEST_ATTR.finditer(text):
            total += 1
            after = text[match.end():]
            fn = FN.search(after)
            if not fn:
                problems.append(f"{relative}:<unknown> — `#[test]` not followed by `fn`")
                continue
            name = fn.group(1)
            brace = after.find("{", fn.end())
            if brace < 0 or ";" in after[fn.end():brace]:
                problems.append(f"{relative}:{name} — `#[test]` without a body")
                continue
            body = body_of(after, brace)
            if not body.strip():
                problems.append(f"{relative}:{name} — the body is empty")
            elif not checks(body, bodies):
                problems.append(f"{relative}:{name} — no assertion in the body")
        for ignore in IGNORE_ATTR.finditer(text):
            line = text.count("\n", 0, ignore.start()) + 1
            fn = FN.search(text[ignore.end():])
            name = fn.group(1) if fn else "<unknown>"
            problems.append(f"{relative}:{name} (line {line}) — `#[ignore]` is forbidden")

    for problem in problems:
        print(f"FAIL  {problem}")
    if problems:
        print(f"\n{len(problems)} problem(s) over {total} `#[test]` function(s)")
        return 1
    print(f"PASS  {total} `#[test]` function(s) all assert something; 0 `#[ignore]`")
    return 0


if __name__ == "__main__":
    sys.exit(main())
