use std::fs::File;
use std::io::Read;
use std::path::Path;

use flate2::read::GzDecoder;

use crate::{Result, SvgGridError};

pub fn read_svg(path: &Path) -> Result<String> {
    let mut svg = String::new();
    if path
        .extension()
        .is_some_and(|extension| extension == "svgz")
    {
        let file = File::open(path).map_err(|error| SvgGridError::at(path, error))?;
        GzDecoder::new(file)
            .read_to_string(&mut svg)
            .map_err(|error| SvgGridError::at(path, error))?;
    } else {
        let mut file = File::open(path).map_err(|error| SvgGridError::at(path, error))?;
        file.read_to_string(&mut svg)
            .map_err(|error| SvgGridError::at(path, error))?;
    }
    Ok(svg)
}
