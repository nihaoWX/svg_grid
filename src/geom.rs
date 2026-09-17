use crate::dom::{Element, Node};
use crate::error::{Result, SvgGridError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn parse_f32(value: &str, attr: &str) -> Result<f32> {
    value.parse::<f32>().map_err(|_| {
        SvgGridError::InvalidInput(format!("invalid numeric SVG attribute {attr}={value:?}"))
    })
}

pub fn find_panel_box(root: &Element) -> Result<Rect> {
    find_panel_box_element(root)?.ok_or_else(|| {
        SvgGridError::InvalidInput(
            "input SVG must contain a <rect data-panel-box=\"main\" .../> element".to_string(),
        )
    })
}

pub fn find_canvas_box(root: &Element) -> Result<Rect> {
    if let Some(view_box) = root.attr("viewBox") {
        let values: Vec<_> = view_box.split_whitespace().collect();
        if values.len() == 4 {
            return Ok(Rect {
                x: parse_f32(values[0], "viewBox.x")?,
                y: parse_f32(values[1], "viewBox.y")?,
                width: parse_f32(values[2], "viewBox.width")?,
                height: parse_f32(values[3], "viewBox.height")?,
            });
        }
    }

    Ok(Rect {
        x: 0.0,
        y: 0.0,
        width: parse_f32(root.attr("width").unwrap_or("0"), "width")?,
        height: parse_f32(root.attr("height").unwrap_or("0"), "height")?,
    })
}

pub fn scale_panel_to_cell(panel: Rect, canvas: Rect, cell: Rect) -> Result<Rect> {
    if canvas.width <= 0.0 || canvas.height <= 0.0 {
        return Err(SvgGridError::InvalidInput(
            "input SVG canvas must have positive width and height".to_string(),
        ));
    }
    let sx = cell.width / canvas.width;
    let sy = cell.height / canvas.height;
    Ok(Rect {
        x: cell.x + (panel.x - canvas.x) * sx,
        y: cell.y + (panel.y - canvas.y) * sy,
        width: panel.width * sx,
        height: panel.height * sy,
    })
}

fn find_panel_box_element(element: &Element) -> Result<Option<Rect>> {
    if element.name == "rect" && element.attr("data-panel-box") == Some("main") {
        return Ok(Some(Rect {
            x: parse_f32(element.attr("x").unwrap_or("0"), "x")?,
            y: parse_f32(element.attr("y").unwrap_or("0"), "y")?,
            width: parse_f32(element.attr("width").unwrap_or("0"), "width")?,
            height: parse_f32(element.attr("height").unwrap_or("0"), "height")?,
        }));
    }

    for child in &element.children {
        if let Node::Element(child) = child {
            if let Some(rect) = find_panel_box_element(child)? {
                return Ok(Some(rect));
            }
        }
    }

    Ok(None)
}
