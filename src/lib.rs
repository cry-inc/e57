//! A pure Rust library for reading E57 files without unsafe code.
//!
//! Some example code can be found [here](https://github.com/cry-inc/e57/tree/master/tools) in the GitHub repository.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value
)]

mod bitpack;
mod blob;
mod bounds;
mod bs_read;
mod bs_write;
mod crc32;
mod cv_section;
mod date_time;
mod e57_reader;
mod e57_writer;
mod error;
mod header;
mod image_writer;
mod images;
mod limits;
mod packet;
mod paged_reader;
mod paged_writer;
mod pc_reader;
mod pc_writer;
mod point;
mod pointcloud;
mod record;
mod root;
mod transform;
mod xml;

// Public types
pub use self::blob::Blob;
pub use self::bounds::CartesianBounds;
pub use self::bounds::IndexBounds;
pub use self::bounds::SphericalBounds;
pub use self::date_time::DateTime;
pub use self::e57_reader::E57Reader;
pub use self::e57_writer::E57Writer;
pub use self::error::Error;
pub use self::error::Result;
pub use self::header::Header;
pub use self::image_writer::ImageWriter;
pub use self::images::CylindricalImage;
pub use self::images::CylindricalImageProperties;
pub use self::images::Image;
pub use self::images::ImageBlob;
pub use self::images::ImageFormat;
pub use self::images::PinholeImage;
pub use self::images::PinholeImageProperties;
pub use self::images::Projection;
pub use self::images::SphericalImage;
pub use self::images::SphericalImageProperties;
pub use self::images::VisualReferenceImage;
pub use self::images::VisualReferenceImageProperties;
pub use self::limits::ColorLimits;
pub use self::limits::IntensityLimits;
pub use self::pc_reader::PointCloudReader;
pub use self::pc_writer::PointCloudWriter;
pub use self::point::CartesianCoordinate;
pub use self::point::Color;
pub use self::point::Point;
pub use self::point::SphericalCoordinate;
pub use self::pointcloud::PointCloud;
pub use self::record::Record;
pub use self::record::RecordDataType;
pub use self::record::RecordName;
pub use self::record::RecordValue;
pub use self::transform::Quaternion;
pub use self::transform::Transform;
pub use self::transform::Translation;

/// Storage container for a low level point data.
pub type RawValues = Vec<RecordValue>;
