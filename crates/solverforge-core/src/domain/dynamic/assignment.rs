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
    /// [`DynamicScalarAssignmentMetadata::required_entity`] is declared.
    pub required_entity: bool,
    /// [`DynamicScalarAssignmentMetadata::capacity_key`] is declared.
    pub capacity_key: bool,
    /// [`DynamicScalarAssignmentMetadata::position_key`] is declared.
    pub position_key: bool,
    /// [`DynamicScalarAssignmentMetadata::sequence_key`] is declared.
    pub sequence_key: bool,
    /// [`DynamicScalarAssignmentMetadata::entity_order_key`] is declared.
    pub entity_order: bool,
    /// [`DynamicScalarAssignmentMetadata::value_order_key`] is declared.
    pub value_order: bool,
    /// [`DynamicScalarAssignmentMetadata::assignment_edge_allowed`] is
    /// declared; requires `sequence_key`.
    pub assignment_rule: bool,
}

/// Object-safe metadata access for one dynamic scalar-assignment group.
///
/// Implementors are bound to a concrete group at model compilation time.  They
/// must not select a group indirectly from thread-local state, a runtime name
/// lookup, or the active phase.  Typed Rust groups retain their direct
/// function-pointer metadata and never use this dynamic boundary.
pub trait DynamicScalarAssignmentMetadata<S>: Send + Sync {
    /// Metadata this group declares. An undeclared capability's method must
    /// return its neutral result.
    fn capabilities(&self) -> DynamicScalarAssignmentMetadataCapabilities;

    /// Whether the row must be assigned before the solution is complete; neutral result `false`.
    fn required_entity(&self, solution: &S, entity_index: usize) -> bool;

    /// Capacity key consumed by assigning `value` to the row; at most one
    /// assigned row may hold each key. `None` consumes no capacity.
    fn capacity_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize>;

    /// Position of the row used to order assigned rows; `None` when the row has no position.
    fn position_key(&self, solution: &S, entity_index: usize) -> Option<i64>;

    /// Position of the `(row, value)` assignment in the value's sequence; rows
    /// at sequence keys `k` and `k + 1` for the same value are adjacent. `None`
    /// when unsequenced.
    fn sequence_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<usize>;

    /// Ranking key for the row in entity-ordered construction; `None` when undeclared.
    fn entity_order_key(&self, solution: &S, entity_index: usize) -> Option<i64>;

    /// Ranking key for `value` on the row in value-ordered construction; `None` when undeclared.
    fn value_order_key(&self, solution: &S, entity_index: usize, value: usize) -> Option<i64>;

    /// Whether `left_entity` holding `left_value` may directly precede
    /// `right_entity` holding `right_value` in a sequence; neutral result
    /// `true`.
    fn assignment_edge_allowed(
        &self,
        solution: &S,
        left_entity: usize,
        left_value: usize,
        right_entity: usize,
        right_value: usize,
    ) -> bool;
}
