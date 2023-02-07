#[derive(Debug)]
pub enum Error {
    InvalidFile(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidFile(reason) => write!(f, "Invalid file: {reason}"),
        }
    }
}

impl std::error::Error for Error {}
