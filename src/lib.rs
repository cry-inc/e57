//! A pure Rust library for reading E57 files without unsafe code.
//!
//! Some example code can be found [here](https://github.com/cry-inc/e57/tree/master/tools) in the GitHub repository.

#![forbid(unsafe_code)]

mod bitpack;
mod blob;
mod bounds;
mod byte_stream;
mod comp_vector;
mod crc32;
mod date_time;
mod e57;
mod error;
mod header;
mod images;
mod iterator;
mod limits;
mod paged_reader;
mod point;
mod pointcloud;
mod record;
mod root;
mod transform;
mod xml;

pub use self::blob::Blob;
pub use self::bounds::CartesianBounds;
pub use self::bounds::IndexBounds;
pub use self::bounds::SphericalBounds;
pub use self::date_time::DateTime;
pub use self::e57::E57;
pub use self::error::Error;
pub use self::error::Result;
pub use self::header::Header;
pub use self::images::CylindricalRepresentation;
pub use self::images::Image;
pub use self::images::ImageBlob;
pub use self::images::ImageFormat;
pub use self::images::PinholeRepresentation;
pub use self::images::Representation;
pub use self::images::SphericalRepresentation;
pub use self::images::VisualReference;
pub use self::iterator::PointCloudIterator;
pub use self::limits::ColorLimits;
pub use self::limits::IntensityLimits;
pub use self::limits::LimitValue;
pub use self::point::CartesianCoordinate;
pub use self::point::Color;
pub use self::point::Point;
pub use self::point::SphericalCoordinate;
pub use self::pointcloud::PointCloud;
pub use self::record::Record;
pub use self::record::RecordType;
pub use self::transform::Quaternion;
pub use self::transform::Transform;
pub use self::transform::Translation;
