use crate::bounds::{
    cartesian_bounds_from_node, index_bounds_from_node, spherical_bounds_from_node,
};
use crate::error::Converter;
use crate::limits::{color_limits_from_node, intensity_limits_from_node};
use crate::record::record_type_from_node;
use crate::transform::transform_from_node;
use crate::{
    CartesianBounds, ColorLimits, Error, IndexBounds, IntensityLimits, Record, Result,
    SphericalBounds, Transform,
};
use roxmltree::{Document, Node};

/// Descriptor with metadata for a single point cloud.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PointCloud {
    /// Globally unique identifier for the point cloud.
    pub guid: String,
    /// User-defined name for the point cloud.
    pub name: Option<String>,
    /// Physical file offset of the start of the associated binary section.
    pub file_offset: u64,
    /// Number of points in the point cloud.
    pub records: u64,
    /// List of point attributes that exist for this point cloud.
    pub prototype: Vec<Record>,
    /// Optional Cartesian bounds for the point cloud.
    pub cartesian_bounds: Option<CartesianBounds>,
    /// Optional spherical bounds for the point cloud.
    pub spherical_bounds: Option<SphericalBounds>,
    /// Optional index bounds (row, column, return values) for the point cloud.
    pub index_bounds: Option<IndexBounds>,
    /// Optional intensity limits for the point cloud.
    pub intensity_limits: Option<IntensityLimits>,
    /// Optional color limits for the point cloud.
    pub color_limits: Option<ColorLimits>,
    /// Optional transformation to convert data from the local point cloud coordinates to the file-level coordinate system.
    pub transform: Option<Transform>,
}

pub fn pointclouds_from_document(document: &Document) -> Result<Vec<PointCloud>> {
    let data3d_node = document
        .descendants()
        .find(|n| n.has_tag_name("data3D"))
        .invalid_err("Cannot find 'data3D' tag in XML document")?;

    let mut data3d = Vec::new();
    for n in data3d_node.children() {
        if n.has_tag_name("vectorChild") && n.attribute("type") == Some("Structure") {
            let point_cloud = extract_pointcloud(&n)?;
            data3d.push(point_cloud);
        }
    }
    Ok(data3d)
}

fn extract_pointcloud(node: &Node) -> Result<PointCloud> {
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
    let index_bounds = node
        .children()
        .find(|n| n.has_tag_name("indexBounds"))
        .map(|n| index_bounds_from_node(&n));

    let intensity_limits = node.children().find(|n| n.has_tag_name("colorLimits"));
    let color_limits = node.children().find(|n| n.has_tag_name("colorLimits"));
    let transform = node.children().find(|n| n.has_tag_name("pose"));

    Ok(PointCloud {
        guid,
        name,
        file_offset,
        records,
        prototype,
        cartesian_bounds,
        spherical_bounds,
        index_bounds,
        intensity_limits: if let Some(node) = intensity_limits {
            Some(intensity_limits_from_node(&node)?)
        } else {
            None
        },
        color_limits: if let Some(node) = color_limits {
            Some(color_limits_from_node(&node)?)
        } else {
            None
        },
        transform: if let Some(node) = transform {
            Some(transform_from_node(&node)?)
        } else {
            None
        },
    })
}
