//! Core constraint types.
//!
//! This module provides fundamental constraint identification and classification
//! types used throughout the constraint evaluation system.

/// Reference to a constraint for identification.
///
/// # Example
///
/// ```
/// use solverforge_core::ConstraintRef;
///
/// let cr = ConstraintRef::new("scheduling", "NoOverlap");
/// assert_eq!(cr.full_name(), "scheduling/NoOverlap");
///
/// let simple = ConstraintRef::new("", "Simple");
/// assert_eq!(simple.full_name(), "Simple");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintRef {
    /// Package/module containing the constraint.
    pub package: String,
    /// Name of the constraint.
    pub name: String,
}

impl ConstraintRef {
    /// Creates a new constraint reference.
    pub fn new(package: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            name: name.into(),
        }
    }

    /// Returns the fully qualified name.
    pub fn full_name(&self) -> String {
        if self.package.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.package, self.name)
        }
    }
}

/// Type of impact a constraint has on the score.
///
/// # Example
///
/// ```
/// use solverforge_core::ImpactType;
///
/// let penalty = ImpactType::Penalty;
/// let reward = ImpactType::Reward;
///
/// assert_ne!(penalty, reward);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImpactType {
    /// Penalize (subtract from score).
    Penalty,
    /// Reward (add to score).
    Reward,
}
