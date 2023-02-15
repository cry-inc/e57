use crate::error::Converter;
use crate::{Error, Result};
use roxmltree::Node;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RecordType {
    Double { min: f64, max: f64 },
    Single { min: f64, max: f64 },
    Integer { min: i64, max: i64 },
    ScaledInteger { min: i64, max: i64, scale: f64 },
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Record {
    CartesianX(RecordType),
    CartesianY(RecordType),
    CartesianZ(RecordType),
    CartesianInvalidState(RecordType),

    SphericalRange(RecordType),
    SphericalAzimuth(RecordType),
    SphericalElevation(RecordType),
    SphericalInvalidState(RecordType),

    Intensity(RecordType),
    IsIntensityInvalid(RecordType),

    ColorRed(RecordType),
    ColorGreen(RecordType),
    ColorBlue(RecordType),
    IsColorInvalid(RecordType),

    RowIndex(RecordType),
    ColumnIndex(RecordType),

    ReturnCount(RecordType),
    ReturnIndex(RecordType),

    TimeStamp(RecordType),
    IsTimeStampInvalid(RecordType),
}

pub fn record_type_from_node(node: &Node) -> Result<RecordType> {
    let type_string = node.attribute("type").invalid_err(format!(
        "Missing type attribute for prototype tag {}",
        node.tag_name().name()
    ))?;
    Ok(match type_string {
        "Float" => {
            let min = node
                .attribute("minimum")
                .invalid_err("Cannot find 'minimum' attribute of 'Float' type")?
                .parse::<f64>()
                .invalid_err("Cannot parse 'minimum' attribute of 'Float' type as f64")?;
            let max = node
                .attribute("maximum")
                .invalid_err("Cannot find 'maximum' attribute of 'Float' type")?
                .parse::<f64>()
                .invalid_err("Cannot parse 'maximum' attribute of 'Float' type as f64")?;
            if max <= min {
                Error::invalid(format!(
                    "Maximum value {max} and minimum value {min} of 'Float' type are invalid"
                ))?
            }
            let precision = node.attribute("precision").unwrap_or("double");
            if precision == "double" {
                RecordType::Double { min, max }
            } else if precision == "single" {
                RecordType::Single { min, max }
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
