//! Diagnostics describing how many named terminals share one relational node.
//!
//! The canonical grouping migration routes every grouped, cross-grouped,
//! projected, and complemented stream through the single `GroupNode` operator.
//! These diagnostics report how many `#[solverforge_constraints]` terminals were
//! authored over that one shared accumulator, which is the compiler-owned
//! sharing contract made visible to operators.

/// Identity of the shared node within the compiled constraint program.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct SharedNodeId(pub usize);

/// The relational family of the shared node, named for diagnostics only.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SharedNodeOperation {
    /// Grouped rows over a unary stream.
    Grouped,
    /// Projected rows grouped into a shared accumulator.
    ProjectedGrouped,
    /// Cross-bi joined rows grouped into a shared accumulator.
    CrossGrouped,
    /// Cross-bi joined rows grouped, then complemented with default rows.
    CrossComplementedGrouped,
    /// Projected rows grouped, then complemented with default rows.
    ProjectedComplementedGrouped,
}

/// A single shared node's diagnostics: its shared consumers and root
/// notification bookkeeping while the terminal was incrementally live.
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
