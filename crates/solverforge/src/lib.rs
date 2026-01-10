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

pub use solverforge_scoring::{ScoreDirector, TypedScoreDirector};

// ============================================================================
// Prelude
// ============================================================================

pub mod prelude {
    pub use crate::stream::{joiner, ConstraintFactory};
    pub use crate::{
        planning_entity, planning_solution, problem_fact, BendableScore, ConstraintSet,
        HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, ScoreDirector,
        SimpleScore, TypedScoreDirector,
    };
}

// ============================================================================
// Internal API for Macros
// ============================================================================

/// Internal module for macro-generated code. Not part of public API.
#[doc(hidden)]
pub mod __internal {
    // Derive macros
    pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

    // Domain types
    pub use solverforge_core::domain::{
        EntityDescriptor, PlanningEntity, PlanningId, PlanningSolution, ProblemFact,
        ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor, TypedEntityExtractor,
        VariableDescriptor,
    };

    // Scoring
    pub use solverforge_scoring::{
        ScoreDirector, ShadowAwareScoreDirector, ShadowVariableSupport, SimpleScoreDirector,
        SolvableSolution, TypedScoreDirector,
    };

    // Solver infrastructure
    pub use solverforge_solver::heuristic::selector::{
        DefaultDistanceMeter, FromSolutionEntitySelector,
    };
    pub use solverforge_solver::manager::{PhaseFactory, SolverManager};
    pub use solverforge_solver::phase::{KOptPhaseBuilder, ListConstructionPhaseBuilder};

    // Config
    pub use solverforge_config::SolverConfig;
}
