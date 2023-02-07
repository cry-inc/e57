#![forbid(unsafe_code)]

mod error;
mod header;
mod paged_reader;

pub trait ReadSeek: std::io::Read + std::io::Seek {}
impl<T: std::io::Read + std::io::Seek> ReadSeek for T {}

pub use error::Error;
pub use header::Header;
