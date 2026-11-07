//! Unit tests for the command-line parser (`ARCHITECTURE/cli.md` §单测要求).
//!
//! They call [`parse`] directly — no process spawn, no cwd, no file system, no font
//! directory — so they assert the argument surface itself rather than a machine's
//! environment.

use super::parse::{
    parse, Invocation, LabelSizeValue, PlotGridArgs, FLAGS, HELP_FLAGS, VERSION_FLAG,
};
use super::{help, CliError};
use svg_grid::{SvgGridError, SvgInput};

/// Parse `args` and unwrap the `plot-grid` arguments.
fn plot_grid(args: &[&str]) -> PlotGridArgs {
    let mut argv = vec!["plot-grid".to_string()];
    argv.extend(args.iter().map(|arg| arg.to_string()));
    match parse(&argv) {
        Ok(Invocation::PlotGrid(args)) => *args,
        other => panic!("expected plot-grid, got {other:?}"),
    }
}

/// Parse `args` and expect a usage error (exit code 2).
fn usage_error(args: &[&str]) -> CliError {
    let argv: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    match parse(&argv) {
        Err(error) => {
            assert_eq!(error.exit_code(), 2, "usage errors exit 2: {error}");
            error
        }
        Ok(other) => panic!("expected a usage error, got {other:?}"),
    }
}

#[test]
fn defaults_match_the_documented_ones() {
    let args = plot_grid(&["--input", "a.svg", "--output", "o.svg"]);
    assert_eq!(args.avif_quality, 70.0);
    assert_eq!(args.png_dpi, 600.0);
    assert_eq!(args.width, 1200.0);
    assert_eq!(args.height, None);
    assert_eq!((args.nrow, args.ncol), (None, None));
    assert_eq!(args.plot_margin.top, 24.0);
    assert!(!args.plot_margin_given);
    assert_eq!(args.gap, 0.0);
    assert_eq!(args.labels, None);
    assert_eq!(args.label_fontfamily, "Arial");
    assert_eq!(args.label_fontface, "bold");
    assert_eq!(args.label_colour, "#000000");
    assert_eq!(args.label_size, LabelSizeValue::Auto);
    assert_eq!(args.label_vjust, vec![-0.25]);
    assert_eq!(args.font_dir, None);
    assert!(!args.list_fonts);
    assert!(!args.plan);
}

#[test]
fn every_value_flag_is_taken_and_stored() {
    let args = plot_grid(&[
        "--input",
        "a.svg",
        "--spacer",
        "--input",
        "null",
        "--output",
        "o.svg",
        "--output-svgz",
        "o.svgz",
        "--output-png",
        "o.png",
        "--output-avif",
        "o.avif",
        "--avif-quality",
        "42",
        "--png-dpi",
        "300",
        "--width",
        "800",
        "--height",
        "600",
        "--nrow",
        "2",
        "--ncol",
        "3",
        "--rel-widths",
        "1,2,3",
        "--rel-heights",
        "1,0",
        "--gap",
        "7",
        "--plot-margin",
        "1,2,3,4",
        "--normalize-input-max-side",
        "900",
        "--labels",
        "A,B",
        "--label-size",
        "12",
        "--label-x",
        "0.1",
        "--label-y",
        "0.9",
        "--hjust",
        "0.5",
        "--vjust",
        "-0.5",
        "--label-fontfamily",
        "Arial, sans-serif",
        "--label-fontface",
        "normal",
        "--label-colour",
        "#123456",
        "--font-dir",
        "/tmp/fonts",
    ]);
    assert_eq!(
        args.inputs.len(),
        3,
        "two paths plus one spacer from `--input null`"
    );
    assert!(matches!(args.inputs[0], SvgInput::Path(_)));
    assert!(matches!(args.inputs[1], SvgInput::Spacer));
    assert!(matches!(args.inputs[2], SvgInput::Spacer));
    assert_eq!(args.output_svg.as_deref().unwrap().to_str(), Some("o.svg"));
    assert_eq!(
        args.output_svgz.as_deref().unwrap().to_str(),
        Some("o.svgz")
    );
    assert_eq!(args.output_png.as_deref().unwrap().to_str(), Some("o.png"));
    assert_eq!(
        args.output_avif.as_deref().unwrap().to_str(),
        Some("o.avif")
    );
    assert_eq!(args.avif_quality, 42.0);
    assert_eq!(args.png_dpi, 300.0);
    assert_eq!(args.width, 800.0);
    assert_eq!(args.height, Some(600.0));
    assert_eq!((args.nrow, args.ncol), (Some(2), Some(3)));
    assert_eq!(args.rel_widths, vec![1.0, 2.0, 3.0]);
    assert_eq!(args.rel_heights, vec![1.0, 0.0]);
    assert_eq!(args.gap, 7.0);
    assert_eq!(
        (
            args.plot_margin.top,
            args.plot_margin.right,
            args.plot_margin.bottom,
            args.plot_margin.left
        ),
        (1.0, 2.0, 3.0, 4.0)
    );
    assert!(args.plot_margin_given);
    assert_eq!(args.normalize.as_ref().unwrap().max_side, 900.0);
    assert_eq!(args.labels.as_deref(), Some("A,B"));
    assert_eq!(args.label_size, LabelSizeValue::Fixed(12.0));
    assert_eq!(args.label_x, vec![0.1]);
    assert_eq!(args.label_y, vec![0.9]);
    assert_eq!(args.label_hjust, vec![0.5]);
    assert_eq!(args.label_vjust, vec![-0.5]);
    assert_eq!(args.label_fontfamily, "Arial, sans-serif");
    assert_eq!(args.label_fontface, "normal");
    assert_eq!(args.label_colour, "#123456");
    assert_eq!(
        args.font_dir.as_deref().unwrap().to_str(),
        Some("/tmp/fonts")
    );
}

#[test]
fn height_auto_and_bare_labels_have_their_own_meanings() {
    let args = plot_grid(&["--input", "a.svg", "--height", "auto", "--labels"]);
    assert_eq!(args.height, None, "`--height auto` = derive the height");
    assert_eq!(
        args.labels.as_deref(),
        Some("AUTO"),
        "a bare `--labels` = AUTO numbering"
    );

    // `--labels` directly followed by another flag is a bare `--labels` as well.
    let args = plot_grid(&["--labels", "--width", "500", "--input", "a.svg"]);
    assert_eq!(args.labels.as_deref(), Some("AUTO"));
    assert_eq!(args.width, 500.0);
}

/// One spelling per meaning (`ARCHITECTURE/cli.md` §子命令与参数面): the underscore
/// spellings that used to be accepted as an undocumented compatibility layer are
/// gone, and every one of them is a plain `unknown flag` now — never a silent accept.
#[test]
fn underscore_aliases_are_unknown_flags() {
    let aliases = [
        "--png_dpi",
        "--plot_margin",
        "--normalize_input_max_side",
        "--label_size",
        "--label_font_size",
        "--label_x",
        "--label_y",
        "--label_fontfamily",
        "--label_font_family",
        "--label_fontface",
        "--label_font_face",
        "--label_colour",
        "--label_color",
        "--font_dir",
    ];
    assert_eq!(aliases.len(), 14, "the 14 aliases the round removed");
    for alias in aliases {
        let error = usage_error(&["plot-grid", "--input", "a.svg", alias, "x"]);
        assert!(
            error
                .message()
                .starts_with(&format!("unknown flag {alias:?}")),
            "{alias} must be an unknown flag, got: {}",
            error.message()
        );
    }
    // The kebab spelling of the same meaning still works.
    let args = plot_grid(&[
        "--input",
        "a.svg",
        "--font-dir",
        "/tmp/f",
        "--label-fontfamily",
        "serif",
        "--label-colour",
        "#000",
        "--plot-margin",
        "0",
        "--label-size",
        "9",
        "--png-dpi",
        "1200",
    ]);
    assert_eq!(args.font_dir.as_deref().unwrap().to_str(), Some("/tmp/f"));
    assert_eq!(args.label_fontfamily, "serif");
    assert_eq!(args.label_colour, "#000");
    assert_eq!(args.plot_margin.left, 0.0);
    assert_eq!(args.label_size, LabelSizeValue::Fixed(9.0));
    assert_eq!(args.png_dpi, 1200.0);
}

/// The alias column of [`FLAGS`] is dead on purpose: a spelling parked there is still
/// an `unknown flag` (`canonical` reads only the first column), and this keeps it
/// empty so re-introducing the undocumented compatibility layer cannot be silent.
#[test]
fn no_flag_has_an_alias() {
    for (flag, aliases) in FLAGS {
        assert!(aliases.is_empty(), "{flag} carries aliases: {aliases:?}");
    }
}

/// `--plan` prints and writes nothing, so pairing it with an output path is a
/// contradiction the caller has to resolve — never a silent ignore, never a figure
/// that silently fails to appear.
#[test]
fn plan_is_read_only_and_rejects_every_output_flag() {
    let args = plot_grid(&[
        "--input", "a.svg", "--plan", "--width", "500", "--ncol", "2",
    ]);
    assert!(args.plan);
    assert_eq!(args.output_svg, None, "--plan needs no output");

    for flag in ["--output", "--output-svgz", "--output-png", "--output-avif"] {
        let error = usage_error(&["plot-grid", "--input", "a.svg", "--plan", flag, "o.out"]);
        assert!(
            error.message().contains("--plan") && error.message().contains(flag),
            "the message must name both flags, got: {}",
            error.message()
        );
    }
}

#[test]
fn out_of_range_values_are_usage_errors() {
    for args in [
        vec!["--input", "a.svg", "--ncol", "0"],
        vec!["--input", "a.svg", "--nrow", "0"],
        vec!["--input", "a.svg", "--avif-quality", "0"],
        vec!["--input", "a.svg", "--avif-quality", "101"],
        vec!["--input", "a.svg", "--avif-quality", "-1"],
        vec!["--input", "a.svg", "--width", "0"],
        vec!["--input", "a.svg", "--width", "-3"],
        vec!["--input", "a.svg", "--height", "0"],
        vec!["--input", "a.svg", "--gap", "-1"],
        vec!["--input", "a.svg", "--plot-margin", "-1"],
        vec!["--input", "a.svg", "--rel-widths", "-1"],
        vec!["--input", "a.svg", "--normalize-input-max-side", "0"],
        vec!["--input", "a.svg", "--label-size", "0"],
        vec!["--input", "a.svg", "--png-dpi", "0"],
        vec!["--input", "a.svg", "--width", "nan"],
        vec!["--input", "a.svg", "--height", "inf"],
        vec!["--input", "a.svg", "--ncol", "abc"],
        vec!["--input", "a.svg", "--label-x", ""],
        vec!["--input", "a.svg", "--plot-margin", "1,2"],
    ] {
        usage_error(&args);
    }
}

#[test]
fn missing_values_are_usage_errors() {
    for args in [
        vec!["plot-grid", "--input"],
        vec!["plot-grid", "--input", "a.svg", "--output"],
        vec!["plot-grid", "--input", "a.svg", "--output", "--ncol", "2"],
        vec!["plot-grid", "--input", "a.svg", "--font-dir"],
        vec!["plot-grid", "--input", "a.svg", "--label-size"],
    ] {
        let error = usage_error(&args);
        assert!(
            error.message().contains("missing value"),
            "expected a missing-value message, got: {}",
            error.message()
        );
        assert!(
            error.message().contains("--help"),
            "a usage error must point at --help: {}",
            error.message()
        );
    }
}

#[test]
fn unknown_flags_and_subcommands_are_usage_errors() {
    let error = usage_error(&["plot-grid", "--input", "a.svg", "--bogus", "1"]);
    assert!(
        error.message().contains("unknown flag"),
        "{}",
        error.message()
    );
    let error = usage_error(&["plot-grid", "--input", "a.svg", "--reuslt", "1.svg"]);
    assert!(
        error.message().contains("unknown flag"),
        "{}",
        error.message()
    );
    let error = usage_error(&["grid", "--input", "a.svg"]);
    assert!(
        error.message().contains("unknown subcommand"),
        "{}",
        error.message()
    );
    usage_error(&[]);
}

#[test]
fn help_and_version_win_before_any_value_is_taken() {
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["help"],
        vec!["plot-grid", "--help"],
        vec!["plot-grid", "--input", "a.svg", "--ncol", "2", "-h"],
        vec!["--input", "--help"],
    ] {
        let argv: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        assert!(
            matches!(parse(&argv), Ok(Invocation::Help)),
            "{args:?} must print help"
        );
    }
    for args in [vec!["--version"], vec!["plot-grid", "--version"]] {
        let argv: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        assert!(matches!(parse(&argv), Ok(Invocation::Version)), "{args:?}");
    }
}

/// Every flag in the registry has a parser arm: the `unreachable!` in
/// `parse_plot_grid` must be un-triggerable, not merely unlikely.
#[test]
fn every_registered_flag_is_implemented() {
    let bare = ["--spacer", "--list-fonts", "--plan"];
    for (flag, _) in FLAGS {
        let mut argv = vec!["plot-grid".to_string(), flag.to_string()];
        if bare.contains(flag) || *flag == "--labels" {
            // These take no value (or an optional one): a bare occurrence is enough.
            assert!(
                parse(&argv).is_ok(),
                "{flag} must parse without a value, got {:?}",
                parse(&argv).err().map(|error| error.message())
            );
            continue;
        }
        argv.push("x".to_string());
        // A nonsense value may be rejected (that is the point of the range checks),
        // but never by the "no arm for this flag" branch.
        if let Err(error) = parse(&argv) {
            assert!(
                !error.message().contains("has no parser arm"),
                "{flag} is registered but unimplemented: {}",
                error.message()
            );
        }
    }
}

#[test]
fn help_text_covers_every_flag() {
    // One spelling per meaning: the registry holds only canonical spellings, and
    // every one of them must appear in `--help` (`ARCHITECTURE/cli.md` §`--help` 契约).
    for (flag, aliases) in FLAGS {
        assert!(
            help::HELP.contains(flag),
            "{flag} is parsed but missing from --help"
        );
        assert!(aliases.is_empty(), "{flag} must not carry an alias");
    }
    for flag in HELP_FLAGS
        .iter()
        .copied()
        .chain(std::iter::once(VERSION_FLAG))
    {
        assert!(
            help::HELP.contains(flag),
            "{flag} is handled before dispatch but missing from --help"
        );
    }
    // The usage synopsis and the exit-code tiers are part of the contract.
    for needle in ["usage:", "plot-grid", "exit codes:", "0 ", "1 ", "2 ", "3 "] {
        assert!(help::HELP.contains(needle), "--help lacks {needle:?}");
    }
    assert!(
        help::HELP.len() > 1000,
        "the help text must actually document the flags"
    );
}

#[test]
fn exit_codes_are_tiered() {
    // 2 — usage: the command line itself is wrong.
    assert_eq!(CliError::usage("x").exit_code(), 2);
    // The library's own tiers survive the CLI layer (no flattening to 1).
    let fonts = CliError::from(SvgGridError::Fonts("empty".to_string()));
    assert_eq!(
        fonts.exit_code(),
        3,
        "an empty font library is a config error"
    );
    let invalid = CliError::from(SvgGridError::InvalidInput("bad".to_string()));
    assert_eq!(
        invalid.exit_code(),
        2,
        "invalid input is a usage-class error"
    );
    let raster = CliError::from(SvgGridError::Raster("boom".to_string()));
    assert_eq!(
        raster.exit_code(),
        1,
        "rasterisation failure is a runtime error"
    );
    let io = CliError::from(SvgGridError::at(
        "/nope/panel.svg",
        std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
    ));
    assert_eq!(io.exit_code(), 1, "I/O failure is a runtime error");
    assert!(
        io.to_string().contains("/nope/panel.svg"),
        "an I/O failure names the file it happened on: {io}"
    );
}

#[test]
fn no_unit_test_depends_on_the_environment() {
    // Parsing is a pure function: the same arguments must parse identically twice,
    // whatever `SVG_GRID_FONT_DIR` / `HOME` hold (font resolution is not parsing).
    let first = plot_grid(&["--input", "a.svg", "--font-dir", "/tmp/fonts"]);
    let second = plot_grid(&["--input", "a.svg", "--font-dir", "/tmp/fonts"]);
    assert_eq!(first.font_dir, second.font_dir);
    assert_eq!(
        first.font_dir.as_deref().unwrap().to_str(),
        Some("/tmp/fonts"),
        "an explicit --font-dir is stored verbatim, never resolved against the cwd"
    );
}

/// `convert` is a subcommand of the **one** binary: its tail is handed over
/// untouched to the converter's own parser, so there is no second parser to drift
/// (`ARCHITECTURE/cli.md` §子命令与参数面).
#[test]
fn convert_is_a_subcommand_that_hands_its_tail_over_verbatim() {
    let argv: Vec<String> = ["convert", "--input", "a.svg", "--check"]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
    match parse(&argv) {
        Ok(Invocation::Convert(rest)) => assert_eq!(rest, vec!["--input", "a.svg", "--check"]),
        other => panic!("expected convert, got {other:?}"),
    }

    // An empty tail parses here and is rejected by the converter (exit 1, like the
    // standalone binary): the subcommand's own errors are not the CLI's.
    let argv = vec!["convert".to_string()];
    assert!(matches!(parse(&argv), Ok(Invocation::Convert(rest)) if rest.is_empty()));

    // `-h`/`--help` still wins before the subcommand is even dispatched.
    for argv in [
        vec!["convert".to_string(), "--help".to_string()],
        vec![
            "convert".to_string(),
            "--input".to_string(),
            "-h".to_string(),
        ],
    ] {
        assert!(
            matches!(parse(&argv), Ok(Invocation::Help)),
            "{argv:?} must print the svg_grid help"
        );
    }
}

/// The convert flags are documented in `svg_grid --help` too: the registry is the
/// converter's own, so this walks the real surface (no copy that can drift).
#[test]
fn help_text_covers_every_convert_flag() {
    for (flag, aliases) in super::convert::FLAGS {
        assert!(
            help::HELP.contains(flag),
            "convert flag {flag} is missing from --help"
        );
        let _ = aliases;
    }
    for needle in [
        "svg_grid convert --input",
        "--check",
        "--report",
        "--force",
        "svg_grid convert:",
    ] {
        assert!(help::HELP.contains(needle), "--help lacks {needle:?}");
    }
}
