//! R52: unresolved-entity diagnostics.
//!
//! `quick_xml` reports escape errors with byte ranges that index the `&name;`
//! fragment it parses internally, never the document (`&nbsp;` at column 12 came
//! out as `at 1..5`). Both spellings it emits — the `at N..M: ` prefix and the
//! infix ` at range N..M` — are stripped by one shared helper in `src/dom.rs`;
//! these tests pin the exact stderr of both paths, and pin that the reason and
//! the entity name survive.

use super::*;

// ---------------------------------------------------------------------------
// R52: an unresolved entity used to print `at 1..5: ` — the byte range of the
// `&nbsp;` fragment `quick_xml` synthesizes internally, not a document position.
// The message must quote the entity and the reason verbatim, and carry no range.
// ---------------------------------------------------------------------------

#[test]
fn convert_text_undefined_entity_message_is_exact() {
    let input = write("exact_nbsp.svg", PLAIN, "a&nbsp;b");
    let output = tmp("exact_nbsp.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(
        stderr.trim_end(),
        "svg_grid convert: XML error: reference &nbsp; cannot be resolved: unrecognized entity `nbsp`"
    );
    assert!(!has_synthetic_range(&stderr), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

#[test]
fn convert_attribute_undefined_entity_message_is_exact() {
    let input = write("exact_attr_foo.svg", PLAIN_ATTR, "a&foo;b");
    let output = tmp("exact_attr_foo.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(
        stderr.trim_end(),
        "svg_grid convert: XML error: attribute `data-x`: reference &foo; cannot be resolved: unrecognized entity `foo`"
    );
    assert!(!has_synthetic_range(&stderr), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

/// The same synthetic offset appears in `quick_xml`'s unterminated-entity
/// message (`... at range 1..6: Cannot find ';' after '&'`), reached only through
/// an attribute value; it must be gone and the reason must survive verbatim.
#[test]
fn convert_attribute_unterminated_entity_message_is_exact() {
    let input = write("exact_attr_open.svg", PLAIN_ATTR, "a&nbsp");
    let output = tmp("exact_attr_open.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(
        stderr.trim_end(),
        "svg_grid convert: XML error: attribute `data-x`: Error while escaping character: Cannot find ';' after '&'"
    );
    assert!(!has_synthetic_range(&stderr), "{stderr}");
    assert!(!stderr.contains("at range "), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

/// Counterexample: stripping the range must not turn an unresolved name into an
/// accepted one. Another undefined name still fails and still names itself.
#[test]
fn convert_other_undefined_named_entity_still_fails_exactly() {
    let input = write("exact_bogus.svg", PLAIN, "a&bogus;b");
    let output = tmp("exact_bogus.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(
        stderr.trim_end(),
        "svg_grid convert: XML error: reference &bogus; cannot be resolved: unrecognized entity `bogus`"
    );
    assert!(!has_synthetic_range(&stderr), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}
