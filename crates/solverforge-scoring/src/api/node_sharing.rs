#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SharedNodeId(pub usize);

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SharedNodeOperation {
    Grouped,
    ProjectedGrouped,
    CrossGrouped,
    CrossComplementedGrouped,
    ProjectedComplementedGrouped,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SharedNodeDiagnostics {
    pub id: SharedNodeId,
    pub fingerprint: String,
    pub operation: SharedNodeOperation,
    pub terminal_consumers: Vec<String>,
    pub update_count: usize,
    pub changed_key_count: usize,
}

impl SharedNodeDiagnostics {
    pub fn new(
        id: SharedNodeId,
        fingerprint: impl Into<String>,
        operation: SharedNodeOperation,
        terminal_consumers: Vec<String>,
        update_count: usize,
        changed_key_count: usize,
    ) -> Self {
        Self {
            id,
            fingerprint: fingerprint.into(),
            operation,
            terminal_consumers,
            update_count,
            changed_key_count,
        }
    }
}
