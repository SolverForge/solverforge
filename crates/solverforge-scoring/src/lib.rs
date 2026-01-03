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
    IncrementalUniConstraint, IncrementalBiConstraint,
    IncrementalCrossBiConstraint, IncrementalTriConstraint,
    IncrementalQuadConstraint, IncrementalPentaConstraint,
    GroupedUniConstraint,
};

// ============================================================================
// Constraint Set (Tuple-Based, Zero-Erasure)
// ============================================================================

pub use api::constraint_set::{ConstraintSet, ConstraintResult, IncrementalConstraint};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

// ============================================================================
// Score Directors
// ============================================================================

pub use director::{
    ScoreDirector, ScoreDirectorFactory,
    SimpleScoreDirector,
    RecordingScoreDirector,
};
pub use director::typed::TypedScoreDirector;

// ============================================================================
// Analysis (for score explanation)
// ============================================================================

pub use api::analysis::{
    ScoreExplanation, ConstraintAnalysis, ConstraintJustification,
    DetailedConstraintMatch, DetailedConstraintEvaluation,
    Indictment, IndictmentMap, EntityRef,
};

// ============================================================================
// Fluent Constraint Stream API
// ============================================================================

pub use stream::{
    ConstraintFactory, UniConstraintStream, UniConstraintBuilder,
    BiConstraintStream, BiConstraintBuilder,
    GroupedConstraintStream, GroupedConstraintBuilder,
};
