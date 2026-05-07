// Monomorphized constraint set for zero-erasure incremental scoring.

mod incremental;

#[cfg(test)]
mod tests;

pub use incremental::{ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint};
