/*
Zero-erasure incremental constraint scoring for SolverForge.

This crate provides monomorphized incremental scoring infrastructure:
- Zero-erasure incremental constraints (IncrementalUniConstraint, IncrementalBiConstraint, etc.)
- Incremental score directors (ScoreDirector)
- Tuple-based constraint sets (zero virtual dispatch)

Architecture:
All scoring is fully monomorphized - no Box<dyn Trait> in hot paths.
Closures are stored as generic type parameters, not Arc<dyn Fn>.
*/

// Zero-erasure architecture intentionally uses complex generic types
#![allow(clippy::type_complexity)]

// Core modules
pub mod api;
pub mod constraint;
pub mod director;
pub mod stream;

/* ============================================================================
Zero-Erasure Incremental Constraints
============================================================================
*/

pub use constraint::{
    CrossComplementedGroupedConstraint, CrossGroupedConstraint, GroupedUniConstraint,
    IncrementalBiConstraint, IncrementalCrossBiConstraint, IncrementalPentaConstraint,
    IncrementalQuadConstraint, IncrementalTriConstraint, IncrementalUniConstraint,
    ProjectedComplementedGroupedConstraint, ProjectedGroupedConstraint, ProjectedUniConstraint,
};

/* ============================================================================
Constraint Set (Tuple-Based, Zero-Erasure)
============================================================================
*/

pub use api::constraint_set::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, IncrementalConstraint,
};
pub use api::node_sharing::{SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation};
pub use api::weight_overrides::{ConstraintWeightOverrides, WeightProvider};

/* ============================================================================
Score Directors
============================================================================
*/

pub use director::score_director::ScoreDirector;
pub use director::{Director, DirectorScoreState, SolvableSolution};

/* ============================================================================
Analysis (for score explanation)
============================================================================
*/

pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintEvaluation,
    DetailedConstraintMatch, EntityRef, Indictment, IndictmentMap, ScoreExplanation,
};

/* ============================================================================
Fluent Constraint Stream API
============================================================================
*/

pub use stream::{
    fixed_weight, hard_weight, BiConstraintBuilder, BiConstraintStream, ConstraintFactory,
    CrossComplementedGroupedConstraintBuilder, CrossComplementedGroupedConstraintStream,
    CrossGroupedConstraintBuilder, CrossGroupedConstraintStream, FixedWeight,
    GroupedConstraintBuilder, GroupedConstraintStream, HardWeight, ProjectedBiConstraintBuilder,
    ProjectedBiConstraintStream, ProjectedComplementedGroupedConstraintBuilder,
    ProjectedComplementedGroupedConstraintStream, ProjectedConstraintBuilder,
    ProjectedConstraintStream, ProjectedGroupedConstraintBuilder, ProjectedGroupedConstraintStream,
    Projection, ProjectionSink, UniConstraintBuilder, UniConstraintStream,
};
