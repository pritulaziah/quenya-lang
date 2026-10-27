use std::fmt;

/// An error with a source line.
///
/// Line `0` means the error has no source line. A file read error uses it.
#[derive(Debug, Clone)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl Error {
    /// Create an error for a source line.
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for Error {}
