// Typed constraint set for zero-erasure incremental scoring.

mod incremental;

#[cfg(test)]
mod tests;

pub use incremental::{ConstraintResult, ConstraintSet, IncrementalConstraint};
