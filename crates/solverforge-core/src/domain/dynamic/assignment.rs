//! Dynamic metadata for declarative scalar-assignment groups.
//!
//! This is intentionally a metadata boundary, not a construction-phase API.
//! Host-language bindings register one metadata object with one scalar group;
//! the solver owns all candidate generation, construction, and local search.

/// Structural capabilities supplied by a dynamic scalar-assignment group.
///
/// A capability says whether metadata is declared, rather than inferring that
/// fact from a value returned while solving.  The solver uses this distinction
/// when validating construction heuristics and assignment-rule dependencies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicScalarAssignmentMetadataCapabilities {
    pub required_entity: bool,
    pub capacity_key: bool,
    pub position_key: bool,
    pub sequence_key: bool,
    pub entity_order: bool,
    pub value_order: bool,
    pub assignment_rule: bool,
}

/// Object-safe metadata access for one dynamic scalar-assignment group.
///
/// Implementors are bound to a concrete group at model compilation time.  They
/// must not select a group indirectly from thread-local state, a runtime name
/// lookup, or the active phase.  Typed Rust groups retain their direct
/// function-pointer metadata and never use this dynamic boundary.
pub trait DynamicScalarAssignmentMetadata<S>: Send + Sync {
    fn capabilities(&self) -> DynamicScalarAssignmentMetadataCapabilities;

    fn required_entity(&self, solution: &S, entity_index: usize) -> bool;

    fn capacity_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize>;

    fn position_key(&self, solution: &S, entity_index: usize) -> Option<i64>;

    fn sequence_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize>;

    fn entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64>;

    fn value_order_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<i64>;

    fn assignment_edge_allowed(
        &self,
        solution: &S,
        left_entity: usize,
        left_value: usize,
        right_entity: usize,
        right_value: usize,
    ) -> bool;
}
