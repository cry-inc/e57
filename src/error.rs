use std::error::Error as StdError;
use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

/// Possible errors that can occur while working with E57 files.
#[derive(Debug)]
pub enum Error {
    /// The file content is invalid and does not confirm with the E57 format specification.
    InvalidFile {
        reason: String,
        source: Option<Box<dyn StdError>>,
    },
    /// Something went wrong while reading data from an E57 file
    Read {
        reason: String,
        source: Option<Box<dyn StdError>>,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Error::InvalidFile { reason, .. } => write!(f, "Invalid E57 file: {reason}"),
            Error::Read { reason, .. } => write!(f, "Failed to read E57: {reason}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::InvalidFile { source, .. } => source.as_ref().map(|s| s.as_ref()),
            Error::Read { source, .. } => source.as_ref().map(|s| s.as_ref()),
        }
    }
}

pub type Result<T> = StdResult<T, Error>;

pub fn invalid_file_err_str(reason: &str) -> Error {
    Error::InvalidFile {
        reason: reason.to_string(),
        source: None,
    }
}

pub fn invalid_file_err(reason: &str, source: impl StdError + 'static) -> Error {
    Error::InvalidFile {
        reason: reason.to_string(),
        source: Some(Box::new(source)),
    }
}

pub fn read_err(reason: &str, source: impl StdError + 'static) -> Error {
    Error::Read {
        reason: reason.to_string(),
        source: Some(Box::new(source)),
    }
}
