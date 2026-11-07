// Raster previews: PNG and AVIF output of a composed SVG.

use std::fs;

use svg_grid::{render_svg_to_avif, render_svg_to_png};

use crate::common::*;

#[test]
fn render_svg_to_png_writes_a_png_preview() {
    if !fonts_available() {
        return;
    }
    let input = temp_path("raster_input.svg");
    let output = temp_path("raster_preview.png");
    fs::write(
        &input,
        r##"<svg width="64" height="48" viewBox="0 0 64 48" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="64" height="48" fill="#FFFFFF"/>
<rect x="8" y="8" width="48" height="32" fill="#336699"/>
</svg>"##,
    )
    .unwrap();

    render_svg_to_png(&input, &output).unwrap();

    let bytes = fs::read(&output).unwrap();
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn render_svg_to_png_preserves_text() {
    if !fonts_available() {
        return;
    }
    let input = temp_path("raster_text_input.svg");
    let output = temp_path("raster_text_preview.png");
    fs::write(
        &input,
        r##"<svg width="160" height="80" viewBox="0 0 160 80" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="160" height="80" fill="#FFFFFF"/>
<text x="20" y="50" font-family="sans-serif" font-size="40" fill="#000000">QC</text>
</svg>"##,
    )
    .unwrap();

    render_svg_to_png(&input, &output).unwrap();

    let file = fs::File::open(&output).unwrap();
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut buffer).unwrap();
    let bytes = &buffer[..frame.buffer_size()];
    let dark_pixels = bytes
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 64 && pixel[1] < 64 && pixel[2] < 64 && pixel[3] > 0)
        .count();
    assert!(
        dark_pixels > 20,
        "expected rendered text to produce dark pixels"
    );

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn render_svg_to_avif_writes_an_avif_preview() {
    if !fonts_available() {
        return;
    }
    let input = temp_path("raster_input_avif.svg");
    let output = temp_path("raster_preview.avif");
    fs::write(
        &input,
        r##"<svg width="64" height="48" viewBox="0 0 64 48" xmlns="http://www.w3.org/2000/svg">
<rect x="0" y="0" width="64" height="48" fill="#FFFFFF"/>
<circle cx="32" cy="24" r="12" fill="#CC5500"/>
</svg>"##,
    )
    .unwrap();

    render_svg_to_avif(&input, &output, 65.0).unwrap();

    let bytes = fs::read(&output).unwrap();
    assert!(bytes.len() > 32);
    assert_eq!(&bytes[4..8], b"ftyp");

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
