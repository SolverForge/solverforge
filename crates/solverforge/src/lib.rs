//! SolverForge - A Constraint Solver in Rust
//!
//! SolverForge is a constraint satisfaction/optimization solver inspired by Timefold.
//! It helps you optimize planning and scheduling problems.
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

// ============================================================================
// Attribute Macros
// ============================================================================

pub use solverforge_macros::{planning_entity, planning_solution, problem_fact};

// Derive macros (used by attribute macros, must be at root level)
pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

// ============================================================================
// Score Types
// ============================================================================

pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
};

// ============================================================================
// Constraint API
// ============================================================================

pub use solverforge_scoring::{
    ConstraintSet, IncrementalBiConstraint, IncrementalConstraint, IncrementalUniConstraint,
};

/// Fluent constraint stream API.
pub mod stream {
    pub use solverforge_scoring::stream::{joiner, ConstraintFactory};
}

// ============================================================================
// Score Director
// ============================================================================

pub use solverforge_scoring::ScoreDirector;

// ============================================================================
// Solver
// ============================================================================

pub use solverforge_solver::{
    Analyzable, ConstraintAnalysis, ScoreAnalysis, SolutionManager, Solvable, SolverManager,
    SolverStatus,
};

// ============================================================================
// Fluent Builder API
// ============================================================================

pub use solverforge_solver::public_api::{acceptors, SolverBuilder};

// ============================================================================
// Config Types
// ============================================================================

pub use solverforge_config::{
    AcceptorConfig, ConstructionHeuristicType, GreatDelugeConfig, LateAcceptanceConfig,
    LocalSearchConfig, PhaseConfig, SimulatedAnnealingConfig, SolverConfig, TabuSearchConfig,
    TerminationConfig,
};

// ============================================================================
// Console Output (feature-gated)
// ============================================================================

#[cfg(feature = "console")]
pub mod console;

// ============================================================================
// Prelude
// ============================================================================

pub mod prelude {
    pub use crate::stream::{joiner, ConstraintFactory};
    pub use crate::{
        acceptors, planning_entity, planning_solution, problem_fact, AcceptorConfig, BendableScore,
        ConstraintSet, ConstructionHeuristicType, HardMediumSoftScore, HardSoftDecimalScore,
        HardSoftScore, Score, ScoreDirector, SimpleScore, Solvable, SolverBuilder,
    };
}

// ============================================================================
// Internal API for Macros
// ============================================================================

#[doc(hidden)]
pub mod __internal {
    #[inline]
    pub fn init_console() {
        #[cfg(feature = "console")]
        crate::console::init();
    }

    pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

    pub use solverforge_core::domain::{
        EntityDescriptor, PlanningEntity, PlanningId, PlanningSolution, ProblemFact,
        ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor, VariableDescriptor,
    };

    pub use solverforge_scoring::{
        RecordingScoreDirector, ScoreDirector, ShadowVariableSupport, SolvableSolution,
    };

    pub use solverforge_solver::heuristic::selector::{
        DefaultDistanceMeter, FromSolutionEntitySelector,
    };
    pub use solverforge_solver::manager::{
        ChangeConstructionPhaseBuilder, ChangeLocalSearchPhaseBuilder, KOptPhaseBuilder,
        ListConstructionPhaseBuilder, PhaseFactory, SolverFactory,
    };
    pub use solverforge_solver::phase::localsearch::AcceptorImpl;
    pub use solverforge_solver::Solver;

    pub use solverforge_config::{
        AcceptorConfig, LateAcceptanceConfig, LocalSearchConfig, PhaseConfig, SolverConfig,
    };

    #[cfg(feature = "console")]
    pub use tracing;
}
