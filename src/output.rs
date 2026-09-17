use std::fs;
use std::io::Write;
use std::path::Path;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::Result;

pub fn write_outputs(svg: &str, svg_path: &Path, svgz_path: Option<&Path>) -> Result<()> {
    fs::write(svg_path, svg)?;
    if let Some(path) = svgz_path {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(svg.as_bytes())?;
        fs::write(path, encoder.finish()?)?;
    }
    Ok(())
}
