use crate::error::{invalid_file_err, invalid_file_err_str};
use crate::{PointCloud, Record, RecordType, Result};
use roxmltree::{Document, Node};

pub struct XmlDocument {
    xml: String,
    format: Option<String>,
    guid: Option<String>,
    pointclouds: Vec<PointCloud>,
}

impl XmlDocument {
    pub fn parse(xml: String) -> Result<Self> {
        let document = Document::parse(&xml)
            .map_err(|e| invalid_file_err("Failed to parse XML document", e))?;
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
        .ok_or(invalid_file_err_str(
            "Cannot find data3D tag in XML document",
        ))?;

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
        .ok_or(invalid_file_err_str(
            "Cannot find GUID tag inside data3D child",
        ))?
        .text()
        .ok_or(invalid_file_err_str("GUID tag is empty"))?
        .to_string();

    let name = node
        .children()
        .find(|n| n.has_tag_name("name") && n.attribute("type") == Some("String"))
        .and_then(|n| n.text())
        .map(|t| t.to_string());

    let points_tag = node
        .children()
        .find(|n| n.has_tag_name("points") && n.attribute("type") == Some("CompressedVector"))
        .ok_or(invalid_file_err_str(
            "Cannot find points tag inside data3D child",
        ))?;

    let file_offset = points_tag
        .attribute("fileOffset")
        .ok_or(invalid_file_err_str(
            "Cannot find fileOffset attribute in points tag",
        ))?
        .parse::<u64>()
        .map_err(|e| invalid_file_err("Cannot parse fileOffset as u64", e))?;

    let records = points_tag
        .attribute("recordCount")
        .ok_or(invalid_file_err_str(
            "Cannot find recordCount attribute in points tag",
        ))?
        .parse::<u64>()
        .map_err(|e| invalid_file_err("Cannot parse recordCount as u64", e))?;

    let prototype_tag = points_tag
        .children()
        .find(|n| n.has_tag_name("prototype") && n.attribute("type") == Some("Structure"))
        .ok_or(invalid_file_err_str(
            "Cannot find prototype child in points tag",
        ))?;

    let mut prototype = Vec::new();
    for n in prototype_tag.children() {
        if n.is_element() {
            let tag_name = n.tag_name().name();
            match tag_name {
                "cartesianX" => prototype.push(Record::CartesianX(parse_record_type(&n)?)),
                "cartesianY" => prototype.push(Record::CartesianY(parse_record_type(&n)?)),
                "cartesianZ" => prototype.push(Record::CartesianZ(parse_record_type(&n)?)),
                "cartesianInvalidState" => {
                    prototype.push(Record::CartesianInvalidState(parse_record_type(&n)?))
                }
                tag => {
                    let msg = format!("Found unknown tag name in prototype: {tag}");
                    Err(invalid_file_err_str(&msg))?
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

fn parse_record_type(node: &Node) -> Result<RecordType> {
    let type_string = node.attribute("type").ok_or(invalid_file_err_str(
        "Missing type attribute for prototype tag",
    ))?;
    Ok(match type_string {
        "Float" => {
            let min = if let Some(min) = node.attribute("minimum") {
                min.parse::<f64>()
                    .map_err(|e| invalid_file_err("Cannot parse minimum value", e))?
            } else {
                f64::MIN
            };
            let max = if let Some(max) = node.attribute("maximum") {
                max.parse::<f64>()
                    .map_err(|e| invalid_file_err("Cannot parse maximum value", e))?
            } else {
                f64::MAX
            };
            RecordType::Float { min, max }
        }
        "Integer" => {
            let min = if let Some(min) = node.attribute("minimum") {
                min.parse::<i64>()
                    .map_err(|e| invalid_file_err("Cannot parse minimum value", e))?
            } else {
                i64::MIN
            };
            let max = if let Some(max) = node.attribute("maximum") {
                max.parse::<i64>()
                    .map_err(|e| invalid_file_err("Cannot parse maximum value", e))?
            } else {
                i64::MAX
            };
            RecordType::Integer { min, max }
        }
        _ => Err(invalid_file_err_str("Unknown record type detected"))?,
    })
}
