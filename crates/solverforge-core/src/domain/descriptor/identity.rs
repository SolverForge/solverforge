//! Stable logical descriptor identifiers for dynamic model integrations.

/// Logical planning-entity class identifier.
///
/// Macro-generated Rust models can continue to use `TypeId`. Dynamic binding
/// models use this ID when multiple host-language entity classes share one
/// Rust backing row type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityClassId(pub usize);

/// Logical planning-variable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableId(pub usize);

/// Logical problem-fact class identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProblemFactClassId(pub usize);
