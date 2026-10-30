//! Core types and traits for SolverForge.
//!
//! Applications use them through the `solverforge` facade. This crate
//! provides:
//! - Score types for representing solution quality
//! - Domain traits for defining planning problems
//! - Descriptor types for runtime metadata
//! - Constraint types for incremental evaluation

/* PhantomData<(fn() -> T, ...)> is an intentional pattern to avoid inheriting
trait bounds from phantom type parameters. Clippy's type_complexity lint
triggers on these tuples but the pattern is architecturally required.
*/
#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]

pub mod constraint;
pub mod constraint_weights;
pub mod domain;
pub mod fast_hash;
pub mod score;

#[cfg(test)]
mod constraint_tests;

pub use constraint::{ConstraintRef, ImpactType};
pub use constraint_weights::{
    ConstraintWeightError, ConstraintWeights, MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
};
pub use domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
pub use score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, ParseableScore, Score,
    ScoreParseError, SoftScore,
};
