use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use svg_grid::{
    auto_label_top_band, auto_labels, plot_grid, render_svg_to_avif, render_svg_to_png_with_dpi,
    AlignMode, InputNormalize, LabelSize, LabelSpec, Margins, PlotGridSpec, SvgInput,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("svg_grid: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> std::result::Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (command, rest) = args.split_first().ok_or_else(|| usage().to_string())?;
    if command != "plot-grid" {
        return Err(usage().to_string());
    }

    let mut inputs = Vec::new();
    let mut output_svg = None;
    let mut output_svgz = None;
    let mut output_png = None;
    let mut output_avif = None;
    let mut avif_quality = 70.0;
    let mut png_dpi = 600.0;
    let mut width = 1200.0;
    // Omitted → the canvas height is derived from the panels' natural heights.
    let mut height: Option<f32> = None;
    let mut nrow = None;
    let mut ncol = None;
    let mut rel_widths = Vec::new();
    let mut rel_heights = Vec::new();
    let mut plot_margin = Margins::uniform(24.0);
    let mut plot_margin_given = false;
    let mut gap = 0.0;
    let mut normalize = None;
    let mut labels = None;
    let mut label_x = vec![0.0];
    let mut label_y = vec![1.0];
    // Default: the letter sits just outside the panel's top-left corner (see the
    // `build_label` defaults).
    let mut label_hjust = vec![0.0];
    let mut label_vjust = vec![-0.25];
    let mut label_size = LabelSize::Auto;
    let mut label_fontfamily = "sans-serif".to_string();
    let mut label_fontface = "bold".to_string();
    let mut label_colour = "#000000".to_string();

    let mut index = 0;
    while index < rest.len() {
        let flag = rest[index].clone();

        // `--spacer` is a bare flag: it inserts a blank grid cell.
        if flag == "--spacer" {
            inputs.push(SvgInput::Spacer);
            index += 1;
            continue;
        }

        // `--labels` takes an *optional* value: a bare `--labels` (or one directly
        // followed by another `--flag`) means AUTO; `--labels A,B` sets them.
        if flag == "--labels" {
            match rest.get(index + 1) {
                Some(value) if !value.starts_with('-') => {
                    labels = Some(value.clone());
                    index += 2;
                }
                _ => {
                    labels = Some("AUTO".to_string());
                    index += 1;
                }
            }
            continue;
        }

        let value = rest
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--input" => inputs.push(if is_spacer_value(value) {
                SvgInput::Spacer
            } else {
                SvgInput::Path(PathBuf::from(value))
            }),
            "--output" => output_svg = Some(PathBuf::from(value)),
            "--output-svgz" => output_svgz = Some(PathBuf::from(value)),
            "--output-png" => output_png = Some(PathBuf::from(value)),
            "--output-avif" => output_avif = Some(PathBuf::from(value)),
            "--avif-quality" => avif_quality = parse_f32(value, "--avif-quality")?,
            "--png-dpi" | "--png_dpi" => png_dpi = parse_f64(value, "--png-dpi")?,
            "--width" => width = parse_f32(value, "--width")?,
            "--height" => {
                height = if value.eq_ignore_ascii_case("auto") {
                    None
                } else {
                    Some(parse_f32(value, "--height")?)
                }
            }
            "--nrow" => nrow = Some(parse_usize(value, "--nrow")?),
            "--ncol" => ncol = Some(parse_usize(value, "--ncol")?),
            "--rel-widths" => rel_widths = parse_rel_f32_list(value, "--rel-widths")?,
            "--rel-heights" => rel_heights = parse_rel_f32_list(value, "--rel-heights")?,
            "--plot-margin" | "--plot_margin" => {
                plot_margin = parse_margins(value, "--plot-margin")?;
                plot_margin_given = true;
            }
            "--normalize-input-max-side" | "--normalize_input_max_side" => {
                let max_side = parse_f32(value, "--normalize-input-max-side")?;
                if max_side <= 0.0 {
                    return Err("--normalize-input-max-side must be greater than zero".to_string());
                }
                normalize = Some(InputNormalize { max_side });
            }
            "--label-x" | "--label_x" => label_x = parse_f32_list(value, "--label-x")?,
            "--label-y" | "--label_y" => label_y = parse_f32_list(value, "--label-y")?,
            "--hjust" => label_hjust = parse_f32_list(value, "--hjust")?,
            "--vjust" => label_vjust = parse_f32_list(value, "--vjust")?,
            "--label-size" | "--label_size" | "--label-font-size" | "--label_font_size" => {
                label_size = if value.eq_ignore_ascii_case("auto") {
                    LabelSize::Auto
                } else {
                    LabelSize::Fixed(parse_f32(value, "--label-size")?)
                }
            }
            "--label-fontfamily"
            | "--label_fontfamily"
            | "--label-font-family"
            | "--label_font_family" => label_fontfamily = value.to_string(),
            "--label-fontface" | "--label_fontface" | "--label-font-face" | "--label_font_face" => {
                label_fontface = value.to_string()
            }
            "--label-colour" | "--label_colour" | "--label-color" | "--label_color" => {
                label_colour = value.to_string()
            }
            "--gap" => gap = parse_f32(value, "--gap")?,
            _ => return Err(format!("unknown argument: {flag}\n{}", usage())),
        }
        index += 2;
    }

    let keep_svg = output_svg.is_some();
    let output_svg = output_svg
        .or_else(|| {
            temporary_svg_path(
                output_svgz.as_ref(),
                output_avif.as_ref(),
                output_png.as_ref(),
            )
        })
        .ok_or_else(|| {
            "missing output: provide --output, --output-svgz, --output-avif, or --output-png"
                .to_string()
        })?;
    let spacers: Vec<bool> = inputs
        .iter()
        .map(|input| matches!(input, SvgInput::Spacer))
        .collect();
    let label_list = labels
        .as_deref()
        .map(|labels| parse_labels(labels, &spacers));
    // Reserve the letter band automatically, unless the caller pinned --plot-margin
    // (in which case it is respected, and a too-small one only warns).
    if !plot_margin_given
        && label_list
            .as_ref()
            .is_some_and(|list| list.iter().any(|label| !label.is_empty()))
    {
        let band = auto_label_top_band(label_size.resolve(width), &label_vjust);
        plot_margin.top = plot_margin.top.max(band);
    }
    let label_spec = label_list.map(|labels| LabelSpec {
        labels,
        x: label_x,
        y: label_y,
        hjust: label_hjust,
        vjust: label_vjust,
        size: label_size,
        font_family: label_fontfamily,
        font_face: label_fontface,
        colour: label_colour,
    });
    plot_grid(PlotGridSpec {
        inputs,
        output_svg: output_svg.clone(),
        output_svgz,
        width,
        height,
        nrow,
        ncol,
        rel_widths,
        rel_heights,
        margin: plot_margin,
        gap,
        align: AlignMode::Panels,
        labels: label_spec,
        normalize,
    })
    .map_err(|error| error.to_string())?;

    if let Some(output_png) = output_png {
        render_svg_to_png_with_dpi(&output_svg, &output_png, png_dpi)
            .map_err(|error| error.to_string())?;
    }
    if let Some(output_avif) = output_avif {
        render_svg_to_avif(&output_svg, &output_avif, avif_quality)
            .map_err(|error| error.to_string())?;
    }
    if !keep_svg {
        let _ = fs::remove_file(&output_svg);
    }

    Ok(())
}

fn parse_f32(value: &str, flag: &str) -> std::result::Result<f32, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be a number, got {value:?}"))
}

fn parse_f64(value: &str, flag: &str) -> std::result::Result<f64, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be a number, got {value:?}"))
}

fn parse_usize(value: &str, flag: &str) -> std::result::Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be a positive integer, got {value:?}"))
}

fn parse_f32_list(value: &str, flag: &str) -> std::result::Result<Vec<f32>, String> {
    let values: std::result::Result<Vec<_>, _> = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| parse_f32(part, flag))
        .collect();
    let values = values?;
    if values.is_empty() {
        return Err(format!(
            "{flag} must be a comma-separated list containing at least one number"
        ));
    }
    Ok(values)
}

/// `--rel-*`: the multipliers may be `0` (e.g. to collapse a column), so only
/// negative values are rejected here; "all zero" is caught by the layout.
fn parse_rel_f32_list(value: &str, flag: &str) -> std::result::Result<Vec<f32>, String> {
    let values = parse_f32_list(value, flag)?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(format!(
            "{flag} must be a comma-separated list of non-negative numbers"
        ));
    }
    Ok(values)
}

/// A spacer input: `--input none` / `--input null` / `--input NULL` (cowplot's
/// `NULL`), or the bare `--spacer` flag.
fn is_spacer_value(value: &str) -> bool {
    value.eq_ignore_ascii_case("none") || value.eq_ignore_ascii_case("null")
}

fn parse_margins(value: &str, flag: &str) -> std::result::Result<Margins, String> {
    let values = parse_f32_list(value, flag)?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(format!("{flag} values must be non-negative"));
    }
    match values.as_slice() {
        [all] => Ok(Margins::uniform(*all)),
        [top, right, bottom, left] => Ok(Margins {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => Err(format!(
            "{flag} must be one number or four comma-separated numbers: top,right,bottom,left"
        )),
    }
}

/// Resolve `--labels`. AUTO numbering labels the *drawn* panels in order and
/// leaves spacer cells blank; explicit lists are taken verbatim (as in R, use
/// `--labels ,,E` style placeholders for the cells you do not want lettered).
fn parse_labels(value: &str, spacers: &[bool]) -> Vec<String> {
    match value {
        "none" => Vec::new(),
        "AUTO" | "auto" => {
            let uppercase = value == "AUTO";
            let real = spacers.iter().filter(|spacer| !**spacer).count();
            let mut letters = auto_labels(real, uppercase).into_iter();
            spacers
                .iter()
                .map(|spacer| {
                    if *spacer {
                        String::new()
                    } else {
                        letters.next().unwrap_or_default()
                    }
                })
                .collect()
        }
        _ => value
            .split(',')
            .map(str::trim)
            .map(str::to_string)
            .collect(),
    }
}

fn temporary_svg_path(
    output_svgz: Option<&PathBuf>,
    output_avif: Option<&PathBuf>,
    output_png: Option<&PathBuf>,
) -> Option<PathBuf> {
    let anchor = output_svgz.or(output_avif).or(output_png)?;
    let mut path = anchor.clone();
    let file_name = anchor
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("plot_grid");
    path.set_file_name(format!(".{file_name}.{}.tmp.svg", std::process::id()));
    Some(path)
}

fn usage() -> &'static str {
    "usage: svg_grid plot-grid --input A.svg [--input B.svgz] [--output OUT.svg] [--output-svgz OUT.svgz] [--output-png OUT.png] [--output-avif OUT.avif] [--avif-quality 70] [--png-dpi 600] [--width 1200] [--height 800] [--ncol 2] [--nrow 1] [--rel-widths 1,2] [--rel-heights 1,1] [--plot-margin 24] [--normalize-input-max-side 1200] [--labels [A,B,...]] [--label-size auto|<size>] [--label-x 0] [--label-y 1] [--hjust 0] [--vjust -0.25] [--label-fontfamily sans-serif] [--label-fontface bold] [--gap 20]

layout:
  --height is optional. Omitted (or --height auto) → the canvas height is the sum of the
           rows' natural heights + margins + row gaps, and becomes the output canvas height.
  --rel-widths/--rel-heights are multipliers on each column/row's NATURAL size (cowplot
           semantics): omit them to let the panels' canvas aspect ratios set the layout
           (each cell then matches its panel's aspect → no distortion). Omitted = all 1.
           Values may be 0 (e.g. to collapse a column); negatives and a length that does
           not match the column/row count are errors.
  spacers: a blank cell (cowplot's NULL) is `--input none` (or `--input null`), or the
           bare `--spacer` flag — e.g. `--input A.svg --input none --input B.svg
           --rel-widths 1,0.6,1` leaves whitespace between A and B.

panel letters (--labels):
  auto band: with --labels and no explicit --plot-margin, the top margin is grown to
             hold the letter band (≈1.05 × the font size) so the letters fit.
             An explicit --plot-margin is respected; if a letter would be clipped,
             a warning is printed to stderr (the run still exits 0).
  (omit --labels)   no letters are drawn (default)
  --labels          AUTO numbering: A, B, C, ... in panel order
  --labels A,B,C    explicit comma-separated labels
  --labels none     no letters (same as omitting --labels)

--label-size: 'auto' (default) = ~2.7% of the canvas width, chosen so a serif 'A'
              has an ink width of ~2% of the canvas width; or an explicit number."
}
