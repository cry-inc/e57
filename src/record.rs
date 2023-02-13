use crate::error::Converter;
use crate::{Error, Result};
use roxmltree::Node;

#[derive(Debug, Clone)]
pub enum RecordType {
    Double { min: f64, max: f64 },
    Single { min: f64, max: f64 },
    Integer { min: i64, max: i64 },
}

#[derive(Debug, Clone)]
pub enum Record {
    CartesianX(RecordType),
    CartesianY(RecordType),
    CartesianZ(RecordType),
    CartesianInvalidState(RecordType),
}

pub fn record_type_from_node(node: &Node) -> Result<RecordType> {
    let type_string = node.attribute("type").invalid_err(format!(
        "Missing type attribute for prototype tag {}",
        node.tag_name().name()
    ))?;
    Ok(match type_string {
        "Float" => {
            let min = if let Some(min) = node.attribute("minimum") {
                min.parse::<f64>()
                    .invalid_err("Cannot parse minimum value of float type")?
            } else {
                f64::MIN
            };
            let max = if let Some(max) = node.attribute("maximum") {
                max.parse::<f64>()
                    .invalid_err("Cannot parse maximum value of float type")?
            } else {
                f64::MAX
            };

            let precision = node.attribute("precision").unwrap_or("double");
            if precision == "double" {
                RecordType::Double { min, max }
            } else if precision == "single" {
                RecordType::Single { min, max }
            } else {
                Error::invalid(format!(
                    "Float precision {precision} in prototype tag is unknown"
                ))?
            }
        }
        "Integer" => {
            let min = if let Some(min) = node.attribute("minimum") {
                min.parse::<i64>()
                    .invalid_err("Cannot parse minimum value of integer type")?
            } else {
                i64::MIN
            };
            let max = if let Some(max) = node.attribute("maximum") {
                max.parse::<i64>()
                    .invalid_err("Cannot parse maximum value of integer type")?
            } else {
                i64::MAX
            };
            RecordType::Integer { min, max }
        }
        _ => Error::not_implemented(format!("Unsupported record type {type_string} detected"))?,
    })
}
