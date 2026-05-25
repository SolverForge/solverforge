//! Stable logical identifiers for dynamic binding models.

/// Logical planning-entity class identifier.
///
/// Dynamic bindings may back many host-language entity classes with one Rust
/// row type, so descriptor identity cannot be derived from `TypeId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityClassId(pub usize);

/// Logical planning-variable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableId(pub usize);

/// Logical problem-fact class identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProblemFactClassId(pub usize);
