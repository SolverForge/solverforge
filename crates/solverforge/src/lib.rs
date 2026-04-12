/* SolverForge - A Constraint Solver in Rust

SolverForge is a high-performance constraint satisfaction/optimization solver.
It helps you optimize planning and scheduling problems.

# Quick Start

```
use solverforge::prelude::*;

#[problem_fact]
pub struct Employee {
#[planning_id]
pub id: i64,
pub name: String,
}

#[planning_entity]
pub struct Shift {
#[planning_id]
pub id: i64,
#[planning_variable]
pub employee: Option<i64>,
}

#[planning_solution]
pub struct Schedule {
#[problem_fact_collection]
pub employees: Vec<Employee>,
#[planning_entity_collection]
pub shifts: Vec<Shift>,
#[planning_score]
pub score: Option<HardSoftScore>,
}
```
*/

/* ============================================================================
Attribute Macros
============================================================================
*/

pub use solverforge_macros::{planning_entity, planning_solution, problem_fact};

// Derive macros (used by attribute macros, must be at root level)
pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

/* ============================================================================
Score Types
============================================================================
*/

pub use solverforge_config::{SolverConfig, SolverConfigOverride};
pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SoftScore,
};

pub mod cvrp;
pub mod prelude;
pub mod stream;

#[doc(hidden)]
pub mod __internal;

/* ============================================================================
Constraint API
============================================================================
*/

pub use solverforge_scoring::{
    ConstraintSet, IncrementalBiConstraint, IncrementalConstraint, IncrementalUniConstraint,
};

/* ============================================================================
Score Director
============================================================================
*/

pub use solverforge_scoring::{Director, ScoreDirector};

/* ============================================================================
Solver
============================================================================
*/

pub use solverforge_solver::heuristic::selector::DefaultDistanceMeter;
pub use solverforge_solver::CrossEntityDistanceMeter;
pub use solverforge_solver::{
    analyze, run_solver, run_solver_with_config, Analyzable, ConstraintAnalysis, ScoreAnalysis,
    Solvable, SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverManager,
    SolverManagerError, SolverRuntime, SolverSnapshot, SolverSnapshotAnalysis, SolverStatus,
    SolverTelemetry, SolverTerminalReason,
};

/* ============================================================================
Console Output (feature-gated)
============================================================================
*/

#[cfg(feature = "console")]
pub use solverforge_console as console;
