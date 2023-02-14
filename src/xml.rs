use crate::error::Converter;
use crate::pointcloud::extract_pointcloud;
use crate::{PointCloud, Result};
use roxmltree::Document;

pub struct XmlDocument {
    xml: String,
    format: Option<String>,
    guid: Option<String>,
    pointclouds: Vec<PointCloud>,
}

impl XmlDocument {
    pub fn parse(xml: String) -> Result<Self> {
        let document = Document::parse(&xml).invalid_err("Failed to parse XML data")?;
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
