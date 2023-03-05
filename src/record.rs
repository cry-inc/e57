use crate::error::Converter;
use crate::{Error, Result};
use roxmltree::Node;

/// Basic primtive E57 data types that are used for the different point attributes.
#[derive(Debug, Clone)]
pub enum RecordType {
    /// 64-bit IEEE 754-2008 floating point value.
    Double,
    /// 32-bit IEEE 754-2008 floating point value.
    Single,
    /// Signed 64-bit integer value.
    Integer { min: i64, max: i64 },
    /// Signed 64-bit integer scaled with a fixed 64-bit floating point value.
    ScaledInteger { min: i64, max: i64, scale: f64 },
}

/// Used to describe the prototype records with all attributes that exit in the point cloud.
#[derive(Debug, Clone)]
pub enum Record {
    /// Cartesian X coordinate (in meters).
    CartesianX(RecordType),
    /// Cartesian Y coordinate (in meters).
    CartesianY(RecordType),
    /// Cartesian Z coordinate (in meters).
    CartesianZ(RecordType),
    /// Indicates whether the Cartesian coordinate or its magnitude is meaningful.
    /// Can have the value 0 (valid), 1 (XYZ is a direction vector) or 2 (invalid).
    CartesianInvalidState(RecordType),

    /// Non-negative range (in meters) of the spherical coordinate.
    SphericalRange(RecordType),
    /// Azimuth angle (in radians between -PI and PI) of the spherical coordinate.
    SphericalAzimuth(RecordType),
    // Elevation angle (in radians between -PI/2 and PI/2) of the spherical coordinate.
    SphericalElevation(RecordType),
    /// Indicates whether the spherical coordinate or its range is meaningful.
    /// Can have the value 0 (valid), 1 (range is not meaningful) or 2 (invalid).
    SphericalInvalidState(RecordType),

    /// Point intensity. Unit is not specified.
    Intensity(RecordType),
    /// Indicates whether the intensity value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsIntensityInvalid(RecordType),

    /// Red color value. Unit is not specified.
    ColorRed(RecordType),
    /// Green color value. Unit is not specified.
    ColorGreen(RecordType),
    /// Blue color value. Unit is not specified.
    ColorBlue(RecordType),
    /// Indicates whether the color value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsColorInvalid(RecordType),

    /// Row number of the point (zero-based). Used for data that is stored in a grid.
    RowIndex(RecordType),
    /// Column number of the point (zero-based). Used for data that is stored in a grid.
    ColumnIndex(RecordType),

    /// For multi-return sensors. The total number of returns for the pulse that this point corresponds to.
    ReturnCount(RecordType),
    /// For multi-return sensors. The number of this return (zero based). That is, 0 is the first, 1 is the second return etc.
    ReturnIndex(RecordType),

    /// Non-negative time (in seconds) since the start time given by acquisition start in the parent point cloud.
    TimeStamp(RecordType),
    /// Indicates whether the time stamp value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsTimeStampInvalid(RecordType),
}

pub fn record_type_from_node(node: &Node) -> Result<RecordType> {
    let type_string = node.attribute("type").invalid_err(format!(
        "Missing type attribute for tag {}",
        node.tag_name().name()
    ))?;
    Ok(match type_string {
        "Float" => {
            let precision = node.attribute("precision").unwrap_or("double");
            if precision == "double" {
                RecordType::Double
            } else if precision == "single" {
                RecordType::Single
            } else {
                Error::invalid(format!(
                    "Float 'precision' attribute value '{precision}' for 'Float' type is unknown"
                ))?
            }
        }
        "Integer" => {
            let min = node
                .attribute("minimum")
                .invalid_err("Cannot find 'minimum' attribute of 'Integer' type")?
                .parse::<i64>()
                .invalid_err("Cannot parse 'minimum' attribute of 'Integer' type as i64")?;
            let max = node
                .attribute("maximum")
                .invalid_err("Cannot find 'maximum' attribute of 'Integer' type")?
                .parse::<i64>()
                .invalid_err("Cannot parse 'maximum' attribute of 'Integer' type as i64")?;
            if max <= min {
                Error::invalid(format!(
                    "Maximum value {max} and minimum value {min} of 'Integer' type are invalid"
                ))?
            }
            RecordType::Integer { min, max }
        }
        "ScaledInteger" => {
            let min = node
                .attribute("minimum")
                .invalid_err("Cannot find 'minimum' attribute of 'ScaledInteger' type")?
                .parse::<i64>()
                .invalid_err("Cannot parse 'minimum' attribute of 'ScaledInteger' type as i64")?;
            let max = node
                .attribute("maximum")
                .invalid_err("Cannot find 'maximum' attribute of 'ScaledInteger' type")?
                .parse::<i64>()
                .invalid_err("Cannot parse 'maximum' attribute of 'ScaledInteger' type as i64")?;
            if max <= min {
                Error::invalid(format!("Maximum value {max} and minimum value {min} of 'ScaledInteger' type are invalid"))?
            }
            let scale = node
                .attribute("scale")
                .invalid_err("Cannot find 'scale' attribute of 'ScaledInteger' type")?
                .parse::<f64>()
                .invalid_err("Cannot parse 'scale' attribute of 'ScaledInteger' type as f64")?;
            RecordType::ScaledInteger { min, max, scale }
        }
        _ => Error::not_implemented(format!("Unsupported record type {type_string} detected"))?,
    })
}
