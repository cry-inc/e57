use crate::date_time::date_time_from_node;
use crate::error::Converter;
use crate::{DateTime, Result};
use roxmltree::{Document, Node};

/// E57 XML Root structure with information shared by all elements in the file.
#[derive(Debug)]
#[non_exhaustive]
pub struct Root {
    pub format: String,
    pub guid: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub creation: Option<DateTime>,
    pub coordinate_metadata: Option<String>,
}

pub fn root_from_document(document: &Document) -> Result<Root> {
    let root = document
        .descendants()
        .find(|n| n.has_tag_name("e57Root"))
        .invalid_err("Unable to find e57Root tag in XML")?;

    // Required fields
    let format =
        extract_string(&root, "formatName").invalid_err("Cannot find 'formatName' in XML root")?;
    let guid = extract_string(&root, "guid").invalid_err("Cannot find 'guid' in XML root")?;
    let major_version = extract_version(&root, "versionMajor")?;
    let minor_version = extract_version(&root, "versionMajor")?;

    // Optional fields
    let creation = extract_creation_date(&root)?;
    let coordinate_metadata = extract_string(&root, "coordinateMetadata");

    Ok(Root {
        format,
        guid,
        creation,
        major_version,
        minor_version,
        coordinate_metadata,
    })
}

fn extract_string(node: &Node, tag_name: &str) -> Option<String> {
    node.children()
        .find(|n| n.has_tag_name(tag_name) && n.attribute("type") == Some("String"))
        .and_then(|n| n.text())
        .map(String::from)
}

fn extract_version(node: &Node, tag_name: &str) -> Result<u32> {
    node.children()
        .find(|n| n.has_tag_name(tag_name) && n.attribute("type") == Some("Integer"))
        .invalid_err(format!(
            "Unable to find required tag '{tag_name}' in XML root"
        ))?
        .text()
        .unwrap_or("0")
        .parse::<u32>()
        .invalid_err(format!(
            "Unable to parse context of tag '{tag_name}' in XML root as u32"
        ))
}

fn extract_creation_date(node: &Node) -> Result<Option<DateTime>> {
    let creation_node = node.children().find(|n| n.has_tag_name("creationDateTime"));
    if let Some(node) = creation_node {
        Ok(date_time_from_node(&node)?)
    } else {
        Ok(None)
    }
}
