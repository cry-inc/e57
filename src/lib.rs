#![forbid(unsafe_code)]

mod e57;
mod error;
mod header;
mod paged_reader;

pub trait ReadSeek: std::io::Read + std::io::Seek {}
impl<T: std::io::Read + std::io::Seek> ReadSeek for T {}

pub use self::e57::E57;
pub use self::error::Error;
pub use self::header::Header;
