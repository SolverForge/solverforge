//! SolverForge Core - Core types and traits for constraint solving
//!
//! This crate provides the fundamental abstractions for SolverForge:
//! - Score types for representing solution quality
//! - Domain traits for defining planning problems
//! - Descriptor types for runtime metadata
//! - Constraint types for incremental evaluation

pub mod constraint;
pub mod score;
pub mod domain;
pub mod error;

pub use constraint::{ConstraintRef, ImpactType};
pub use score::{ParseableScore, Score, ScoreParseError, SimpleScore, HardSoftScore, HardMediumSoftScore, HardSoftDecimalScore, BendableScore};
pub use domain::{PlanningSolution, PlanningEntity, ProblemFact, PlanningId};
pub use error::SolverForgeError;
