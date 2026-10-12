use rapl::DimError;

/// Everything that can go wrong in `rapl-plot`.
///
/// Building marks and plots never fails; problems are found when a figure is
/// rendered, and the message names the mark or axis at fault.
#[derive(Debug)]
pub enum Error {
    /// A shape or rank mismatch reported by `rapl`.
    Dim(DimError),
    /// A mark's arrays do not describe the same points or series.
    Shape(String),
    /// An axis was configured inconsistently.
    Axis(String),
    /// The figure itself cannot be drawn, for example at a size of zero.
    Figure(String),
    /// Writing a file failed.
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Dim(e) => write!(f, "{e}"),
            Error::Shape(msg) | Error::Axis(msg) | Error::Figure(msg) => f.write_str(msg),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DimError> for Error {
    fn from(e: DimError) -> Self {
        Error::Dim(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
