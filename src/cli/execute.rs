//! Executing a parsed `plot-grid` invocation.
//!
//! Every fallible step returns a typed error ([`SvgGridError`] from the library, or
//! [`CliError::Usage`] for a bad command line), so `main` can map it to the exit-code
//! tiers of `ARCHITECTURE/cli.md` instead of flattening everything into one code.

use std::fs;
use std::path::{Path, PathBuf};

use svg_grid::{
    auto_label_top_band, auto_labels, default_font_dir, font_families, format_number, plan_grid,
    plot_grid, render_svg_to_avif_with_fonts, render_svg_to_png_with_dpi_with_fonts, AlignMode,
    GridPlan, GridRequest, LabelSize, LabelSpec, PlannedInput, PlotGridSpec, SvgInput,
};

use super::parse::{LabelSizeValue, PlotGridArgs};
use super::CliError;

pub(crate) fn run(args: PlotGridArgs) -> Result<(), CliError> {
    // `--font-dir` > `$SVG_GRID_FONT_DIR` > `$HOME/svg_grid/fontlib`; nothing about
    // the build machine's checkout is consulted (`ARCHITECTURE/cli.md`).
    let font_dir = args.font_dir.clone().unwrap_or_else(default_font_dir);

    // `--list-fonts`: print what the library holds and stop — no figure is composed.
    // An empty library is a configuration error (exit 3), so the output is never empty.
    if args.list_fonts {
        for family in font_families(&font_dir)? {
            println!("{family}");
        }
        return Ok(());
    }

    let spacers: Vec<bool> = args
        .inputs
        .iter()
        .map(|input| matches!(input, SvgInput::Spacer))
        .collect();
    let label_list = args
        .labels
        .as_deref()
        .map(|labels| parse_labels(labels, &spacers));
    let wants_letters = label_list
        .as_ref()
        .is_some_and(|list| list.iter().any(|label| !label.is_empty()));
    let label_size = match args.label_size {
        LabelSizeValue::Auto => LabelSize::Auto,
        LabelSizeValue::Fixed(size) => LabelSize::Fixed(size),
    };

    // Reserve the letter band automatically, unless the caller pinned --plot-margin
    // (in which case it is respected, and a too-small one only warns).
    let mut plot_margin = args.plot_margin;
    if !args.plot_margin_given && wants_letters {
        let band = auto_label_top_band(label_size.resolve(args.width), &args.label_vjust);
        plot_margin.top = plot_margin.top.max(band);
    }

    // The geometry of this call, resolved once. `--plan` reports it and the real run
    // composes from the very same values (`ARCHITECTURE/layout.md` §格子尺寸由工具算).
    let request = GridRequest {
        inputs: args.inputs.clone(),
        width: args.width,
        height: args.height,
        nrow: args.nrow,
        ncol: args.ncol,
        rel_widths: args.rel_widths.clone(),
        rel_heights: args.rel_heights.clone(),
        margin: plot_margin,
        gap: args.gap,
        normalize: args.normalize.clone(),
    };

    // `--plan`: print the layout this call would produce and exit 0. Read-only by
    // contract (`ARCHITECTURE/cli.md` §`--plan`): no output path is resolved, the
    // font library is never consulted, no file is opened for writing.
    if args.plan {
        print_plan(&plan_grid(request)?);
        return Ok(());
    }

    let output_svg = args
        .output_svg
        .clone()
        .or_else(|| {
            temporary_svg_path(
                args.output_svgz.as_ref(),
                args.output_avif.as_ref(),
                args.output_png.as_ref(),
            )
        })
        .ok_or_else(|| {
            CliError::usage(
                "missing output: provide --output, --output-svgz, --output-avif, or --output-png",
            )
        })?;
    let keep_svg = args.output_svg.is_some();

    // The font library is consulted only when this run really needs glyphs: PNG /
    // AVIF output, or panel letters. A pure-vector run without letters is not
    // blocked by an empty library. Loading it is fail-closed: an empty library is a
    // configuration error (exit 3), not something to render past.
    let wants_raster = args.output_png.is_some() || args.output_avif.is_some();
    let families = if wants_raster || wants_letters {
        Some(font_families(&font_dir)?)
    } else {
        None
    };

    let label_spec = label_list.map(|labels| LabelSpec {
        labels,
        x: args.label_x.clone(),
        y: args.label_y.clone(),
        hjust: args.label_hjust.clone(),
        vjust: args.label_vjust.clone(),
        size: label_size,
        font_family: args.label_fontfamily.clone(),
        font_face: args.label_fontface.clone(),
        colour: args.label_colour.clone(),
    });

    plot_grid(PlotGridSpec {
        inputs: request.inputs,
        output_svg: output_svg.clone(),
        output_svgz: args.output_svgz,
        width: request.width,
        height: request.height,
        nrow: request.nrow,
        ncol: request.ncol,
        rel_widths: request.rel_widths,
        rel_heights: request.rel_heights,
        margin: request.margin,
        gap: request.gap,
        align: AlignMode::Panels,
        labels: label_spec,
        normalize: request.normalize,
        font_families: families,
    })?;

    if let Some(output_png) = args.output_png {
        render_svg_to_png_with_dpi_with_fonts(&output_svg, &output_png, args.png_dpi, &font_dir)?;
    }
    if let Some(output_avif) = args.output_avif {
        render_svg_to_avif_with_fonts(&output_svg, &output_avif, args.avif_quality, &font_dir)?;
    }
    if !keep_svg {
        let _ = fs::remove_file(&output_svg);
    }

    Ok(())
}

/// Resolve `--labels`. AUTO numbering labels the *drawn* panels in order and leaves
/// spacer cells blank; explicit lists are taken verbatim (as in R, use `--labels
/// ,,E` style placeholders for the cells you do not want lettered).
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

/// Print one `--plan` report on stdout, in the contracted order
/// (`ARCHITECTURE/cli.md` §`--plan` 输出契约): the canvas, then one `#<i>` line per
/// input in input order, then one `note:` line per input — and, when that input has
/// to be re-emitted at its cell size first, the exact command on the next line.
///
/// Every number comes from `plan_grid`, i.e. from the same `prepare` the real run
/// calls; nothing here recomputes geometry.
fn print_plan(plan: &GridPlan) {
    println!(
        "canvas {}x{}",
        format_number(plan.width),
        format_number(plan.height)
    );
    for input in &plan.inputs {
        print_plan_input(input);
    }
}

/// One input's plan line plus its `note:` (and the command when it needs one).
fn print_plan_input(input: &PlannedInput) {
    let (cell_w, cell_h) = input.cell;
    let cell = (format_number(cell_w), format_number(cell_h));
    let SvgInput::Path(path) = &input.input else {
        println!("#{} (blank cell)  cell {}x{}", input.index, cell.0, cell.1);
        println!(
            "note: #{} is a blank cell; nothing is drawn and nothing is scaled.",
            input.index
        );
        return;
    };
    let (Some((canvas_w, canvas_h)), Some((sx, sy))) = (input.canvas, input.scale) else {
        return;
    };
    let distortion = input.distortion().unwrap_or(0.0) * 100.0;
    // A `-0.00%` is noise, not information: clamp it to +0.00.
    let distortion = if distortion.abs() < 0.005 {
        0.0
    } else {
        distortion
    };
    println!(
        "#{} {}  canvas {}x{}  cell {}x{}  s={}x{}  Δ{distortion:+.2}%",
        input.index,
        path.display(),
        format_number(canvas_w),
        format_number(canvas_h),
        cell.0,
        cell.1,
        format_number(sx),
        format_number(sy),
    );

    let stretch = if input.is_stretched() {
        format!(" and its cell is stretched {distortion:+.2}%")
    } else {
        String::new()
    };
    if !input.needs_normalize() {
        if input.is_stretched() {
            println!(
                "note: #{} is at its cell size (s=1){stretch}; omit --rel-* so the cell \
                 matches the panel.",
                input.index
            );
        } else {
            println!(
                "note: #{} is at its cell size (s=1); no normalization needed.",
                input.index
            );
        }
        return;
    }
    // The scale that is actually wrong is the one the fix must remove: report it as
    // one number when both axes agree (the usual case), two when they do not.
    let scale = if (sx - sy).abs() < 1e-5 {
        format_number(sx)
    } else {
        format!("{}x{}", format_number(sx), format_number(sy))
    };
    println!(
        "note: #{} is not at its cell size (s={scale}){stretch}; re-emit it at the cell \
         size first:",
        input.index
    );
    // Copy-pasteable as printed: one input, one column, no margins ⇒ the product's
    // canvas IS the cell, so the later composition runs at s = 1.
    println!(
        "svg_grid plot-grid --input {} --output {} --width {} --ncol 1 --plot-margin 0 \
         --normalize-input-max-side {}",
        path.display(),
        cell_sized_path(path).display(),
        cell.0,
        format_number(input.normalize_target().unwrap_or(cell_w.max(cell_h)))
    );
}

/// Where `--plan` suggests re-emitting a panel at its cell size: next to the source,
/// `<stem>.cell.svg`. Deterministic, and it can never overwrite the source.
fn cell_sized_path(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("panel");
    let mut path = source.to_path_buf();
    path.set_file_name(format!("{stem}.cell.svg"));
    path
}

/// The throwaway SVG written when only a raster output was asked for: it is derived
/// from that output's path and removed at the end of the run.
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
