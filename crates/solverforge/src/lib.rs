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
#[planning_variable(value_range_provider = "employees", allows_unassigned = true)]
pub employee_idx: Option<usize>,
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

pub use solverforge_macros::{
    planning_entity, planning_model, planning_solution, problem_fact, solverforge_constraints,
};

/* ============================================================================
Score Types
============================================================================
*/

pub use solverforge_config::{
    AcceptorConfig, ConstructionHeuristicType, ConstructionObligation, EnvironmentMode,
    ForagerConfig, HardRegressionPolicyConfig, MoveSelectorConfig, MoveThreadCount, PhaseConfig,
    RecreateHeuristicType, SolverConfig, SolverConfigOverride, UnionSelectionOrder,
};
pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SoftScore,
};

pub mod cvrp;
pub mod planning;
pub mod prelude;
pub mod stream;

#[doc(hidden)]
pub mod __internal;

/* ============================================================================
Constraint API
============================================================================
*/

pub use solverforge_scoring::{
    fixed_weight, hard_weight, ConstraintMetadata, ConstraintSet, FixedWeight, HardWeight,
    IncrementalBiConstraint, IncrementalConstraint, IncrementalUniConstraint, Projection,
    ProjectionSink,
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
    analyze, local_search, run_solver, run_solver_with_config, Analyzable, ConflictRepair,
    ConstraintAnalysis, CustomSearchPhase, ExhaustiveSearchConfig, ExhaustiveSearchPhase,
    ExplorationType, FunctionalPartitioner, PartitionedSearchPhase, RepairCandidate, RepairLimits,
    RepairProvider, ScalarAssignmentRule, ScalarCandidate, ScalarCandidateProvider, ScalarEdit,
    ScalarGroup, ScalarGroupLimits, ScalarTarget, ScoreAnalysis, Search, SearchContext,
    SelectorTelemetry, SimpleDecider, SolutionPartitioner, Solvable, SolverEvent,
    SolverEventMetadata, SolverLifecycleState, SolverManager, SolverManagerError, SolverRuntime,
    SolverSnapshot, SolverSnapshotAnalysis, SolverStatus, SolverTelemetry, SolverTerminalReason,
    ThreadCount,
};

/* ============================================================================
Console Output (feature-gated)
============================================================================
*/

#[cfg(feature = "console")]
pub use solverforge_console as console;
