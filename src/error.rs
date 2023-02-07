#[derive(Debug)]
pub enum Error {
    InvalidFile(String),
    Read(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidFile(reason) => write!(f, "Invalid file: {reason}"),
            Error::Read(reason) => write!(f, "Failed to read E57: {reason}"),
        }
    }
}

impl std::error::Error for Error {}
