use std::fs;
use std::path::{Path, PathBuf};

use flate2::Crc;
use ravif::{Encoder, Img, RGBA8};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::{input, Result, SvgGridError};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// IHDR is required by the PNG spec to be the first chunk and to be 13 bytes long.
const IHDR_END: usize = 8 + 4 + 4 + 13 + 4;
const PIXELS_PER_METRE_PER_DPI: f64 = 1.0 / 0.0254;

/// The family svg_grid's own text uses when nothing else is asked for: `Arial`.
///
/// It is the default of `--label-fontfamily` **and** the renderer's "no family
/// declared" default, so text never silently inherits the renderer's own
/// built-in default (see `ARCHITECTURE/fonts.md`).
pub const DEFAULT_FONT_FAMILY: &str = "Arial";

/// Environment variable that overrides the data root for the font library.
pub const FONT_DIR_ENV: &str = "SVG_GRID_FONT_DIR";

/// The **data root** font directory: `<home>/svg_grid/fontlib`.
///
/// Runtime resolution on purpose — a compile-time path would be baked into the
/// binary, so a copy installed elsewhere would still read the build machine's
/// checkout (`ARCHITECTURE/cli.md` §字体目录解析). The home variable itself is
/// resolved cross-platform in [`crate::home`] (`HOME`, else `USERPROFILE` on Windows).
pub fn data_root_font_dir() -> PathBuf {
    crate::home::data_root().join("fontlib")
}

/// The font directory used when `--font-dir` is not given:
/// `$SVG_GRID_FONT_DIR`, else the data root ([`data_root_font_dir`]).
pub fn default_font_dir() -> PathBuf {
    match std::env::var_os(FONT_DIR_ENV) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => data_root_font_dir(),
    }
}

/// Load **only** `font_dir` (recursively) into a fresh font database.
///
/// Fail-closed on purpose: a missing or empty library is a *configuration* error,
/// not a malformed input. Without fonts every glyph would silently disappear while
/// the run still exited 0 — exactly the "looks right, is wrong" outcome this tool
/// must not produce.
pub fn load_font_database(font_dir: &Path) -> Result<usvg::fontdb::Database> {
    let mut database = usvg::fontdb::Database::new();
    database.load_fonts_dir(font_dir);
    if database.is_empty() {
        return Err(SvgGridError::Fonts(format!(
            "no fonts in {} — the font library is missing or empty, so panel letters and text \
             would be laid out with no glyphs while the run still looked successful. Fix: put \
             Arial / Times New Roman fonts (.ttf/.otf/.ttc) into {}, or run fontlib/setup.sh (it \
             copies them from this machine); --font-dir <dir> selects a different font directory.",
            font_dir.display(),
            font_dir.display(),
        )));
    }
    Ok(database)
}

/// Every family present in `font_dir`, deduplicated and sorted — what
/// `--list-fonts` prints.
pub fn font_families(font_dir: &Path) -> Result<Vec<String>> {
    let database = load_font_database(font_dir)?;
    let mut families: Vec<String> = database
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();
    families.sort_by_key(|name| name.to_lowercase());
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    Ok(families)
}

pub fn render_svg_to_png(input_svg: &Path, output_png: &Path) -> Result<()> {
    render_svg_to_png_with_fonts(input_svg, output_png, &default_font_dir())
}

/// [`render_svg_to_png`] for an explicit font directory.
pub fn render_svg_to_png_with_fonts(
    input_svg: &Path,
    output_png: &Path,
    font_dir: &Path,
) -> Result<()> {
    let pixmap = rasterize_svg(input_svg, font_dir)?;
    pixmap.save_png(output_png).map_err(|error| {
        SvgGridError::Raster(format!(
            "failed to write PNG {}: {error}",
            output_png.display()
        ))
    })?;
    Ok(())
}

/// Renders the composed SVG to PNG and records `dpi` in the PNG's pHYs chunk.
///
/// The canvas keeps its own coordinates; only the file's declared physical
/// resolution changes, so downstream tools can compute the printed size.
pub fn render_svg_to_png_with_dpi(input_svg: &Path, output_png: &Path, dpi: f64) -> Result<()> {
    render_svg_to_png_with_dpi_with_fonts(input_svg, output_png, dpi, &default_font_dir())
}

/// [`render_svg_to_png_with_dpi`] for an explicit font directory.
pub fn render_svg_to_png_with_dpi_with_fonts(
    input_svg: &Path,
    output_png: &Path,
    dpi: f64,
    font_dir: &Path,
) -> Result<()> {
    let pixmap = rasterize_svg(input_svg, font_dir)?;
    let mut png = pixmap.encode_png().map_err(|error| {
        SvgGridError::Raster(format!(
            "failed to encode PNG for {}: {error}",
            output_png.display()
        ))
    })?;
    set_png_physical_dpi(&mut png, dpi)?;
    fs::write(output_png, png).map_err(|error| SvgGridError::at(output_png, error))?;
    Ok(())
}

/// Inserts (or replaces) a `pHYs` chunk declaring `dpi` pixels per inch.
fn set_png_physical_dpi(png: &mut Vec<u8>, dpi: f64) -> Result<()> {
    if !dpi.is_finite() || dpi <= 0.0 {
        return Err(SvgGridError::Raster(format!(
            "png dpi must be a positive number, got {dpi}"
        )));
    }
    let pixels_per_metre = (dpi * PIXELS_PER_METRE_PER_DPI).round();
    if pixels_per_metre < 1.0 || pixels_per_metre > f64::from(u32::MAX) {
        return Err(SvgGridError::Raster(format!(
            "png dpi {dpi} is out of the supported range"
        )));
    }
    let pixels_per_metre = pixels_per_metre as u32;

    if png.get(..8) != Some(PNG_SIGNATURE.as_slice()) {
        return Err(SvgGridError::Raster(
            "rasterized output is not a PNG".to_string(),
        ));
    }
    if png.len() < IHDR_END || png.get(12..16) != Some(b"IHDR".as_slice()) {
        return Err(SvgGridError::Raster(
            "rasterized PNG does not start with an IHDR chunk".to_string(),
        ));
    }

    let mut chunk = Vec::with_capacity(21);
    chunk.extend_from_slice(&9u32.to_be_bytes());
    chunk.extend_from_slice(b"pHYs");
    chunk.extend_from_slice(&pixels_per_metre.to_be_bytes());
    chunk.extend_from_slice(&pixels_per_metre.to_be_bytes());
    chunk.push(1); // unit specifier: 1 = metre
    let mut crc = Crc::new();
    crc.update(&chunk[4..]);
    chunk.extend_from_slice(&crc.sum().to_be_bytes());

    let mut offset = IHDR_END;
    while offset + 12 <= png.len() {
        let Some(length_bytes) = png.get(offset..offset + 4) else {
            break;
        };
        let Ok(length) = <[u8; 4]>::try_from(length_bytes) else {
            break;
        };
        let end = offset + 12 + u32::from_be_bytes(length) as usize;
        if end > png.len() {
            break;
        }
        let kind = png.get(offset + 4..offset + 8);
        if kind == Some(b"pHYs".as_slice()) {
            png.splice(offset..end, chunk);
            return Ok(());
        }
        if kind == Some(b"IDAT".as_slice()) {
            break;
        }
        offset = end;
    }

    png.splice(IHDR_END..IHDR_END, chunk);
    Ok(())
}

pub fn render_svg_to_avif(input_svg: &Path, output_avif: &Path, quality: f32) -> Result<()> {
    render_svg_to_avif_with_fonts(input_svg, output_avif, quality, &default_font_dir())
}

/// [`render_svg_to_avif`] for an explicit font directory.
pub fn render_svg_to_avif_with_fonts(
    input_svg: &Path,
    output_avif: &Path,
    quality: f32,
    font_dir: &Path,
) -> Result<()> {
    // `ravif` asserts on an out-of-range quality, and that assertion would take the
    // user's own argument and turn it into `rc=101`. Checked here as a plain error
    // as well as at parse time: a library caller must not be able to panic either.
    if !quality.is_finite() || !(1.0..=100.0).contains(&quality) {
        return Err(SvgGridError::InvalidInput(format!(
            "--avif-quality must be between 1 and 100, got {quality}"
        )));
    }
    let pixmap = rasterize_svg(input_svg, font_dir)?;
    let pixels = pixmap_to_rgba(&pixmap);
    let encoded = Encoder::new()
        .with_quality(quality)
        .with_speed(10)
        .encode_rgba(Img::new(
            &pixels,
            pixmap.width() as usize,
            pixmap.height() as usize,
        ))
        .map_err(|error| {
            SvgGridError::Raster(format!(
                "failed to encode AVIF {}: {error}",
                output_avif.display()
            ))
        })?;
    fs::write(output_avif, encoded.avif_file)
        .map_err(|error| SvgGridError::at(output_avif, error))?;
    Ok(())
}

fn rasterize_svg(input_svg: &Path, font_dir: &Path) -> Result<Pixmap> {
    let svg = input::read_svg(input_svg)?;
    let mut options = usvg::Options::default();
    // Only `fontlib/` is loaded: the host's system fonts are deliberately out of
    // reach, so the same SVG rasterises identically on every machine.
    *options.fontdb_mut() = load_font_database(font_dir)?;
    // Text without a `font-family` gets the tool's own default (Arial), never the
    // renderer's built-in one.
    options.font_family = DEFAULT_FONT_FAMILY.to_string();
    let tree = usvg::Tree::from_data(svg.as_bytes(), &options).map_err(|error| {
        SvgGridError::Raster(format!(
            "failed to parse SVG {}: {error}",
            input_svg.display()
        ))
    })?;
    let size = tree.size().to_int_size();
    let mut pixmap = Pixmap::new(size.width(), size.height()).ok_or_else(|| {
        SvgGridError::Raster(format!(
            "failed to allocate raster buffer for {}x{} SVG",
            size.width(),
            size.height()
        ))
    })?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    Ok(pixmap)
}

fn pixmap_to_rgba(pixmap: &Pixmap) -> Vec<RGBA8> {
    pixmap
        .data()
        .chunks_exact(4)
        .map(|pixel| RGBA8::new(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect()
}
