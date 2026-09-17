use std::fmt;

pub type Result<T> = std::result::Result<T, SvgGridError>;

#[derive(Debug)]
pub enum SvgGridError {
    Io(std::io::Error),
    Xml(quick_xml::Error),
    Raster(String),
    InvalidInput(String),
}

impl fmt::Display for SvgGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SvgGridError::Io(error) => write!(f, "I/O error: {error}"),
            SvgGridError::Xml(error) => write!(f, "SVG XML error: {error}"),
            SvgGridError::Raster(error) => write!(f, "rasterization error: {error}"),
            SvgGridError::InvalidInput(error) => write!(f, "invalid input: {error}"),
        }
    }
}

impl std::error::Error for SvgGridError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SvgGridError::Io(error) => Some(error),
            SvgGridError::Xml(error) => Some(error),
            SvgGridError::Raster(_) => None,
            SvgGridError::InvalidInput(_) => None,
        }
    }
}

impl From<std::io::Error> for SvgGridError {
    fn from(error: std::io::Error) -> Self {
        SvgGridError::Io(error)
    }
}

impl From<quick_xml::Error> for SvgGridError {
    fn from(error: quick_xml::Error) -> Self {
        SvgGridError::Xml(error)
    }
}
