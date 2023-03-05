use crate::error::Converter;
use crate::xml::{optional_date_time, optional_string, required_integer, required_string};
use crate::{DateTime, Result};
use roxmltree::Document;

/// E57 XML Root structure with information shared by all elements in the file.
#[derive(Debug)]
#[non_exhaustive]
pub struct Root {
    pub format: String,
    pub guid: String,
    pub major_version: i64,
    pub minor_version: i64,
    pub creation: Option<DateTime>,
    pub coordinate_metadata: Option<String>,
}

pub fn root_from_document(document: &Document) -> Result<Root> {
    let root = document
        .descendants()
        .find(|n| n.has_tag_name("e57Root"))
        .invalid_err("Unable to find e57Root tag in XML document")?;

    // Required fields
    let format = required_string(&root, "formatName")?;
    let guid = required_string(&root, "guid")?;
    let major_version = required_integer(&root, "versionMajor")?;
    let minor_version = required_integer(&root, "versionMajor")?;

    // Optional fields
    let creation = optional_date_time(&root, "creationDateTime")?;
    let coordinate_metadata = optional_string(&root, "coordinateMetadata")?;

    Ok(Root {
        format,
        guid,
        creation,
        major_version,
        minor_version,
        coordinate_metadata,
    })
}
