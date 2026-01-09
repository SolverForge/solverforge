//! SolverForge - A Constraint Solver in Rust
//!
//! SolverForge is a constraint satisfaction/optimization solver inspired by Timefold.
//! It helps you optimize planning and scheduling problems.
//!
//! # Architecture
//!
//! SolverForge uses **zero-erasure** constraint evaluation - all scoring code is
//! fully monomorphized with no `Box<dyn Trait>` in hot paths.
//!
//! # Quick Start
//!
//! ```
//! use solverforge::prelude::*;
//!
//! #[problem_fact]
//! pub struct Employee {
//!     #[planning_id]
//!     pub id: i64,
//!     pub name: String,
//! }
//!
//! #[planning_entity]
//! pub struct Shift {
//!     #[planning_id]
//!     pub id: i64,
//!     #[planning_variable]
//!     pub employee: Option<i64>,
//! }
//!
//! #[planning_solution]
//! pub struct Schedule {
//!     #[problem_fact_collection]
//!     pub employees: Vec<Employee>,
//!     #[planning_entity_collection]
//!     pub shifts: Vec<Shift>,
//!     #[planning_score]
//!     pub score: Option<HardSoftScore>,
//! }
//! ```

// =============================================================================
// Core types (domain modeling)
// =============================================================================

pub use solverforge_core::{
    // Domain traits
    domain::{
        PlanningEntity as PlanningEntityTrait, PlanningId,
        PlanningSolution as PlanningSolutionTrait, ProblemFact as ProblemFactTrait,
    },
    // Descriptors (for macro-generated code)
    domain::{
        EntityDescriptor, FieldValueRangeProvider, ProblemFactDescriptor, ShadowVariableKind,
        SolutionDescriptor, TypedEntityExtractor, VariableDescriptor,
    },
    // Score types
    score::{
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
    },
    // Other
    ConstraintRef, ImpactType, SolverForgeError,
};

// =============================================================================
// Macros (domain modeling)
// =============================================================================

pub use solverforge_macros::{
    planning_entity, planning_solution, problem_fact, PlanningEntityImpl, PlanningSolutionImpl,
    ProblemFactImpl,
};

// =============================================================================
// Configuration
// =============================================================================

pub use solverforge_config::{
    AcceptorConfig, ConstructionHeuristicConfig, ConstructionHeuristicType, EnvironmentMode,
    ForagerConfig, LateAcceptanceConfig, LocalSearchConfig, PhaseConfig, SolverConfig,
    TerminationConfig,
};

// =============================================================================
// Solver (high-level API)
// =============================================================================

pub use solverforge_solver::{Solver, SolverFactory};

// Fluent solver builder API
pub use solverforge_solver::manager::{
    ConstructionPhaseFactory, ConstructionType, KOptPhaseBuilder, LocalSearchPhaseFactory,
    LocalSearchType, SolverManager, SolverManagerBuilder, SolverPhaseFactory,
};

// Phase execution (for advanced usage)
pub use solverforge_solver::{Phase, SolverScope};

// Statistics
pub use solverforge_solver::statistics::{
    PhaseStatistics, ScoreImprovement, SolverStatistics, StatisticsCollector,
};

// =============================================================================
// Constraint infrastructure (zero-erasure)
// =============================================================================

pub use solverforge_scoring::{
    // Constraint types
    ConstraintAnalysis, ConstraintJustification, ConstraintResult, ConstraintSet,
    GroupedUniConstraint,
    // Incremental constraints
    IncrementalBiConstraint, IncrementalConstraint, IncrementalCrossBiConstraint,
    IncrementalPentaConstraint, IncrementalQuadConstraint, IncrementalTriConstraint,
    IncrementalUniConstraint,
    // Score directors
    ScoreDirector, ShadowAwareScoreDirector, ShadowVariableSupport, SimpleScoreDirector,
    TypedScoreDirector,
    // Analysis
    ScoreExplanation,
};

// Fluent constraint stream API
pub use solverforge_scoring::stream;

// =============================================================================
// Phase builder support types
// =============================================================================

// Entity selector for phase builders
pub use solverforge_solver::FromSolutionEntitySelector;

// Distance meter trait for nearby k-opt
pub use solverforge_solver::ListPositionDistanceMeter;

// =============================================================================
// Prelude
// =============================================================================

/// Prelude module for convenient imports.
pub mod prelude {
    pub use super::{
        // Macros
        planning_entity, planning_solution, problem_fact,
        // Domain traits
        PlanningEntityTrait, PlanningId, PlanningSolutionTrait, ProblemFactTrait,
        // Scores
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
        // Descriptors
        EntityDescriptor, SolutionDescriptor,
        // Constraints
        ConstraintRef, ConstraintSet, ImpactType,
        IncrementalBiConstraint, IncrementalConstraint, IncrementalCrossBiConstraint,
        IncrementalPentaConstraint, IncrementalQuadConstraint, IncrementalTriConstraint,
        IncrementalUniConstraint,
        // Score directors
        SimpleScoreDirector, TypedScoreDirector,
        // Solver
        Solver, SolverConfig, SolverFactory,
    };
    // Fluent constraint API
    pub use super::stream::{joiner, ConstraintFactory};
}
