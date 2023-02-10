use crate::Record;

#[derive(Debug, Clone)]
pub struct PointCloud {
    pub guid: String,
    pub name: Option<String>,
    pub file_offset: u64,
    pub records: u64,
    pub prototype: Vec<Record>,
}
