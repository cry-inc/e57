use crate::bounds::{cartesian_bounds_from_node, spherical_bounds_from_node};
use crate::record::record_type_from_node;
use crate::Error;
use crate::{error::Converter, CartesianBounds, Record, Result, SphericalBounds};
use roxmltree::Node;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PointCloud {
    pub guid: String,
    pub name: Option<String>,
    pub file_offset: u64,
    pub records: u64,
    pub prototype: Vec<Record>,
    pub cartesian_bounds: Option<CartesianBounds>,
    pub spherical_bounds: Option<SphericalBounds>,
}

pub fn extract_pointcloud(node: &Node) -> Result<PointCloud> {
    let guid = node
        .children()
        .find(|n| n.has_tag_name("guid") && n.attribute("type") == Some("String"))
        .invalid_err("Cannot find 'guid' tag inside 'data3D' child")?
        .text()
        .invalid_err("GUID tag is empty")?
        .to_string();

    let name = node
        .children()
        .find(|n| n.has_tag_name("name") && n.attribute("type") == Some("String"))
        .and_then(|n| n.text())
        .map(|t| t.to_string());

    let points_tag = node
        .children()
        .find(|n| n.has_tag_name("points") && n.attribute("type") == Some("CompressedVector"))
        .invalid_err("Cannot find 'points' tag inside 'data3D' child")?;

    let file_offset = points_tag
        .attribute("fileOffset")
        .invalid_err("Cannot find 'fileOffset' attribute in 'points' tag")?
        .parse::<u64>()
        .invalid_err("Cannot parse 'fileOffset' attribute value as u64")?;

    let records = points_tag
        .attribute("recordCount")
        .invalid_err("Cannot find 'recordCount' attribute in 'points' tag")?
        .parse::<u64>()
        .invalid_err("Cannot parse 'recordCount' attribute value as u64")?;

    let prototype_tag = points_tag
        .children()
        .find(|n| n.has_tag_name("prototype") && n.attribute("type") == Some("Structure"))
        .invalid_err("Cannot find 'prototype' child in 'points' tag")?;

    let mut prototype = Vec::new();
    for n in prototype_tag.children() {
        if n.is_element() {
            let tag_name = n.tag_name().name();
            match tag_name {
                "cartesianX" => prototype.push(Record::CartesianX(record_type_from_node(&n)?)),
                "cartesianY" => prototype.push(Record::CartesianY(record_type_from_node(&n)?)),
                "cartesianZ" => prototype.push(Record::CartesianZ(record_type_from_node(&n)?)),
                "cartesianInvalidState" => {
                    prototype.push(Record::CartesianInvalidState(record_type_from_node(&n)?))
                }
                "sphericalRange" => {
                    prototype.push(Record::SphericalRange(record_type_from_node(&n)?))
                }
                "sphericalAzimuth" => {
                    prototype.push(Record::SphericalAzimuth(record_type_from_node(&n)?))
                }
                "sphericalElevation" => {
                    prototype.push(Record::SphericalElevation(record_type_from_node(&n)?))
                }
                "sphericalInvalidState" => {
                    prototype.push(Record::SphericalInvalidState(record_type_from_node(&n)?))
                }
                "intensity" => prototype.push(Record::Intensity(record_type_from_node(&n)?)),
                "isIntensityInvalid" => {
                    prototype.push(Record::IsIntensityInvalid(record_type_from_node(&n)?))
                }
                "colorRed" => prototype.push(Record::ColorRed(record_type_from_node(&n)?)),
                "colorGreen" => prototype.push(Record::ColorGreen(record_type_from_node(&n)?)),
                "colorBlue" => prototype.push(Record::ColorBlue(record_type_from_node(&n)?)),
                "isColorInvalid" => {
                    prototype.push(Record::IsColorInvalid(record_type_from_node(&n)?))
                }
                "rowIndex" => prototype.push(Record::RowIndex(record_type_from_node(&n)?)),
                "columnIndex" => prototype.push(Record::ColumnIndex(record_type_from_node(&n)?)),
                "returnCount" => prototype.push(Record::ReturnCount(record_type_from_node(&n)?)),
                "returnIndex" => prototype.push(Record::ReturnIndex(record_type_from_node(&n)?)),
                "timeStamp" => prototype.push(Record::TimeStamp(record_type_from_node(&n)?)),
                "isTimeStampInvalid" => {
                    prototype.push(Record::IsTimeStampInvalid(record_type_from_node(&n)?))
                }
                tag => Error::not_implemented(format!(
                    "Found unsupported record named '{tag}' inside 'prototype'"
                ))?,
            }
        }
    }

    let cartesian_bounds = node
        .children()
        .find(|n| n.has_tag_name("cartesianBounds"))
        .map(|n| cartesian_bounds_from_node(&n));

    let spherical_bounds = node
        .children()
        .find(|n| n.has_tag_name("sphericalBounds"))
        .map(|n| spherical_bounds_from_node(&n));

    Ok(PointCloud {
        guid,
        name,
        file_offset,
        records,
        prototype,
        cartesian_bounds,
        spherical_bounds,
    })
}
