//! Command-line parsing (`ARCHITECTURE/cli.md` §子命令与参数面).
//!
//! A pure function over the arguments: no environment, no cwd, no file system, so
//! every branch can be unit-tested (see `tests.rs`). Nothing here panics on user
//! input — every rejection is a [`CliError::Usage`] (exit code 2).

use std::path::PathBuf;

use svg_grid::{InputNormalize, Margins, SvgInput};

use super::CliError;

/// Every flag the `plot-grid` subcommand accepts — **one spelling per meaning**.
///
/// It is the *single* registry: the parser resolves a token through it and the unit
/// test for `--help` coverage walks it, so a new flag cannot be added without either
/// documenting it or failing that test.
///
/// The second column is the **alias column, and it is empty for every flag** (R26):
/// the underscore spellings (`--font_dir`, `--label_colour`, …) that used to live
/// there are gone, and [`canonical`] reads **only the first column** — so a spelling
/// parked here would still be an `unknown flag` at parse time. `no_flag_has_an_alias`
/// asserts the column stays empty. `ARCHITECTURE/cli.md` §子命令与参数面 has the rule.
pub(crate) const FLAGS: &[(&str, &[&str])] = &[
    ("--input", &[]),
    ("--spacer", &[]),
    ("--output", &[]),
    ("--output-svgz", &[]),
    ("--output-png", &[]),
    ("--output-avif", &[]),
    ("--avif-quality", &[]),
    ("--png-dpi", &[]),
    ("--width", &[]),
    ("--height", &[]),
    ("--nrow", &[]),
    ("--ncol", &[]),
    ("--rel-widths", &[]),
    ("--rel-heights", &[]),
    ("--gap", &[]),
    ("--plot-margin", &[]),
    ("--normalize-input-max-side", &[]),
    ("--plan", &[]),
    ("--labels", &[]),
    ("--label-size", &[]),
    ("--label-x", &[]),
    ("--label-y", &[]),
    ("--hjust", &[]),
    ("--vjust", &[]),
    ("--label-fontfamily", &[]),
    ("--label-fontface", &[]),
    ("--label-colour", &[]),
    ("--font-dir", &[]),
    ("--list-fonts", &[]),
];

/// Flags handled *before* any value is taken, so they must also appear in `--help`.
pub(crate) const HELP_FLAGS: &[&str] = &["-h", "--help"];
/// The version flag; also handled before any value is taken.
pub(crate) const VERSION_FLAG: &str = "--version";

/// A parsed command line.
#[derive(Debug)]
pub(crate) enum Invocation {
    /// `-h` / `--help`: print [`super::help::HELP`] on stdout, exit 0.
    Help,
    /// `--version`: print the version, exit 0.
    Version,
    PlotGrid(Box<PlotGridArgs>),
    Doctor(Box<super::doctor::DoctorArgs>),
    /// `convert …`: the unconsumed tail, handed to the converter's own parser
    /// (`svg_grid_convert::cli::run_args`). This crate models no `convert` flags of
    /// its own — the library's surface is the only one there is.
    Convert(Vec<String>),
}

/// Everything `plot-grid` accepts, with the documented defaults.
#[derive(Debug)]
pub(crate) struct PlotGridArgs {
    pub inputs: Vec<SvgInput>,
    pub output_svg: Option<PathBuf>,
    pub output_svgz: Option<PathBuf>,
    pub output_png: Option<PathBuf>,
    pub output_avif: Option<PathBuf>,
    pub avif_quality: f32,
    pub png_dpi: f64,
    pub width: f32,
    /// `None` → derive the canvas height from the panels' natural heights.
    pub height: Option<f32>,
    pub nrow: Option<usize>,
    pub ncol: Option<usize>,
    pub rel_widths: Vec<f32>,
    pub rel_heights: Vec<f32>,
    pub plot_margin: Margins,
    pub plot_margin_given: bool,
    pub gap: f32,
    pub normalize: Option<InputNormalize>,
    /// Raw `--labels` value: `None` when absent, `Some("AUTO")` for a bare `--labels`.
    pub labels: Option<String>,
    pub label_x: Vec<f32>,
    pub label_y: Vec<f32>,
    pub label_hjust: Vec<f32>,
    pub label_vjust: Vec<f32>,
    pub label_size: LabelSizeValue,
    pub label_fontfamily: String,
    pub label_fontface: String,
    pub label_colour: String,
    /// `None` → `$SVG_GRID_FONT_DIR`, else the data root (`$HOME/svg_grid/fontlib`).
    pub font_dir: Option<PathBuf>,
    pub list_fonts: bool,
    /// `--plan`: print the resolved layout on stdout and exit, writing nothing
    /// (`ARCHITECTURE/cli.md` §`--plan`). Read-only by contract, so it rejects
    /// every output flag instead of silently ignoring one.
    pub plan: bool,
}

/// `--label-size` before the layout resolves it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LabelSizeValue {
    Auto,
    Fixed(f32),
}

impl Default for PlotGridArgs {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            output_svg: None,
            output_svgz: None,
            output_png: None,
            output_avif: None,
            avif_quality: 70.0,
            png_dpi: 600.0,
            width: 1200.0,
            height: None,
            nrow: None,
            ncol: None,
            rel_widths: Vec::new(),
            rel_heights: Vec::new(),
            plot_margin: Margins::uniform(24.0),
            plot_margin_given: false,
            gap: 0.0,
            normalize: None,
            labels: None,
            label_x: vec![0.0],
            label_y: vec![1.0],
            // Default: the letter sits just outside the panel's top-left corner.
            label_hjust: vec![0.0],
            label_vjust: vec![-0.25],
            label_size: LabelSizeValue::Auto,
            // The panel letter's family: Arial, and every family comes from the font
            // library (`ARCHITECTURE/fonts.md`). A CSS list works as a fallback chain.
            label_fontfamily: "Arial".to_string(),
            label_fontface: "bold".to_string(),
            label_colour: "#000000".to_string(),
            font_dir: None,
            list_fonts: false,
            plan: false,
        }
    }
}

/// Parse the arguments that follow the program name.
pub(crate) fn parse(args: &[String]) -> Result<Invocation, CliError> {
    // `-h`/`--help`/`--version` win over everything and are recognised *before* any
    // flag takes a value: `plot-grid --help` must print help, not complain that
    // `--help` has no value (`ARCHITECTURE/cli.md` §`--help` 契约).
    if args.iter().any(|arg| HELP_FLAGS.contains(&arg.as_str())) {
        return Ok(Invocation::Help);
    }
    if args.iter().any(|arg| arg == VERSION_FLAG) {
        return Ok(Invocation::Version);
    }

    let Some((command, rest)) = args.split_first() else {
        return Err(CliError::usage("missing subcommand"));
    };
    match command.as_str() {
        "help" => Ok(Invocation::Help),
        "plot-grid" => Ok(Invocation::PlotGrid(Box::new(parse_plot_grid(rest)?))),
        "doctor" => Ok(Invocation::Doctor(Box::new(super::doctor::parse(rest)?))),
        // `convert` parses its own flags (`--input/--output/--check/--report/--force`,
        // plus `-i/-o/-f`): the converter library's parser is the single
        // implementation, so no value is inspected here. `-h/--help` was already
        // handled above (it prints svg_grid's help, exit 0).
        "convert" => Ok(Invocation::Convert(rest.to_vec())),
        other => Err(CliError::usage(format!("unknown subcommand {other:?}"))),
    }
}

fn parse_plot_grid(rest: &[String]) -> Result<PlotGridArgs, CliError> {
    let mut args = PlotGridArgs::default();
    let mut index = 0;

    while index < rest.len() {
        let raw = rest[index].as_str();
        let Some(flag) = canonical(raw) else {
            return Err(CliError::usage(format!("unknown flag {raw:?}")));
        };

        // Bare flags: they never take a value.
        match flag {
            "--spacer" => {
                args.inputs.push(SvgInput::Spacer);
                index += 1;
                continue;
            }
            "--list-fonts" => {
                args.list_fonts = true;
                index += 1;
                continue;
            }
            "--plan" => {
                args.plan = true;
                index += 1;
                continue;
            }
            _ => {}
        }

        // `--labels` takes an *optional* value: a bare `--labels` (or one directly
        // followed by another flag) means AUTO; `--labels A,B` sets the letters.
        if flag == "--labels" {
            match rest.get(index + 1) {
                Some(value) if !is_flag_like(value) => {
                    args.labels = Some(value.clone());
                    index += 2;
                }
                _ => {
                    args.labels = Some("AUTO".to_string());
                    index += 1;
                }
            }
            continue;
        }

        let value = take_value(rest, index, flag)?;
        match flag {
            "--input" => args.inputs.push(if is_spacer_value(value) {
                SvgInput::Spacer
            } else {
                SvgInput::Path(PathBuf::from(value))
            }),
            "--output" => args.output_svg = Some(PathBuf::from(value)),
            "--output-svgz" => args.output_svgz = Some(PathBuf::from(value)),
            "--output-png" => args.output_png = Some(PathBuf::from(value)),
            "--output-avif" => args.output_avif = Some(PathBuf::from(value)),
            // ravif asserts on an out-of-range quality; validating it here keeps the
            // user's own argument from becoming a panic (`rc=101`).
            "--avif-quality" => {
                let quality = parse_f32(flag, value)?;
                if !(1.0..=100.0).contains(&quality) {
                    return Err(CliError::usage(format!(
                        "--avif-quality must be between 1 and 100, got {value:?}"
                    )));
                }
                args.avif_quality = quality;
            }
            "--png-dpi" => {
                let dpi = parse_f64(flag, value)?;
                args.png_dpi = positive_f64(flag, dpi)?;
            }
            "--width" => args.width = positive_f32(flag, parse_f32(flag, value)?)?,
            "--height" => {
                args.height = if value.eq_ignore_ascii_case("auto") {
                    None
                } else {
                    Some(positive_f32(flag, parse_f32(flag, value)?)?)
                }
            }
            "--nrow" => args.nrow = Some(count(flag, value)?),
            "--ncol" => args.ncol = Some(count(flag, value)?),
            "--rel-widths" => args.rel_widths = parse_rel_list(flag, value)?,
            "--rel-heights" => args.rel_heights = parse_rel_list(flag, value)?,
            "--gap" => args.gap = non_negative(flag, parse_f32(flag, value)?)?,
            "--plot-margin" => {
                args.plot_margin = parse_margins(flag, value)?;
                args.plot_margin_given = true;
            }
            "--normalize-input-max-side" => {
                args.normalize = Some(InputNormalize {
                    max_side: positive_f32(flag, parse_f32(flag, value)?)?,
                });
            }
            "--label-x" => args.label_x = parse_f32_list(flag, value)?,
            "--label-y" => args.label_y = parse_f32_list(flag, value)?,
            "--hjust" => args.label_hjust = parse_f32_list(flag, value)?,
            "--vjust" => args.label_vjust = parse_f32_list(flag, value)?,
            "--label-size" => {
                args.label_size = if value.eq_ignore_ascii_case("auto") {
                    LabelSizeValue::Auto
                } else {
                    LabelSizeValue::Fixed(positive_f32(flag, parse_f32(flag, value)?)?)
                };
            }
            "--label-fontfamily" => args.label_fontfamily = value.to_string(),
            "--label-fontface" => args.label_fontface = value.to_string(),
            "--label-colour" => args.label_colour = value.to_string(),
            "--font-dir" => args.font_dir = Some(PathBuf::from(value)),
            // Not reachable: `every_registered_flag_is_implemented` proves that every entry of
            // `FLAGS` has an arm here, and an unknown token never gets this far.
            other => unreachable!("flag {other} has no parser arm"),
        }
        index += 2;
    }

    // `--plan` prints the layout and exits, writing nothing. An output path next to
    // it is a contradiction — silently ignoring the output would make the caller
    // believe a figure was written — so it is rejected here, before any I/O.
    if args.plan {
        let offending = [
            ("--output", args.output_svg.is_some()),
            ("--output-svgz", args.output_svgz.is_some()),
            ("--output-png", args.output_png.is_some()),
            ("--output-avif", args.output_avif.is_some()),
        ]
        .into_iter()
        .find_map(|(flag, given)| given.then_some(flag));
        if let Some(flag) = offending {
            return Err(CliError::usage(format!(
                "--plan writes nothing, so it cannot be combined with {flag}"
            )));
        }
    }

    Ok(args)
}

/// The canonical spelling of `flag`, if the parser knows it.
///
/// Exact match on the **first** column only: the alias column is not consulted, so
/// there is no way for a second spelling of a flag to be accepted (`R26`).
fn canonical(flag: &str) -> Option<&'static str> {
    FLAGS
        .iter()
        .map(|(name, _)| *name)
        .find(|name| *name == flag)
}

/// Whether a token would be read as a flag rather than a value. A negative number
/// (`--vjust -0.25`) is a value, not a flag.
pub(super) fn is_flag_like(token: &str) -> bool {
    token.starts_with('-') && token.parse::<f64>().is_err()
}

pub(super) fn take_value<'a>(
    rest: &'a [String],
    index: usize,
    flag: &str,
) -> Result<&'a str, CliError> {
    match rest.get(index + 1) {
        Some(next) if !is_flag_like(next) => Ok(next.as_str()),
        _ => Err(CliError::usage(format!("missing value for {flag}"))),
    }
}

fn parse_f32(flag: &str, value: &str) -> Result<f32, CliError> {
    let parsed: f32 = value
        .parse()
        .map_err(|_| CliError::usage(format!("{flag} must be a number, got {value:?}")))?;
    if !parsed.is_finite() {
        return Err(CliError::usage(format!(
            "{flag} must be a finite number, got {value:?}"
        )));
    }
    Ok(parsed)
}

fn parse_f64(flag: &str, value: &str) -> Result<f64, CliError> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| CliError::usage(format!("{flag} must be a number, got {value:?}")))?;
    if !parsed.is_finite() {
        return Err(CliError::usage(format!(
            "{flag} must be a finite number, got {value:?}"
        )));
    }
    Ok(parsed)
}

fn positive_f32(flag: &str, value: f32) -> Result<f32, CliError> {
    if value <= 0.0 {
        return Err(CliError::usage(format!(
            "{flag} must be greater than zero, got {value}"
        )));
    }
    Ok(value)
}

fn positive_f64(flag: &str, value: f64) -> Result<f64, CliError> {
    if value <= 0.0 {
        return Err(CliError::usage(format!(
            "{flag} must be greater than zero, got {value}"
        )));
    }
    Ok(value)
}

fn non_negative(flag: &str, value: f32) -> Result<f32, CliError> {
    if value < 0.0 {
        return Err(CliError::usage(format!(
            "{flag} must not be negative, got {value}"
        )));
    }
    Ok(value)
}

/// `--nrow` / `--ncol`: a positive integer. Zero is rejected at parse time; the
/// layout would otherwise divide by it (a panic, `rc=101`).
pub(super) fn count(flag: &str, value: &str) -> Result<usize, CliError> {
    let parsed: usize = value.parse().map_err(|_| {
        CliError::usage(format!("{flag} must be a positive integer, got {value:?}"))
    })?;
    if parsed == 0 {
        return Err(CliError::usage(format!(
            "{flag} must be greater than zero, got {value:?}"
        )));
    }
    Ok(parsed)
}

fn parse_f32_list(flag: &str, value: &str) -> Result<Vec<f32>, CliError> {
    let values: Result<Vec<f32>, CliError> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| parse_f32(flag, part))
        .collect();
    let values = values?;
    if values.is_empty() {
        return Err(CliError::usage(format!(
            "{flag} must be a comma-separated list containing at least one number"
        )));
    }
    Ok(values)
}

/// `--rel-*`: the multipliers may be `0` (e.g. to collapse a column), so only
/// negative values are rejected here; "all zero" is caught by the layout.
fn parse_rel_list(flag: &str, value: &str) -> Result<Vec<f32>, CliError> {
    let values = parse_f32_list(flag, value)?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(CliError::usage(format!(
            "{flag} must be a comma-separated list of non-negative numbers"
        )));
    }
    Ok(values)
}

fn parse_margins(flag: &str, value: &str) -> Result<Margins, CliError> {
    let values = parse_f32_list(flag, value)?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(CliError::usage(format!(
            "{flag} values must be non-negative"
        )));
    }
    match values.as_slice() {
        [all] => Ok(Margins::uniform(*all)),
        [top, right, bottom, left] => Ok(Margins {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => Err(CliError::usage(format!(
            "{flag} must be one number or four comma-separated numbers: top,right,bottom,left"
        ))),
    }
}

/// A spacer input: `--input none` / `--input null` (cowplot's `NULL`), or `--spacer`.
fn is_spacer_value(value: &str) -> bool {
    value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("null")
}
