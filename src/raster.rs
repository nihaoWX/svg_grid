use std::fs;
use std::path::Path;

use flate2::Crc;
use ravif::{Encoder, Img, RGBA8};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::{input, Result, SvgGridError};

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// IHDR is required by the PNG spec to be the first chunk and to be 13 bytes long.
const IHDR_END: usize = 8 + 4 + 4 + 13 + 4;
const PIXELS_PER_METRE_PER_DPI: f64 = 1.0 / 0.0254;

pub fn render_svg_to_png(input_svg: &Path, output_png: &Path) -> Result<()> {
    let pixmap = rasterize_svg(input_svg)?;
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
    let pixmap = rasterize_svg(input_svg)?;
    let mut png = pixmap.encode_png().map_err(|error| {
        SvgGridError::Raster(format!(
            "failed to encode PNG for {}: {error}",
            output_png.display()
        ))
    })?;
    set_png_physical_dpi(&mut png, dpi)?;
    fs::write(output_png, png).map_err(|error| {
        SvgGridError::Raster(format!(
            "failed to write PNG {}: {error}",
            output_png.display()
        ))
    })?;
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
    let pixmap = rasterize_svg(input_svg)?;
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
    fs::write(output_avif, encoded.avif_file)?;
    Ok(())
}

fn rasterize_svg(input_svg: &Path) -> Result<Pixmap> {
    let svg = input::read_svg(input_svg)?;
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
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
