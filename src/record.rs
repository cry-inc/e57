#[derive(Debug, Clone)]
pub enum RecordType {
    Float { min: f64, max: f64 },
    Integer { min: i64, max: i64 },
}

#[derive(Debug, Clone)]
pub enum Record {
    CartesianX(RecordType),
    CartesianY(RecordType),
    CartesianZ(RecordType),
    CartesianInvalidState(RecordType),
}
