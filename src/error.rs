use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

/// Possible errors that can occur while working with E57 files.
#[derive(Debug)]
pub enum Error {
    /// The file content is invalid and does not confirm with the E57 format specification.
    Invalid {
        reason: String,
        source: Option<Box<dyn StdError>>,
    },
    /// Something went wrong while reading data from an E57 file.
    /// Typically this is caused by an IO error outside the library or because of an incomplete file.
    Read {
        reason: String,
        source: Option<Box<dyn StdError>>,
    },
    /// An unexpected internal issue occured.
    /// Most likely this is a logic inside the library.
    /// Please file an issue, if possible.
    Internal {
        reason: String,
        source: Option<Box<dyn StdError>>,
    },
}

impl Error {
    /// Creates an invalid file error from text.
    pub fn invalid<T>(reason: &str) -> Result<T> {
        Err(Error::Invalid {
            reason: reason.to_string(),
            source: None,
        })
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Error::Invalid { reason, .. } => write!(f, "Invalid E57 file: {reason}"),
            Error::Read { reason, .. } => write!(f, "Failed to read E57: {reason}"),
            Error::Internal { reason, .. } => write!(f, "Internal error: {reason}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Invalid { source, .. } => source.as_ref().map(|s| s.as_ref()),
            Error::Read { source, .. } => source.as_ref().map(|s| s.as_ref()),
            Error::Internal { source, .. } => source.as_ref().map(|s| s.as_ref()),
        }
    }
}

pub type Result<T> = StdResult<T, Error>;

/// Helper trait for types that can be converted into an Error.
pub trait ErrorConverter<T, E> {
    fn read_err<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;

    fn invalid_err<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;

    fn internal_err<C>(self, context: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;
}

/// Create an library Error from std Error instances.
impl<T, E> ErrorConverter<T, E> for StdResult<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn read_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(Error::Read {
                reason: reason.to_string(),
                source: Some(Box::new(error)),
            }),
        }
    }

    fn invalid_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(Error::Invalid {
                reason: reason.to_string(),
                source: Some(Box::new(error)),
            }),
        }
    }

    fn internal_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(Error::Internal {
                reason: reason.to_string(),
                source: Some(Box::new(error)),
            }),
        }
    }
}

/// Create an library Error from Option instances.
impl<T> ErrorConverter<T, Infallible> for Option<T> {
    fn read_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(ok) => Ok(ok),
            None => Err(Error::Read {
                reason: reason.to_string(),
                source: None,
            }),
        }
    }

    fn invalid_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(ok) => Ok(ok),
            None => Err(Error::Invalid {
                reason: reason.to_string(),
                source: None,
            }),
        }
    }

    fn internal_err<C>(self, reason: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(ok) => Ok(ok),
            None => Err(Error::Internal {
                reason: reason.to_string(),
                source: None,
            }),
        }
    }
}
