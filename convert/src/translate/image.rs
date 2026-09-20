//! `<image>` baking.
//!
//! A pure translation and any *axis-aligned* scale — uniform or not, optionally
//! with a flip — are baked into the box `x`/`y`/`width`/`height` (a non-uniform
//! scale simply stretches the bitmap box, which loses nothing). A negative scale
//! additionally decodes the embedded PNG, flips it and re-encodes. Rotation and
//! skew are rejected (fail closed).

use crate::affine::Affine;
use crate::base64;
use crate::dom::Element;
use crate::error::ConvertError;
use crate::transform::fmt_num;

use super::number;

pub(super) fn bake_image(element: &mut Element, transform: Affine) -> Result<(), ConvertError> {
    let (sx, sy, _, _) = transform.as_axis_scale_translate().ok_or_else(|| {
        ConvertError::Translate(
            "<image> transform contains rotation/skew; only an axis-aligned scale (possibly with a \
             flip) or a translation can be baked"
                .to_string(),
        )
    })?;

    let x = number(element, "x")?;
    let y = number(element, "y")?;
    let width = number(element, "width")?;
    let height = number(element, "height")?;
    let (x0, y0) = transform.apply(x, y);
    let (x1, y1) = transform.apply(x + width, y + height);
    element.set_attr("x", fmt_num(x0.min(x1)));
    element.set_attr("y", fmt_num(y0.min(y1)));
    element.set_attr("width", fmt_num((x1 - x0).abs()));
    element.set_attr("height", fmt_num((y1 - y0).abs()));
    element.set_attr("preserveAspectRatio", "none");

    if sx < 0.0 || sy < 0.0 {
        let (name, href) = match element.attr("xlink:href") {
            Some(href) => ("xlink:href", href.to_string()),
            None => (
                "href",
                element
                    .attr("href")
                    .ok_or_else(|| {
                        ConvertError::Translate("<image> has no href to bake".to_string())
                    })?
                    .to_string(),
            ),
        };
        let baked = flip_data_uri(&href, sx < 0.0, sy < 0.0)?;
        element.set_attr(name, baked);
    }
    Ok(())
}

fn flip_data_uri(href: &str, flip_x: bool, flip_y: bool) -> Result<String, ConvertError> {
    let payload = href
        .split_once(',')
        .map(|(_, payload)| payload)
        .ok_or_else(|| {
            ConvertError::Translate(format!("<image> href is not a data URI: {href:?}"))
        })?;
    if !href[..href.find(',').unwrap_or(0)].contains("base64") {
        return Err(ConvertError::Translate(
            "<image> data URI is not base64 encoded".to_string(),
        ));
    }
    let bytes = base64::decode(payload).ok_or_else(|| {
        ConvertError::Translate("<image> data URI is not valid base64".to_string())
    })?;
    let flipped = flip_png(&bytes, flip_x, flip_y)?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::encode(&flipped)
    ))
}

fn flip_png(bytes: &[u8], flip_x: bool, flip_y: bool) -> Result<Vec<u8>, ConvertError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|error| {
        ConvertError::Translate(format!("failed to decode the embedded PNG: {error}"))
    })?;
    let buffer_size = reader.output_buffer_size().ok_or_else(|| {
        ConvertError::Translate("embedded PNG has an unknown output size".to_string())
    })?;
    let mut buffer = vec![0u8; buffer_size];
    let info = reader.next_frame(&mut buffer).map_err(|error| {
        ConvertError::Translate(format!("failed to read the embedded PNG frame: {error}"))
    })?;

    let width = info.width as usize;
    let height = info.height as usize;
    let samples = info.color_type.samples();
    let row = width * samples;
    let source = &buffer[..info.buffer_size()];

    let mut out = vec![0u8; source.len()];
    for y in 0..height {
        let source_y = if flip_y { height - 1 - y } else { y };
        let source_row = &source[source_y * row..source_y * row + row];
        let destination_row = &mut out[y * row..y * row + row];
        if flip_x {
            for x in 0..width {
                destination_row[x * samples..(x + 1) * samples]
                    .copy_from_slice(&source_row[(width - 1 - x) * samples..(width - x) * samples]);
            }
        } else {
            destination_row.copy_from_slice(source_row);
        }
    }

    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, info.width, info.height);
        encoder.set_color(info.color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| {
            ConvertError::Translate(format!("failed to re-encode the embedded PNG: {error}"))
        })?;
        writer.write_image_data(&out).map_err(|error| {
            ConvertError::Translate(format!("failed to re-encode the embedded PNG: {error}"))
        })?;
        writer.finish().map_err(|error| {
            ConvertError::Translate(format!("failed to finish the embedded PNG: {error}"))
        })?;
    }
    Ok(encoded)
}
