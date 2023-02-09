use crate::{error::invalid_file_err, Result};
use roxmltree::Document;

#[derive(Debug)]
pub struct XmlDocument {
    xml: String,
    format: Option<String>,
    guid: Option<String>,
}

impl XmlDocument {
    pub fn parse(xml: String) -> Result<Self> {
        let document = Document::parse(&xml)
            .map_err(|e| invalid_file_err("Failed to parse XML document", e))?;
        let format = extract_string(&document, "formatName");
        let guid = extract_string(&document, "guid");
        Ok(Self { xml, format, guid })
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
}

fn extract_string(document: &Document, tag_name: &str) -> Option<String> {
    document
        .descendants()
        .find(|n| n.has_tag_name(tag_name) && n.attribute("type") == Some("String"))
        .and_then(|n| n.text())
        .map(String::from)
}
