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

    pub(crate) fn validate_prototype(prototype: &[Record], extensions: &[Extension]) -> Result<()> {
        for record in prototype {
            if let RecordName::Unknown { namespace, name } = &record.name {
                Self::validate_name(namespace)?;
                Self::validate_name(name)?;
                if !extensions.iter().any(|e| &e.namespace == namespace) {
                    Error::invalid(format!(
                        "Cannot find extension namespace {namespace} used by attribute {name}, please register extension first"
                    ))?
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_name(name: &str) -> Result<()> {
        if name.to_lowercase().starts_with("xml") {
            Error::invalid(format!(
                "Strings used as XML namespaces or attributes must not start with 'XML': {name}"
            ))?
        }
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || (c == '_') || (c == '-'));
        if !valid_chars {
            Error::invalid(
                format!("Strings used as XML namespaces or attributes should consist only of a-z, A-Z, 0-9, dashes and underscores: '{name}'"),
            )?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name() {
        assert!(Extension::validate_name("abcz").is_ok());
        assert!(Extension::validate_name("ABCZ").is_ok());
        assert!(Extension::validate_name("0129").is_ok());
        assert!(Extension::validate_name("-_-").is_ok());
        assert!(Extension::validate_name("aBC-DEf-Z_09").is_ok());

        assert!(Extension::validate_name("xmlabc").is_err());
        assert!(Extension::validate_name("XMLabc").is_err());
        assert!(Extension::validate_name("axml").is_ok());

        assert!(Extension::validate_name("abc.").is_err());
        assert!(Extension::validate_name("äüöß").is_err());
    }
}
