//! Text-node entity decoding (R47).
//!
//! `quick_xml` hands a text-node reference over as `Event::GeneralRef` without its
//! `&`/`;`. Both DOMs used to resolve only the five predefined names and echo
//! everything else verbatim, so `&#176;` became the literal `&#176;` and the
//! serializer escaped the `&` again ⇒ `&amp;#176;`, which renders as visible text.
//!
//! These tests drive the real binary on both paths (`plot-grid` and `convert`) and
//! assert the decoded text node character for character, plus the fail-closed
//! behaviour for an undefined name and the untouched CDATA literal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use quick_xml::events::Event;
use quick_xml::Reader;

const PANEL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>
  <text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text>
</svg>"#;

const PLAIN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <text x="10" y="50" font-size="20" font-family="Arial">{BODY}</text>
</svg>"#;

fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).unwrap();
    dir.join(format!("textref_{name}_{}", std::process::id()))
}

fn write(name: &str, template: &str, body: &str) -> PathBuf {
    let path = tmp(name);
    fs::write(&path, template.replace("{BODY}", body)).unwrap();
    path
}

fn plot_grid(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_svg_grid"))
        .args([
            "plot-grid",
            "--width",
            "400",
            "--ncol",
            "1",
            "--plot-margin",
            "0",
            "--input",
        ])
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn convert(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_svg_grid"))
        .arg("convert")
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

/// The character data of every `<text>` element, in document order.
fn text_nodes(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .map(|chunk| {
            let start = chunk.find('>').expect("text start tag") + 1;
            let end = chunk[start..].find("</text>").expect("text end tag") + start;
            chunk[start..end].to_string()
        })
        .collect()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn plot_grid_decodes_character_references_in_text() {
    let input = write("plot_grid.svg", PANEL, "37&#176;C &#215;2 &#177;1");
    let output = tmp("plot_grid.out.svg");
    let result = plot_grid(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    assert_eq!(text_nodes(&read(&output)), ["37°C ×2 ±1"]);
}

#[test]
fn convert_decodes_character_references_in_text() {
    let input = write("convert.svg", PLAIN, "37&#176;C &#215;2 &#177;1");
    let output = tmp("convert.out.svg");
    let result = convert(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    assert_eq!(text_nodes(&read(&output)), ["37°C ×2 ±1"]);
}

#[test]
fn plot_grid_rejects_an_undefined_named_entity_in_text() {
    let input = write("plot_grid_nbsp.svg", PANEL, "a&nbsp;b");
    let output = tmp("plot_grid_nbsp.out.svg");
    let _ = fs::remove_file(&output);
    let result = plot_grid(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("unrecognized entity"), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

#[test]
fn convert_rejects_an_undefined_named_entity_in_text() {
    let input = write("convert_nbsp.svg", PLAIN, "a&nbsp;b");
    let output = tmp("convert_nbsp.out.svg");
    let _ = fs::remove_file(&output);
    let result = convert(&input, &output);
    assert_eq!(result.status.code(), Some(1), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("unrecognized entity"), "{stderr}");
    assert!(!output.exists(), "no output may be written on failure");
}

#[test]
fn cdata_content_stays_a_literal() {
    let input = write("cdata.svg", PANEL, "<![CDATA[37&#176;C]]>");
    let output = tmp("cdata.out.svg");
    let result = plot_grid(&input, &output);
    assert!(result.status.success(), "{:?}", result);
    // CDATA is character data, not markup: the `&` is escaped on the way out and
    // the renderer draws the same literal it drew before R47.
    assert_eq!(text_nodes(&read(&output)), ["37&amp;#176;C"]);
}

// ---------------------------------------------------------------------------
// R50: the XML 1.0 `Char` production. A reference that decodes to a character
// XML 1.0 forbids (`&#1;`, `&#xFFFE;`, …) must fail closed instead of writing a
// bare control character into the product; the same predicate guards attributes.
// ---------------------------------------------------------------------------

const LEGAL_REFS: &[&str] = &[
    "&#9;",
    "&#10;",
    "&#13;",
    "&#32;",
    "&#65;",
    "&#x1F600;",
    "&#xFFFD;",
    "&#x10FFFF;",
    "&#xE000;",
    "&#xD7FF;",
];

const ILLEGAL_REFS: &[(&str, &str)] = &[
    ("&#0;", "U+0000"),
    ("&#x0;", "U+0000"),
    ("&#1;", "U+0001"),
    ("&#8;", "U+0008"),
    ("&#11;", "U+000B"),
    ("&#12;", "U+000C"),
    ("&#31;", "U+001F"),
    ("&#xFFFE;", "U+FFFE"),
    ("&#xFFFF;", "U+FFFF"),
    ("&#55296;", "U+D800"),
    ("&#xDFFF;", "U+DFFF"),
    ("&#x110000;", "U+110000"),
];

const PLAIN_ATTR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <text x="10" y="50" font-size="20" font-family="Arial" data-x="{BODY}">x</text>
</svg>"#;

const PANEL_ATTR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="80">
  <rect data-panel-box="main" x="0" y="0" width="400" height="80" fill="none"/>
  <text x="10" y="50" font-size="20" font-family="Arial" data-x="{BODY}">x</text>
</svg>"#;

/// The XML 1.0 `Char` production, duplicated here as an independent oracle.
fn is_xml_char(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&c)
        || ('\u{E000}'..='\u{FFFD}').contains(&c)
        || ('\u{10000}'..='\u{10FFFF}').contains(&c)
}

/// The strong R50 invariant: an output that was written must be well-formed XML
/// 1.0 — it re-parses with `quick_xml` and contains no illegal character.
fn assert_well_formed(path: &Path) {
    let bytes = fs::read(path).unwrap();
    let text = std::str::from_utf8(&bytes).expect("output is UTF-8");
    for c in text.chars() {
        assert!(
            is_xml_char(c),
            "{} contains illegal XML 1.0 character {c:?} (U+{:04X})",
            path.display(),
            c as u32
        );
    }
    let mut reader = Reader::from_str(text);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => panic!("{} does not re-parse: {error}", path.display()),
        }
    }
}

fn assert_rejected(result: &Output, output: &Path, reference: &str, codepoint: &str, prefix: &str) {
    assert_eq!(result.status.code(), Some(1), "{reference}: {result:?}");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains(prefix), "{reference}: {stderr}");
    assert!(stderr.contains(reference), "{reference}: {stderr}");
    assert!(stderr.contains(codepoint), "{reference}: {stderr}");
    assert!(
        !output.exists(),
        "{reference}: no output may be written on failure"
    );
}

#[test]
fn convert_rejects_illegal_character_references_in_text() {
    for (index, &(reference, codepoint)) in ILLEGAL_REFS.iter().enumerate() {
        let input = write(
            &format!("reject_text_{index}"),
            PLAIN,
            &format!("a{reference}b"),
        );
        let output = tmp(&format!("reject_text_{index}.out.svg"));
        let _ = fs::remove_file(&output);
        let result = convert(&input, &output);
        assert_rejected(&result, &output, reference, codepoint, "XML error: ");
    }
}

#[test]
fn convert_rejects_illegal_character_references_in_attributes() {
    for (index, &(reference, codepoint)) in ILLEGAL_REFS.iter().enumerate() {
        let input = write(
            &format!("reject_attr_{index}"),
            PLAIN_ATTR,
            &format!("a{reference}b"),
        );
        let output = tmp(&format!("reject_attr_{index}.out.svg"));
        let _ = fs::remove_file(&output);
        let result = convert(&input, &output);
        assert_rejected(&result, &output, reference, codepoint, "XML error: ");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("attribute `data-x`"),
            "{reference}: {stderr}"
        );
    }
}

#[test]
fn plot_grid_rejects_illegal_character_references_in_text() {
    for (index, &(reference, codepoint)) in ILLEGAL_REFS.iter().enumerate() {
        let input = write(
            &format!("pg_reject_text_{index}"),
            PANEL,
            &format!("a{reference}b"),
        );
        let output = tmp(&format!("pg_reject_text_{index}.out.svg"));
        let _ = fs::remove_file(&output);
        let result = plot_grid(&input, &output);
        assert_rejected(&result, &output, reference, codepoint, "SVG XML error: ");
    }
}

#[test]
fn plot_grid_rejects_illegal_character_references_in_attributes() {
    for (index, &(reference, codepoint)) in ILLEGAL_REFS.iter().enumerate() {
        let input = write(
            &format!("pg_reject_attr_{index}"),
            PANEL_ATTR,
            &format!("a{reference}b"),
        );
        let output = tmp(&format!("pg_reject_attr_{index}.out.svg"));
        let _ = fs::remove_file(&output);
        let result = plot_grid(&input, &output);
        assert_rejected(&result, &output, reference, codepoint, "SVG XML error: ");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("attribute `data-x`"),
            "{reference}: {stderr}"
        );
    }
}

#[test]
fn legal_character_references_stay_accepted() {
    for (index, reference) in LEGAL_REFS.iter().enumerate() {
        for (kind, template, command) in [
            ("text", PLAIN, "convert"),
            ("attr", PLAIN_ATTR, "convert"),
            ("pg_text", PANEL, "plot-grid"),
            ("pg_attr", PANEL_ATTR, "plot-grid"),
        ] {
            let name = format!("legal_{kind}_{index}");
            let input = write(&name, template, &format!("a{reference}b"));
            let output = tmp(&format!("{name}.out.svg"));
            let _ = fs::remove_file(&output);
            let result = if command == "convert" {
                convert(&input, &output)
            } else {
                plot_grid(&input, &output)
            };
            assert!(result.status.success(), "{reference} {kind}: {result:?}");
            assert!(output.exists(), "{reference} {kind}: no output");
            assert_well_formed(&output);
        }
    }
}

/// The strong invariant: for a batch that mixes legal references, predefined
/// entities, CDATA and illegal references, **every** output that was written is
/// well-formed XML 1.0. A build that lets `&#1;` through (rc=0, bare 0x01 in the
/// product) fails here even though the exit code looks fine.
#[test]
fn every_accepted_output_is_well_formed() {
    let text_cases: &[(&str, &str, bool)] = &[
        ("plain", "hello", true),
        ("predefined", "&amp;&lt;&gt;&quot;&apos;", true),
        ("decimal", "37&#176;C", true),
        ("hex", "&#x1F600;", true),
        ("cdata", "<![CDATA[37&#176;C]]>", true),
        ("astral", "😀", true),
        ("tab", "a&#9;b", true),
        ("replacement", "a&#xFFFD;b", true),
        ("illegal_1", "a&#1;b", false),
        ("illegal_fffe", "a&#xFFFE;b", false),
        ("illegal_0", "a&#0;b", false),
        ("illegal_surrogate", "a&#55296;b", false),
        ("illegal_overflow", "a&#x110000;b", false),
    ];
    let attr_cases: &[(&str, &str, bool)] = &[
        ("plain", "hello", true),
        ("predefined", "&amp;&lt;&gt;&quot;&apos;", true),
        ("decimal", "37&#176;C", true),
        ("hex", "&#x1F600;", true),
        ("astral", "😀", true),
        ("tab", "a&#9;b", true),
        ("replacement", "a&#xFFFD;b", true),
        ("illegal_1", "a&#1;b", false),
        ("illegal_fffe", "a&#xFFFE;b", false),
        ("illegal_0", "a&#0;b", false),
        ("illegal_surrogate", "a&#55296;b", false),
        ("illegal_overflow", "a&#x110000;b", false),
    ];

    let mut accepted = 0;
    let mut rejected = 0;
    for (index, &(name, body, must_accept)) in text_cases.iter().enumerate() {
        for (kind, template, command) in
            [("text", PLAIN, "convert"), ("pg_text", PANEL, "plot-grid")]
        {
            let case = format!("inv_{kind}_{index}");
            let input = write(&case, template, body);
            let output = tmp(&format!("{case}.out.svg"));
            let _ = fs::remove_file(&output);
            let result = if command == "convert" {
                convert(&input, &output)
            } else {
                plot_grid(&input, &output)
            };
            let rc = result.status.code();
            if rc == Some(0) {
                accepted += 1;
                assert!(output.exists(), "{name} {kind}: rc=0 but no output");
                assert_well_formed(&output);
            } else {
                rejected += 1;
                assert_eq!(rc, Some(1), "{name} {kind}: {result:?}");
                assert!(!output.exists(), "{name} {kind}: no output on failure");
            }
            if must_accept {
                assert_eq!(
                    rc,
                    Some(0),
                    "{name} {kind}: expected acceptance: {result:?}"
                );
            } else {
                assert_eq!(rc, Some(1), "{name} {kind}: expected rejection: {result:?}");
            }
        }
    }
    for (index, &(name, body, must_accept)) in attr_cases.iter().enumerate() {
        for (kind, template, command) in [
            ("attr", PLAIN_ATTR, "convert"),
            ("pg_attr", PANEL_ATTR, "plot-grid"),
        ] {
            let case = format!("inv_{kind}_{index}");
            let input = write(&case, template, body);
            let output = tmp(&format!("{case}.out.svg"));
            let _ = fs::remove_file(&output);
            let result = if command == "convert" {
                convert(&input, &output)
            } else {
                plot_grid(&input, &output)
            };
            let rc = result.status.code();
            if rc == Some(0) {
                accepted += 1;
                assert!(output.exists(), "{name} {kind}: rc=0 but no output");
                assert_well_formed(&output);
            } else {
                rejected += 1;
                assert_eq!(rc, Some(1), "{name} {kind}: {result:?}");
                assert!(!output.exists(), "{name} {kind}: no output on failure");
            }
            if must_accept {
                assert_eq!(
                    rc,
                    Some(0),
                    "{name} {kind}: expected acceptance: {result:?}"
                );
            } else {
                assert_eq!(rc, Some(1), "{name} {kind}: expected rejection: {result:?}");
            }
        }
    }
    assert!(
        accepted > 0 && rejected > 0,
        "accepted={accepted} rejected={rejected}"
    );
}
