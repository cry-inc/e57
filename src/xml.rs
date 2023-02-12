use crate::error::{Converter, Error};
use crate::record::record_type_from_node;
use crate::{PointCloud, Record, Result};
use roxmltree::{Document, Node};

pub struct XmlDocument {
    xml: String,
    format: Option<String>,
    guid: Option<String>,
    pointclouds: Vec<PointCloud>,
}

impl XmlDocument {
    pub fn parse(xml: String) -> Result<Self> {
        let document = Document::parse(&xml).invalid_err("Failed to parse XML document")?;
        let format = extract_string(&document, "formatName");
        let guid = extract_string(&document, "guid");
        let data3d = extract_pointclouds(&document)?;
        Ok(Self {
            xml,
            format,
            guid,
            pointclouds: data3d,
        })
    }

    pub fn format_name(&self) -> Option<&String> {
        self.format.as_ref()
    }

    pub fn guid(&self) -> Option<&String> {
        self.guid.as_ref()
    }

    pub fn raw_xml(&self) -> &str {
        &self.xml
    }

    pub fn pointclouds(&self) -> Vec<PointCloud> {
        self.pointclouds.clone()
    }
}

fn extract_string(document: &Document, tag_name: &str) -> Option<String> {
    document
        .descendants()
        .find(|n| n.has_tag_name(tag_name) && n.attribute("type") == Some("String"))
        .and_then(|n| n.text())
        .map(String::from)
}

fn extract_pointclouds(document: &Document) -> Result<Vec<PointCloud>> {
    let data3d_node = document
        .descendants()
        .find(|n| n.has_tag_name("data3D"))
        .invalid_err("Cannot find data3D tag in XML document")?;

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
        .invalid_err("Cannot find GUID tag inside data3D child")?
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
        .invalid_err("Cannot find points tag inside data3D child")?;

    let file_offset = points_tag
        .attribute("fileOffset")
        .invalid_err("Cannot find fileOffset attribute in points tag")?
        .parse::<u64>()
        .invalid_err("Cannot parse fileOffset as u64")?;

    let records = points_tag
        .attribute("recordCount")
        .invalid_err("Cannot find recordCount attribute in points tag")?
        .parse::<u64>()
        .invalid_err("Cannot parse recordCount as u64")?;

    let prototype_tag = points_tag
        .children()
        .find(|n| n.has_tag_name("prototype") && n.attribute("type") == Some("Structure"))
        .invalid_err("Cannot find prototype child in points tag")?;

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
                tag => {
                    let msg = format!("Found unknown tag name in prototype: {tag}");
                    Error::invalid(&msg)?
                }
            }
        }
    }

    Ok(PointCloud {
        guid,
        name,
        file_offset,
        records,
        prototype,
    })
}
