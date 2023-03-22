use crate::date_time::serialize_date_time;
use crate::error::Converter;
use crate::pointcloud::serialize_pointcloud;
use crate::xml::{optional_date_time, optional_string, required_integer, required_string};
use crate::{DateTime, Error, PointCloud, Result};
use roxmltree::Document;

/// E57 XML Root structure with information shared by all elements in the file.
#[derive(Debug)]
#[non_exhaustive]
pub struct Root {
    pub format: String,
    pub guid: String,
    pub major_version: i64,
    pub minor_version: i64,
    pub library_version: Option<String>,
    pub creation: Option<DateTime>,
    pub coordinate_metadata: Option<String>,
}

impl Default for Root {
    fn default() -> Self {
        Self {
            format: String::from("ASTM E57 3D Imaging Data File"),
            guid: String::new(),
            major_version: 1,
            minor_version: 0,
            creation: None,
            coordinate_metadata: None,
            library_version: None,
        }
    }
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
    let library_version = optional_string(&root, "e57LibraryVersion")?;

    Ok(Root {
        format,
        guid,
        creation,
        major_version,
        minor_version,
        coordinate_metadata,
        library_version,
    })
}

pub fn serialize_root(root: &Root, pointclouds: &[PointCloud]) -> Result<String> {
    let mut xml = String::new();
    xml += "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    xml += "<e57Root type=\"Structure\" xmlns=\"http://www.astm.org/COMMIT/E57/2010-e57-v1.0\">\n";
    xml += "<formatName type=\"String\"><![CDATA[ASTM E57 3D Imaging Data File]]></formatName>\n";
    if root.guid.is_empty() {
        Error::invalid("Empty file GUID is not allowed")?
    }
    xml += &format!("<guid type=\"String\"><![CDATA[{}]]></guid>\n", root.guid);
    xml += &format!(
        "<versionMajor type=\"Integer\">{}</versionMajor>\n",
        root.major_version
    );
    xml += &format!(
        "<versionMinor type=\"Integer\">{}</versionMinor>\n",
        root.minor_version
    );
    if let Some(cm) = &root.coordinate_metadata {
        xml +=
            &format!("<coordinateMetadata type=\"String\"><![CDATA[{cm}]]></coordinateMetadata>\n");
    }
    if let Some(lv) = &root.library_version {
        xml +=
            &format!("<e57LibraryVersion type=\"String\"><![CDATA[{lv}]]></e57LibraryVersion>\n");
    }
    if let Some(dt) = &root.creation {
        xml += &serialize_date_time(dt, "creationDateTime");
    }
    xml += "<data3D type=\"Vector\" allowHeterogeneousChildren=\"1\">\n";
    for pc in pointclouds {
        xml += &serialize_pointcloud(pc)?;
    }
    xml += "</data3D>\n";
    xml += "<images2D type=\"Vector\" allowHeterogeneousChildren=\"1\">\n";
    xml += "</images2D>\n";
    xml += "</e57Root>\n";
    Ok(xml)
}
