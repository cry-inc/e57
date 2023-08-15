use crate::{Record, RecordName};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Describes an extension by name and url.
#[derive(Clone, Debug, Eq)]
pub struct Extension {
    pub name: String,
    pub url: String,
}

impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Extension {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Extension {
    pub fn from_prototype(prototype: &[Record]) -> Vec<Self> {
        let mut ext = HashSet::new();
        for proto in prototype {
            if let RecordName::Unknown { extension, name: _ } = &proto.name {
                ext.insert(extension.clone());
            }
        }
        Vec::from_iter(ext)
    }
}
