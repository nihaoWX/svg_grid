use std::fs::File;
use std::io::Read;
use std::path::Path;

use flate2::read::GzDecoder;

use crate::Result;

pub fn read_svg(path: &Path) -> Result<String> {
    let mut svg = String::new();
    if path
        .extension()
        .is_some_and(|extension| extension == "svgz")
    {
        GzDecoder::new(File::open(path)?).read_to_string(&mut svg)?;
    } else {
        File::open(path)?.read_to_string(&mut svg)?;
    }
    Ok(svg)
}
