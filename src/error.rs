use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, SvgGridError>;

#[derive(Debug)]
pub enum SvgGridError {
    /// An I/O failure, carrying the file it happened on whenever the caller knows it.
    ///
    /// A single run reads several inputs and writes several outputs, so a bare
    /// `No such file or directory` leaves the reader unable to tell *which* path failed
    /// (`ARCHITECTURE/warnings.md` §1 rule 1: what / why / how to fix). Construct it with
    /// [`SvgGridError::at`] at every site that has a path.
    Io {
        path: Option<PathBuf>,
        source: std::io::Error,
    },
    Xml(quick_xml::Error),
    Raster(String),
    InvalidInput(String),
    /// The font library is missing / empty although this run needs glyphs
    /// (see `ARCHITECTURE/fonts.md`): a configuration error, never tolerated.
    Fonts(String),
}

impl SvgGridError {
    /// An I/O failure that **names the file** it happened on.
    pub fn at(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        SvgGridError::Io {
            path: Some(path.into()),
            source,
        }
    }

    /// The process exit code this error maps to (`ARCHITECTURE/cli.md`):
    /// `1` = runtime failure (I/O, XML, rasterisation), `2` = invalid usage or
    /// invalid input, `3` = configuration error (the font library).
    ///
    /// The tier lives on the error itself so no caller has to flatten it into a
    /// string and guess.
    pub fn exit_code(&self) -> u8 {
        match self {
            SvgGridError::Io { .. } | SvgGridError::Xml(_) | SvgGridError::Raster(_) => 1,
            SvgGridError::InvalidInput(_) => 2,
            SvgGridError::Fonts(_) => 3,
        }
    }
}

impl fmt::Display for SvgGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SvgGridError::Io {
                path: Some(path),
                source,
            } => write!(f, "I/O error at {}: {source}", path.display()),
            SvgGridError::Io { path: None, source } => write!(f, "I/O error: {source}"),
            SvgGridError::Xml(error) => write!(f, "SVG XML error: {error}"),
            SvgGridError::Raster(error) => write!(f, "rasterization error: {error}"),
            SvgGridError::InvalidInput(error) => write!(f, "invalid input: {error}"),
            SvgGridError::Fonts(error) => write!(f, "font library error: {error}"),
        }
    }
}

impl std::error::Error for SvgGridError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SvgGridError::Io { source, .. } => Some(source),
            SvgGridError::Xml(error) => Some(error),
            SvgGridError::Raster(_) => None,
            SvgGridError::InvalidInput(_) => None,
            SvgGridError::Fonts(_) => None,
        }
    }
}

impl From<std::io::Error> for SvgGridError {
    /// A failure with no path attached. Prefer [`SvgGridError::at`] wherever a path
    /// is at hand — this form is only for errors that have none (e.g. `current_exe`).
    fn from(source: std::io::Error) -> Self {
        SvgGridError::Io { path: None, source }
    }
}

impl From<quick_xml::Error> for SvgGridError {
    fn from(error: quick_xml::Error) -> Self {
        SvgGridError::Xml(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_io_error_names_the_file_it_happened_on() {
        let error = SvgGridError::at(
            "/nope/panel.svg",
            std::io::Error::new(std::io::ErrorKind::NotFound, "boom"),
        );
        assert_eq!(error.to_string(), "I/O error at /nope/panel.svg: boom");
        assert_eq!(error.exit_code(), 1);
    }

    #[test]
    fn an_io_error_without_a_path_keeps_the_short_shape() {
        let error = SvgGridError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "boom"));
        assert_eq!(error.to_string(), "I/O error: boom");
        assert_eq!(error.exit_code(), 1);
    }
}
