//! Monomorphized constraint set for zero-erasure incremental scoring.

mod blanket;
mod chain;
mod incremental;
mod tuple;
mod weighting;

#[cfg(test)]
mod tests;

pub use chain::{ConstraintSetChain, ConstraintSetSource, OrderedConstraintSetChain};
pub use incremental::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint, SingleConstraint,
};
pub(crate) use weighting::{scale_contribution, scale_contribution_parts};
