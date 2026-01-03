//! Zero-erasure typed constraint infrastructure.
//!
//! This module provides a fully typed constraint evaluation system where
//! all closures are stored as concrete generic types - no Arc, no dyn,
//! fully monomorphized.
//!
//! # Key Benefits
//!
//! - **No hot-path erasure**: Filters and weights are generic type params
//! - **Inline evaluation**: No boxing or downcasting per predicate call
//! - **Monomorphized pipelines**: Each constraint is fully specialized

#[macro_use]
pub mod macros;

pub mod balance;
pub mod bi_incremental;
pub mod complemented;
pub mod cross_bi_incremental;
pub mod flattened_bi;
pub mod grouped;
pub mod if_exists;
pub mod incremental;
pub mod penta_incremental;
pub mod quad_incremental;
pub mod shared;
pub mod tri_incremental;

#[cfg(test)]
mod bi_incr_tests;
#[cfg(test)]
mod tri_incr_tests;
#[cfg(test)]
mod quad_incr_tests;
#[cfg(test)]
mod penta_incr_tests;

pub use balance::BalanceConstraint;
pub use bi_incremental::IncrementalBiConstraint;
pub use complemented::ComplementedGroupConstraint;
pub use cross_bi_incremental::IncrementalCrossBiConstraint;
pub use flattened_bi::FlattenedBiConstraint;
pub use grouped::GroupedUniConstraint;
pub use if_exists::{ExistenceMode, IfExistsUniConstraint};
pub use incremental::IncrementalUniConstraint;
pub use penta_incremental::IncrementalPentaConstraint;
pub use quad_incremental::IncrementalQuadConstraint;
pub use tri_incremental::IncrementalTriConstraint;
