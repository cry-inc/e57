use crate::{Error, Record, RecordName, Result};
use roxmltree::Document;

/// Describes an extension by name and URL as used in the XML namespace desclaration.
///
/// Each extension has its own namespace that used when describing additional data
/// in the XML section of the E57 file.
#[derive(Clone, Debug)]
pub struct Extension {
    /// XML namespace name.
    pub namespace: String,
    /// XML namespace URL.
    pub url: String,
}

impl Extension {
    /// Intialize and return a new Extension structure with the given values.
    pub fn new(namespace: &str, url: &str) -> Self {
        Self {
            namespace: namespace.to_owned(),
            url: url.to_owned(),
        }
    }

    pub(crate) fn vec_from_document(document: &Document) -> Vec<Extension> {
        let mut extensions = Vec::new();
        for item in document.root_element().namespaces() {
            if let Some(name) = item.name() {
                extensions.push(Extension {
                    namespace: name.to_string(),
                    url: item.uri().to_string(),
                });
            }
        }
        extensions
    }

    pub(crate) fn validate(prototype: &[Record], extensions: &[Extension]) -> Result<()> {
        for record in prototype {
            if let RecordName::Unknown { namespace, name } = &record.name {
                if !extensions.iter().any(|e| &e.namespace == namespace) {
                    Error::invalid(format!(
                        "Cannot find extension namespace {namespace} used by attribute {name}, please register extension first"
                    ))?
                }
            }
        }
        Ok(())
    }
}
