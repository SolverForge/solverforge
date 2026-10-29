//! Incremental constraint scoring for SolverForge.
//!
//! Applications use it through the `solverforge` facade. It provides:
//! - the constraint stream API (`stream`): `for_each`, joins, filters,
//!   grouping and the `penalize`/`reward` terminals that build constraints;
//! - incremental constraints such as [`IncrementalUniConstraint`], which
//!   update their score from the entities a move changed instead of
//!   rescoring the whole solution;
//! - [`ScoreDirector`], which owns the working solution and its constraint
//!   set and keeps the score current as moves are applied and undone;
//! - [`ConstraintSet`], a tuple of constraints evaluated without dynamic
//!   dispatch.
//!
//! Every constraint is monomorphized: closures are generic parameters, and
//! there is no `Box<dyn Trait>` on the scoring path.

// Zero-erasure architecture intentionally uses complex generic types
#![allow(clippy::type_complexity)]
#![warn(unreachable_pub)]

// Core modules
pub mod api;
pub mod constraint;
pub mod director;
pub mod stream;

/* ============================================================================
Zero-Erasure Incremental Constraints
============================================================================
*/

pub use constraint::{IncrementalUniConstraint, ListPrecedenceMakespanConstraint};

/* ============================================================================
Constraint Set (Tuple-Based, Zero-Erasure)
============================================================================
*/

pub use api::constraint_set::{
    ConstraintMetadata, ConstraintResult, ConstraintSet, ConstraintSetSource,
    IncrementalConstraint, OrderedConstraintSetChain, SingleConstraint,
};
pub use api::node_sharing::{SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation};

/* ============================================================================
Score Directors
============================================================================
*/

pub use director::score_director::{
    requires_weighted_mode, DirectorDescriptor, ReplicaDescriptor, ScoreDirector, UnweightedMode,
    WeightMode, WeightedMode,
};
pub use director::{Director, DirectorScoreState, SolvableSolution};

/* ============================================================================
Analysis (for score explanation)
============================================================================
*/

pub use api::analysis::{
    ConstraintAnalysis, ConstraintJustification, DetailedConstraintMatch, EntityRef,
};

/* ============================================================================
Fluent Constraint Stream API
============================================================================
*/

pub use stream::{
    fixed_weight, hard_weight, BiConstraintBuilder, BiConstraintStream, ConstraintFactory,
    FixedWeight, GroupedConstraintBuilder, GroupedConstraintStream, HardWeight, Projection,
    ProjectionSink, UniConstraintBuilder, UniConstraintStream,
};
