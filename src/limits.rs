use crate::error::Converter;
use crate::Error;
use crate::Result;
use roxmltree::Node;

/// Minimum or maximum value represented as Float, Integer or ScaledInteger.
#[derive(Clone, Debug, PartialEq)]
pub enum LimitValue {
    /// Floating point limit.
    Float(f64),
    /// Integer limit.
    Integer(i64),
    /// Scaled integer limit converted to a floating point value.
    ScaledInteger(f64),
}

fn extract_limit(bounds: &Node, tag_name: &str) -> Result<Option<LimitValue>> {
    if let Some(tag) = bounds.descendants().find(|n| n.has_tag_name(tag_name)) {
        let type_str = tag
            .attribute("type")
            .invalid_err(format!("Cannot find type attribute of limit '{tag_name}'"))?;
        let value_str = tag.text().unwrap_or("0");
        Ok(match type_str {
            "Integer" => Some(LimitValue::Integer(
                value_str
                    .parse::<i64>()
                    .invalid_err("Cannot parse Integer limit value")?,
            )),
            "Float" => Some(LimitValue::Float(
                value_str
                    .parse::<f64>()
                    .invalid_err("Cannot parse Integer limit value")?,
            )),
            _ => Error::not_implemented(format!(
                "Found unsupported limit of type '{type_str}' for '{tag_name}'"
            ))?,
        })
    } else {
        Ok(None)
    }
}

/// Optional minimum and maximum values for intensity.
#[derive(Clone, Debug)]
pub struct IntensityLimits {
    pub intensity_min: Option<LimitValue>,
    pub intensity_max: Option<LimitValue>,
}

pub fn intensity_limits_from_node(node: &Node) -> Result<IntensityLimits> {
    let intensity_min = extract_limit(node, "intensityMinimum")?;
    let intensity_max = extract_limit(node, "intenstiyMaximum")?;
    Ok(IntensityLimits {
        intensity_min,
        intensity_max,
    })
}

/// Optional minimum and maximum values for the colors red, green and blue.
#[derive(Clone, Debug)]
pub struct ColorLimits {
    pub red_min: Option<LimitValue>,
    pub red_max: Option<LimitValue>,
    pub green_min: Option<LimitValue>,
    pub green_max: Option<LimitValue>,
    pub blue_min: Option<LimitValue>,
    pub blue_max: Option<LimitValue>,
}

pub fn color_limits_from_node(node: &Node) -> Result<ColorLimits> {
    let red_min = extract_limit(node, "colorRedMinimum")?;
    let red_max = extract_limit(node, "colorRedMaximum")?;
    let green_min = extract_limit(node, "colorGreenMinimum")?;
    let green_max = extract_limit(node, "colorGreenMaximum")?;
    let blue_min = extract_limit(node, "colorBlueMinimum")?;
    let blue_max = extract_limit(node, "colorBlueMaximum")?;
    Ok(ColorLimits {
        red_min,
        red_max,
        green_min,
        green_max,
        blue_min,
        blue_max,
    })
}
