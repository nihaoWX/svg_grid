use std::fs;
use std::io::Write;
use std::path::Path;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::{Result, SvgGridError};

pub fn write_outputs(svg: &str, svg_path: &Path, svgz_path: Option<&Path>) -> Result<()> {
    fs::write(svg_path, svg).map_err(|error| SvgGridError::at(svg_path, error))?;
    if let Some(path) = svgz_path {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(svg.as_bytes())?;
        let bytes = encoder
            .finish()
            .map_err(|error| SvgGridError::at(path, error))?;
        fs::write(path, bytes).map_err(|error| SvgGridError::at(path, error))?;
    }
    Ok(())
}
