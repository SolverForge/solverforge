//! Fluent constraint stream API for zero-erasure constraint programming.
//!
//! This module provides an ergonomic builder pattern for defining constraints
//! that compile to fully-typed, monomorphized constraint implementations.
//!
//! # Overview
//!
//! The stream API transforms verbose constraint definitions into concise,
//! fluent declarations while preserving full type information through the
//! entire evaluation pipeline.
//!
//! # Example
//!
//! ```
//! use solverforge_scoring::stream::ConstraintFactory;
//! use solverforge_scoring::api::constraint_set::{ConstraintSet, IncrementalConstraint};
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone)]
//! struct Schedule {
//!     shifts: Vec<Shift>,
//! }
//!
//! #[derive(Clone, Debug)]
//! struct Shift {
//!     employee_idx: Option<usize>,
//!     required_skill: String,
//! }
//!
//! // Define constraints using the fluent API
//! let unassigned = ConstraintFactory::<Schedule, SimpleScore>::new()
//!     .for_each(|s: &Schedule| &s.shifts)
//!     .filter(|shift: &Shift| shift.employee_idx.is_none())
//!     .penalize(SimpleScore::of(1))
//!     .as_constraint("Unassigned shift");
//!
//! // Use the constraint
//! let schedule = Schedule {
//!     shifts: vec![
//!         Shift { employee_idx: Some(0), required_skill: "A".into() },
//!         Shift { employee_idx: None, required_skill: "B".into() },
//!     ],
//! };
//!
//! assert_eq!(unassigned.evaluate(&schedule), SimpleScore::of(-1));
//! ```
//!
//! # Architecture
//!
//! The stream builders produce existing constraint types at definition time:
//!
//! ```text
//! ConstraintFactory::new()
//!     .for_each(extractor)     -> UniConstraintStream<S, A, Sc>
//!     .filter(predicate)            -> UniConstraintStream (accumulates filters)
//!     .penalize(weight)             -> UniConstraintBuilder<S, A, Sc>
//!     .as_constraint(name)          -> IncrementalUniConstraint<S, A, Sc>
//! ```
//!
//! The final `IncrementalUniConstraint` is fully monomorphized with no
//! virtual dispatch in the hot path.

#[macro_use]
mod arity_stream_macros;
mod balance_stream;
mod bi_stream;
pub mod collector;
mod complemented_stream;
mod cross_bi_stream;
mod factory;
pub mod filter;
mod flattened_bi_stream;
mod grouped_stream;
mod if_exists_stream;
pub mod joiner;
mod penta_stream;
mod quad_stream;
mod tri_stream;
mod uni_stream;

pub use balance_stream::{BalanceConstraintBuilder, BalanceConstraintStream};
pub use bi_stream::{BiConstraintBuilder, BiConstraintStream};
pub use complemented_stream::{ComplementedConstraintBuilder, ComplementedConstraintStream};
pub use cross_bi_stream::{CrossBiConstraintBuilder, CrossBiConstraintStream};
pub use factory::ConstraintFactory;
pub use flattened_bi_stream::{FlattenedBiConstraintBuilder, FlattenedBiConstraintStream};
pub use grouped_stream::{GroupedConstraintBuilder, GroupedConstraintStream};
pub use if_exists_stream::{IfExistsBuilder, IfExistsStream};
pub use penta_stream::{PentaConstraintBuilder, PentaConstraintStream};
pub use quad_stream::{QuadConstraintBuilder, QuadConstraintStream};
pub use tri_stream::{TriConstraintBuilder, TriConstraintStream};
pub use uni_stream::{UniConstraintBuilder, UniConstraintStream};
