//! SolverForge - A Constraint Solver in Rust
//!
//! Zero-wiring API: Just annotate your domain and call `solution.solve()`.
//!
//! # Example
//!
//! ```rust
//! use solverforge::prelude::*;
//!
//! // Score types are re-exported
//! let score = HardSoftScore::of(0, -100);
//! assert_eq!(score.hard(), 0);
//! assert_eq!(score.soft(), -100);
//! ```

// User-facing macros
pub use solverforge_macros::{planning_entity, planning_solution, problem_fact};

// Derive macros (used by attribute macros, not called directly by users)
#[doc(hidden)]
pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

// Score types
pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
};

// Constraint stream API
pub use solverforge_scoring::stream;

// User-facing traits for constraint definitions
pub use solverforge_scoring::ConstraintSet;

// Score director for constraint analysis
pub use solverforge_scoring::TypedScoreDirector;

// Distance meter for k-opt optimization
pub use solverforge_solver::ListPositionDistanceMeter;

mod solver;
pub use solver::run_solver;

/// Internal types for macro-generated code. Do not use directly.
#[doc(hidden)]
pub mod __internal {
    pub use solverforge_scoring::{
        ScoreDirector, ShadowAwareScoreDirector, ShadowVariableSupport,
        SolvableSolution, TypedScoreDirector,
    };
    pub use solverforge_core::domain::{
        EntityDescriptor, ListVariableSolution, PlanningEntity, PlanningId, PlanningSolution,
        ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor, TypedEntityExtractor,
        VariableDescriptor,
    };
    pub use solverforge_config::SolverConfig;
    pub use solverforge_solver::{
        SolverManager, SolverManagerBuilder, ListPositionDistanceMeter,
        KOptPhaseBuilder, ListConstructionPhaseBuilder, SolverPhaseFactory,
        DiminishedReturnsTermination,
        BasicConstructionPhaseBuilder, BasicLocalSearchPhaseBuilder,
        LocalSearchType,
    };
    pub use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
    pub use solverforge_solver::heuristic::selector::k_opt::DefaultDistanceMeter;
}

pub mod prelude {
    pub use super::{planning_entity, planning_solution, problem_fact};
    pub use super::{
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
    };
    pub use super::stream::{joiner, ConstraintFactory};
    pub use super::ConstraintSet;
}
