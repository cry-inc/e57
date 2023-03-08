use crate::error::Converter;
use crate::{Error, Result};
use roxmltree::Node;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::str::FromStr;

/// Basic primtive E57 data types that are used for the different point attributes.
#[derive(Debug, Clone)]
pub enum RecordType {
    /// 64-bit IEEE 754-2008 floating point value.
    Double { min: Option<f64>, max: Option<f64> },
    /// 32-bit IEEE 754-2008 floating point value.
    Single { min: Option<f32>, max: Option<f32> },
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
    let tag_name = node.tag_name().name();
    let type_name = node
        .attribute("type")
        .invalid_err(format!("Missing type attribute for XML tag '{tag_name}'"))?;
    Ok(match type_name {
        "Float" => {
            let precision = node.attribute("precision").unwrap_or("double");
            if precision == "double" {
                let min = optional_attribute(node, "minimum", tag_name, type_name)?;
                let max = optional_attribute(node, "maximum", tag_name, type_name)?;
                RecordType::Double { min, max }
            } else if precision == "single" {
                let min = optional_attribute(node, "minimum", tag_name, type_name)?;
                let max = optional_attribute(node, "maximum", tag_name, type_name)?;
                RecordType::Single { min, max }
            } else {
                Error::invalid(format!(
                    "Float 'precision' attribute value '{precision}' for 'Float' type is unknown"
                ))?
            }
        }
        "Integer" => {
            let min = required_attribute(node, "minimum", tag_name, type_name)?;
            let max = required_attribute(node, "maximum", tag_name, type_name)?;
            if max <= min {
                Error::invalid(format!(
                    "Maximum value '{max}' and minimum value '{min}' of type '{type_name}' in XML tag '{tag_name}' are inconsistent"
                ))?
            }
            RecordType::Integer { min, max }
        }
        "ScaledInteger" => {
            let min = required_attribute(node, "minimum", tag_name, type_name)?;
            let max = required_attribute(node, "maximum", tag_name, type_name)?;
            if max <= min {
                Error::invalid(format!(
                    "Maximum value '{max}' and minimum value '{min}' of type '{type_name}' in XML tag '{tag_name}' are inconsistent"
                ))?
            }
            let scale = required_attribute(node, "scale", tag_name, type_name)?;
            RecordType::ScaledInteger { min, max, scale }
        }
        _ => Error::not_implemented(format!(
            "Unsupported type '{type_name}' in XML tag '{tag_name}' detected"
        ))?,
    })
}

fn optional_attribute<T>(
    node: &Node,
    attribute: &str,
    tag_name: &str,
    type_name: &str,
) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: StdError + Send + Sync + 'static,
{
    Ok(if let Some(attr) = node.attribute(attribute) {
        let parsed = attr.parse::<T>();
        Some(parsed.invalid_err(format!(
            "Failed to parse attribute '{attribute}' for type '{type_name}' in XML tag '{tag_name}'"
        ))?)
    } else {
        None
    })
}

fn required_attribute<T>(node: &Node, attribute: &str, tag_name: &str, type_name: &str) -> Result<T>
where
    T: FromStr,
    T::Err: StdError + Send + Sync + 'static,
{
    let value = optional_attribute(node, attribute, tag_name, type_name)?;
    value.invalid_err(format!(
        "Cannot find '{attribute}' for type '{type_name}' in XML tag '{tag_name}'"
    ))
}
