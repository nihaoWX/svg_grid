//! "Does the produced PNG carry any ink?"
//!
//! A run that exits 0 and writes a PNG can still have painted nothing at all — the
//! exact silent-ink-loss failure mode this tool exists to prevent (the corpus
//! caught it twice: round-data/R16's dropped paint servers, gradient_fill and
//! bars_grouped). Counting **non-white pixels** is the cheapest mechanical check
//! for it.

use std::fs;
use std::path::Path;

/// The 8-byte PNG signature.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Whether `bytes` starts with a PNG signature.
pub(crate) fn looks_like_png(bytes: &[u8]) -> bool {
    bytes.len() >= PNG_SIGNATURE.len() && bytes[..PNG_SIGNATURE.len()] == PNG_SIGNATURE
}

/// Count the pixels of `path` that are not pure white (`#FFFFFF`).
///
/// Every channel is lowered to 8 bit first (`png`'s `normalize_to_color8`), so the
/// count is comparable across colour types. A failure is a `String` sentence: the
/// caller turns it into a failed criterion, never into a panic.
pub(crate) fn ink_pixels(path: &Path) -> Result<u64, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if !looks_like_png(&bytes) {
        return Err(format!("{} is not a PNG file", path.display()));
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Palette / sub-8-bit frames are expanded to plain 8-bit samples, so the count
    // below only has to know about Rgb/Rgba/Grayscale(-Alpha).
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("cannot decode {}: {error}", path.display()))?;
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| format!("{} has an unknown decoded size", path.display()))?;
    let mut buffer = vec![0u8; buffer_size];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("cannot decode {}: {error}", path.display()))?;
    let pixels = &buffer[..info.buffer_size()];
    Ok(count_non_white(pixels, info.color_type, info.width))
}

/// Count non-white pixels of a packed 8-bit frame.
fn count_non_white(pixels: &[u8], color_type: png::ColorType, width: u32) -> u64 {
    let samples = color_type.samples();
    if samples == 0 || width == 0 {
        return 0;
    }
    let mut ink = 0u64;
    for pixel in pixels.chunks_exact(samples) {
        let dark = match color_type {
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha => pixel[0] != 255,
            png::ColorType::Rgb | png::ColorType::Rgba => {
                pixel[0] != 255 || pixel[1] != 255 || pixel[2] != 255
            }
            // A palette is expanded by `EXPAND`; an indexed frame that somehow gets
            // here is measured on its index, which is conservative (only index 255
            // would read as white) rather than silently blind.
            png::ColorType::Indexed => pixel[0] != 255,
        };
        if dark {
            ink += 1;
        }
    }
    ink
}

#[cfg(test)]
mod tests {
    use super::looks_like_png;

    #[test]
    fn only_the_png_signature_counts_as_a_png() {
        assert!(looks_like_png(b"\x89PNG\r\n\x1a\nrest"));
        assert!(!looks_like_png(b"<svg"));
        assert!(!looks_like_png(b""));
        assert!(!looks_like_png(b"\x89PNG\r\n\x1a"));
    }
}
