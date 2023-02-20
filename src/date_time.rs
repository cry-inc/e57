use crate::{error::Converter, Result};
use roxmltree::Node;

/// Represents a specific date and time used in E57 files.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DateTime {
    /// Number of seconds since GPS start epoch (00:00 UTC on January 6, 1980).
    pub gps_time: f64,
    /// True if the a global navigation satellite system device (such as GPS or GLONASS) was used to record the time.
    pub atomic_reference: bool,
}

pub fn date_time_from_node(node: &Node) -> Result<DateTime> {
    let gps_time = node
        .children()
        .find(|n| n.has_tag_name("dateTimeValue") && n.attribute("type") == Some("Float"))
        .invalid_err("Unable to find tag 'dateTimeValue' with type 'Float'")?
        .text()
        .invalid_err("Unable to read 'dateTimeValue' text")?
        .parse::<f64>()
        .invalid_err("Failed to parse 'dateTimeValue' text as f64")?;

    let atomic_reference = node
        .children()
        .find(|n| {
            n.has_tag_name("isAtomicClockReferenced") && n.attribute("type") == Some("Integer")
        })
        .invalid_err("Unable to find tag 'isAtomicClockReferenced' with type 'Integer'")?
        .text()
        .unwrap_or("0")
        .trim()
        == "1";

    Ok(DateTime {
        gps_time,
        atomic_reference,
    })
}
