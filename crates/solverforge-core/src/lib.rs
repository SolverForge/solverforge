//! SolverForge Core - Core types and traits for constraint solving
//!
//! This crate provides the fundamental abstractions for SolverForge:
//! - Score types for representing solution quality
//! - Domain traits for defining planning problems
//! - Descriptor types for runtime metadata
//! - Constraint types for incremental evaluation

pub mod constraint;
pub mod domain;
pub mod error;
pub mod score;

#[cfg(test)]
mod constraint_tests;

pub use constraint::{ConstraintRef, ImpactType};
pub use domain::{PlanningEntity, PlanningId, PlanningSolution, ProblemFact};
pub use error::SolverForgeError;
pub use score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, ParseableScore, Score,
    ScoreParseError, SimpleScore,
};
