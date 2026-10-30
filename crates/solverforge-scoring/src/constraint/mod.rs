//! Zero-erasure monomorphized constraint infrastructure.
//!
//! This module provides a fully monomorphized constraint evaluation system where
//! all closures are stored as concrete generic types - no Arc, no dyn,
//! fully monomorphized.
//!
//! # Key Benefits
//!
//! - **No hot-path erasure**: Filters and weights are generic type params
//! - **Inline evaluation**: No boxing or downcasting per predicate call
//! - **Monomorphized pipelines**: Each constraint is fully specialized

pub mod balance;
pub mod incremental;
/// List-plus-precedence makespan scoring.
pub mod list_precedence;
/// Terminal constraints over retained projected rows.
pub mod projected;
pub mod relational;
pub(crate) mod score_sum;
mod single_constraint_markers;

#[cfg(test)]
mod tests;

pub use balance::BalanceConstraint;
pub use incremental::IncrementalUniConstraint;
pub use list_precedence::ListPrecedenceMakespanConstraint;
