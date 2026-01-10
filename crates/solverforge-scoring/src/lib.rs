//! Zero-erasure incremental constraint scoring for SolverForge.
//!
//! This crate provides fully-typed incremental scoring infrastructure:
//! - Zero-erasure incremental constraints (`IncrementalUniConstraint`, `IncrementalBiConstraint`, etc.)
//! - Typed score directors (`TypedScoreDirector`)
//! - Tuple-based constraint sets (zero virtual dispatch)
//!
//! # Architecture
//!
//! All scoring is fully monomorphized - no `Box<dyn Trait>` in hot paths.
//! Closures are stored as generic type parameters, not `Arc<dyn Fn>`.

// Zero-erasure architecture intentionally uses complex generic types
#![allow(clippy::type_complexity)]

// Core modules
pub mod api;
pub mod constraint;
pub mod director;
pub mod stream;

// ============================================================================
// Zero-Erasure Incremental Constraints
// ============================================================================

pub use constraint::{
    GroupedUniConstraint, IncrementalBiConstraint, IncrementalCrossBiConstraint,
    IncrementalPentaConstraint, IncrementalQuadConstraint, IncrementalTriConstraint,
    IncrementalUniConstraint,
};

// ============================================================================
// Constraint Set (Tuple-Based, Zero-Erasure)
// ============================================================================

pub use api::constraint_set::{ConstraintResult, ConstraintSet, IncrementalConstraint};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// ============================================================================
// Score Directors
// ============================================================================

pub use director::typed::TypedScoreDirector;
pub use director::{
    RecordingScoreDirector, ScoreDirector, ScoreDirectorFactory, SimpleScoreDirector,
};

// ============================================================================
// Analysis (for score explanation)
// ============================================================================

pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};

// ============================================================================
// Fluent Constraint Stream API
// ============================================================================

pub use stream::{
    BiConstraintBuilder, BiConstraintStream, ConstraintFactory, GroupedConstraintBuilder,
    GroupedConstraintStream, UniConstraintBuilder, UniConstraintStream,
};
