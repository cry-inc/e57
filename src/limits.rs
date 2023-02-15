use crate::record::record_type_from_node;
use crate::RecordType;
use crate::Result;
use roxmltree::Node;

fn extract_limit(bounds: &Node, tag_name: &str) -> Result<Option<RecordType>> {
    if let Some(tag) = bounds.descendants().find(|n| n.has_tag_name(tag_name)) {
        Ok(Some(record_type_from_node(&tag)?))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IntensityLimits {
    pub intensity_min: Option<RecordType>,
    pub intensity_max: Option<RecordType>,
}

pub fn intensity_limits_from_node(node: &Node) -> Result<IntensityLimits> {
    let intensity_min = extract_limit(node, "intensityMinimum")?;
    let intensity_max = extract_limit(node, "intenstiyMaximum")?;
    Ok(IntensityLimits {
        intensity_min,
        intensity_max,
    })
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ColorLimits {
    pub red_min: Option<RecordType>,
    pub red_max: Option<RecordType>,
    pub green_min: Option<RecordType>,
    pub green_max: Option<RecordType>,
    pub blue_min: Option<RecordType>,
    pub blue_max: Option<RecordType>,
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
