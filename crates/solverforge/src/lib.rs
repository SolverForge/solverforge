//! SolverForge - A Constraint Solver in Rust
//!
//! SolverForge is a constraint satisfaction/optimization solver.
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
// Prelude - The ONE import users need
// ============================================================================

pub mod prelude {
    // Macros to define domain
    pub use solverforge_macros::{planning_entity, planning_solution, problem_fact};

    // Score types
    pub use solverforge_core::score::{
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
    };

    // Constraint API
    pub use solverforge_scoring::stream::{collector, joiner, ConstraintFactory};
    pub use solverforge_scoring::ConstraintSet;

    // Score director for manual solving
    pub use solverforge_scoring::{ScoreDirector, TypedScoreDirector};

    // Solver API
    pub use solverforge_solver::{
        Analyzable, ConstraintAnalysis, ScoreAnalysis, SolutionManager, Solvable, SolverManager,
        SolverStatus,
    };
}

// Re-export prelude at root for convenience
pub use prelude::*;

// Derive macros must be at crate root for macro expansion
pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

// Incremental constraints (advanced API, not in prelude)
pub use solverforge_scoring::{
    IncrementalBiConstraint, IncrementalConstraint, IncrementalUniConstraint,
};

// Console output (feature-gated)
#[cfg(feature = "console")]
pub mod console;

// ============================================================================
// Internal API for Macros
// ============================================================================

/// Internal module for macro-generated code. Not part of public API.
#[doc(hidden)]
pub mod __internal {
    /// Initializes console output if the feature is enabled.
    #[inline]
    pub fn init_console() {
        #[cfg(feature = "console")]
        crate::console::init();
    }

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
    pub use solverforge_solver::manager::{
        KOptPhaseBuilder, ListConstructionPhaseBuilder, PhaseFactory, SolverFactory,
    };
    pub use solverforge_solver::{MoveImpl, MoveSelectorImpl};

    // Variable operations for run_solver
    pub use solverforge_solver::operations::VariableOperations;
    pub use solverforge_solver::run_solver;

    // Config
    pub use solverforge_config::SolverConfig;
}
