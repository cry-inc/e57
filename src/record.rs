use crate::bs_write::ByteStreamWriteBuffer;
use crate::error::Converter;
use crate::{Error, Result};
use roxmltree::Node;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::str::FromStr;

/// Describes a record inside a E57 file with name and data type.
#[derive(Clone, Debug)]
pub struct Record {
    pub name: RecordName,
    pub data_type: RecordDataType,
}

/// Basic primtive E57 data types that are used for the different point attributes.
#[derive(Clone, Debug)]
pub enum RecordDataType {
    /// 32-bit IEEE 754-2008 floating point value.
    Single { min: Option<f32>, max: Option<f32> },
    /// 64-bit IEEE 754-2008 floating point value.
    Double { min: Option<f64>, max: Option<f64> },
    /// Signed 64-bit integer scaled with a fixed 64-bit floating point value.
    ScaledInteger { min: i64, max: i64, scale: f64 },
    /// Signed 64-bit integer value.
    Integer { min: i64, max: i64 },
}

/// Used to describe the prototype records with all attributes that exit in the point cloud.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RecordName {
    /// Cartesian X coordinate (in meters).
    CartesianX,
    /// Cartesian Y coordinate (in meters).
    CartesianY,
    /// Cartesian Z coordinate (in meters).
    CartesianZ,
    /// Indicates whether the Cartesian coordinate or its magnitude is meaningful.
    /// Can have the value 0 (valid), 1 (XYZ is a direction vector) or 2 (invalid).
    CartesianInvalidState,

    /// Non-negative range (in meters) of the spherical coordinate.
    SphericalRange,
    /// Azimuth angle (in radians between -PI and PI) of the spherical coordinate.
    SphericalAzimuth,
    // Elevation angle (in radians between -PI/2 and PI/2) of the spherical coordinate.
    SphericalElevation,
    /// Indicates whether the spherical coordinate or its range is meaningful.
    /// Can have the value 0 (valid), 1 (range is not meaningful) or 2 (invalid).
    SphericalInvalidState,

    /// Point intensity. Unit is not specified.
    Intensity,
    /// Indicates whether the intensity value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsIntensityInvalid,

    /// Red color value. Unit is not specified.
    ColorRed,
    /// Green color value. Unit is not specified.
    ColorGreen,
    /// Blue color value. Unit is not specified.
    ColorBlue,
    /// Indicates whether the color value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsColorInvalid,

    /// Row number of the point (zero-based). Used for data that is stored in a grid.
    RowIndex,
    /// Column number of the point (zero-based). Used for data that is stored in a grid.
    ColumnIndex,

    /// For multi-return sensors. The total number of returns for the pulse that this point corresponds to.
    ReturnCount,
    /// For multi-return sensors. The number of this return (zero based). That is, 0 is the first, 1 is the second return etc.
    ReturnIndex,

    /// Non-negative time (in seconds) since the start time given by acquisition start in the parent point cloud.
    TimeStamp,
    /// Indicates whether the time stamp value is meaningful.
    /// Can have the value 0 (valid) or 1 (invalid).
    IsTimeStampInvalid,
}

/// Represents a raw value of records inside a point cloud.
///
/// For scaled integers the record data type with the scale is needed to calulcate the actual f64 value.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordValue {
    Single(f32),
    Double(f64),
    ScaledInteger(i64),
    Integer(i64),
}

impl Record {
    pub(crate) fn serialize(&self) -> String {
        let tag_name = self.name.to_tag_name();
        let type_attrs = serialize_record_type(&self.data_type);
        format!("<{tag_name} {type_attrs}/>\n")
    }
}

impl RecordName {
    pub(crate) fn to_tag_name(&self) -> String {
        String::from(match self {
            RecordName::CartesianX => "cartesianX",
            RecordName::CartesianY => "cartesianY",
            RecordName::CartesianZ => "cartesianZ",
            RecordName::CartesianInvalidState => "cartesianInvalidState",
            RecordName::SphericalRange => "sphericalRange",
            RecordName::SphericalAzimuth => "sphericalAzimuth",
            RecordName::SphericalElevation => "sphericalElevation",
            RecordName::SphericalInvalidState => "sphericalInvalidState",
            RecordName::Intensity => "intensity",
            RecordName::IsIntensityInvalid => "isIntensityInvalid",
            RecordName::ColorRed => "colorRed",
            RecordName::ColorGreen => "colorGreen",
            RecordName::ColorBlue => "colorBlue",
            RecordName::IsColorInvalid => "isColorInvalid",
            RecordName::RowIndex => "rowIndex",
            RecordName::ColumnIndex => "columnIndex",
            RecordName::ReturnCount => "returnCount",
            RecordName::ReturnIndex => "returnIndex",
            RecordName::TimeStamp => "timeStamp",
            RecordName::IsTimeStampInvalid => "isTimeStampInvalid",
        })
    }

    pub(crate) fn from_tag_name(value: &str) -> Result<Self> {
        Ok(match value {
            "cartesianX" => RecordName::CartesianX,
            "cartesianY" => RecordName::CartesianY,
            "cartesianZ" => RecordName::CartesianZ,
            "cartesianInvalidState" => RecordName::CartesianInvalidState,
            "sphericalRange" => RecordName::SphericalRange,
            "sphericalAzimuth" => RecordName::SphericalAzimuth,
            "sphericalElevation" => RecordName::SphericalElevation,
            "sphericalInvalidState" => RecordName::SphericalInvalidState,
            "intensity" => RecordName::Intensity,
            "isIntensityInvalid" => RecordName::IsIntensityInvalid,
            "colorRed" => RecordName::ColorRed,
            "colorGreen" => RecordName::ColorGreen,
            "colorBlue" => RecordName::ColorBlue,
            "isColorInvalid" => RecordName::IsColorInvalid,
            "rowIndex" => RecordName::RowIndex,
            "columnIndex" => RecordName::ColumnIndex,
            "returnCount" => RecordName::ReturnCount,
            "returnIndex" => RecordName::ReturnIndex,
            "timeStamp" => RecordName::TimeStamp,
            "isTimeStampInvalid" => RecordName::IsTimeStampInvalid,
            name => Error::not_implemented(format!("Found unknown record name: '{name}'"))?,
        })
    }
}

impl RecordDataType {
    pub(crate) fn from_node(node: &Node) -> Result<Self> {
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
                    RecordDataType::Double { min, max }
                } else if precision == "single" {
                    let min = optional_attribute(node, "minimum", tag_name, type_name)?;
                    let max = optional_attribute(node, "maximum", tag_name, type_name)?;
                    RecordDataType::Single { min, max }
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
                RecordDataType::Integer { min, max }
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
                RecordDataType::ScaledInteger { min, max, scale }
            }
            _ => Error::not_implemented(format!(
                "Unsupported type '{type_name}' in XML tag '{tag_name}' detected"
            ))?,
        })
    }

    pub(crate) fn bit_size(&self) -> usize {
        match self {
            RecordDataType::Single { .. } => std::mem::size_of::<f32>() * 8,
            RecordDataType::Double { .. } => std::mem::size_of::<f64>() * 8,
            RecordDataType::ScaledInteger { min, max, .. } => integer_bits(*min, *max),
            RecordDataType::Integer { min, max } => integer_bits(*min, *max),
        }
    }

    pub(crate) fn write(
        &self,
        value: &RecordValue,
        buffer: &mut ByteStreamWriteBuffer,
    ) -> Result<()> {
        match self {
            RecordDataType::Single { .. } => {
                if let RecordValue::Single(float) = value {
                    let bytes = float.to_le_bytes();
                    buffer.add_bytes(&bytes);
                } else {
                    Error::invalid("Data type single only supports single values")?
                }
            }
            RecordDataType::Double { .. } => {
                if let RecordValue::Double(double) = value {
                    let bytes = double.to_le_bytes();
                    buffer.add_bytes(&bytes);
                } else {
                    Error::invalid("Data type double only supports double values")?
                }
            }
            RecordDataType::ScaledInteger { .. } => {
                Error::not_implemented("Scaled integer serialization is not yet supported")?
            }
            RecordDataType::Integer { min, max } => {
                if let RecordValue::Integer(int) = value {
                    let bit_size = integer_bits(*min, *max);
                    if bit_size % 8 != 0 || bit_size > 64 {
                        Error::not_implemented("Only bit sizes with a multiple of 8 and up to 64 are currently supported")?
                    }
                    let byte_size = bit_size / 8;
                    let uint = (int - min) as u64;
                    let bytes = uint.to_le_bytes();
                    buffer.add_bytes(&bytes[..byte_size]);
                } else {
                    Error::invalid("Data type integer only supports integer values")?
                }
            }
        };
        Ok(())
    }
}

#[inline]
fn integer_bits(min: i64, max: i64) -> usize {
    let range = max - min;
    f64::ceil(f64::log2(range as f64 + 1.0)) as usize
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

fn serialize_record_type(rt: &RecordDataType) -> String {
    match rt {
        RecordDataType::Single { min, max } => {
            let mut str = String::from("type=\"Float\" precision=\"Single\"");
            if let Some(min) = min {
                str += &format!(" minimum=\"{min}\"");
            }
            if let Some(max) = max {
                str += &format!(" maximum=\"{max}\"");
            }
            str
        }
        RecordDataType::Double { min, max } => {
            let mut str = String::from("type=\"Float\"");
            if let Some(min) = min {
                str += &format!(" minimum=\"{min}\"");
            }
            if let Some(max) = max {
                str += &format!(" maximum=\"{max}\"");
            }
            str
        }
        RecordDataType::ScaledInteger { min, max, scale } => {
            format!("type=\"ScaledInteger\" minimum=\"{min}\" maximum=\"{max}\"  scale=\"{scale}\"")
        }
        RecordDataType::Integer { min, max } => {
            format!("type=\"Integer\" minimum=\"{min}\" maximum=\"{max}\"")
        }
    }
}
