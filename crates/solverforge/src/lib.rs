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

pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SoftScore,
};

/* ============================================================================
Constraint API
============================================================================
*/

pub use solverforge_scoring::{
    ConstraintSet, IncrementalBiConstraint, IncrementalConstraint, IncrementalUniConstraint,
};

/// Fluent constraint stream API.
pub mod stream {
    pub use solverforge_scoring::stream::collection_extract::vec;
    pub use solverforge_scoring::stream::collection_extract::{CollectionExtract, VecExtract};
    pub use solverforge_scoring::stream::{joiner, ConstraintFactory};
}

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
    analyze, run_solver, Analyzable, ConstraintAnalysis, ScoreAnalysis, Solvable, SolverEvent,
    SolverManager, SolverStatus, SolverTelemetry,
};

/* ============================================================================
CVRP domain helpers
============================================================================
*/

pub mod cvrp {
    pub use solverforge_cvrp::{
        capacity, depot_for_cw, depot_for_entity, distance, element_load, get_route,
        is_kopt_feasible, is_time_feasible, replace_route, MatrixDistanceMeter,
        MatrixIntraDistanceMeter, ProblemData, VrpSolution,
    };
}

/* ============================================================================
Console Output (feature-gated)
============================================================================
*/

#[cfg(feature = "console")]
pub use solverforge_console as console;

/* ============================================================================
Prelude
============================================================================
*/

pub mod prelude {
    pub use crate::stream::{joiner, ConstraintFactory};
    pub use crate::{
        planning_entity, planning_solution, problem_fact, BendableScore, ConstraintSet, Director,
        HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, ScoreDirector, SoftScore,
    };
}

/* ============================================================================
Internal API for Macros
============================================================================
*/

// Internal module for macro-generated code. Not part of public API.
#[doc(hidden)]
pub mod __internal {
    // Initializes console output if the feature is enabled.
    #[inline]
    pub fn init_console() {
        #[cfg(feature = "console")]
        solverforge_console::init();
    }

    // Derive macros
    pub use solverforge_macros::{PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl};

    // Domain types
    pub use solverforge_core::domain::{
        EntityDescriptor, PlanningEntity, PlanningId, PlanningSolution, ProblemFact,
        ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor, TypedEntityExtractor,
        ValueRangeType, VariableDescriptor,
    };

    // Scoring
    pub use solverforge_scoring::{
        Director, ScoreDirector, ShadowVariableSupport, SolvableSolution,
    };
    pub use tokio::sync::mpsc::UnboundedSender;

    // Solver infrastructure
    pub use solverforge_solver::builder::ListContext;
    pub use solverforge_solver::heuristic::selector::{
        DefaultCrossEntityDistanceMeter, DefaultDistanceMeter, FromSolutionEntitySelector,
    };
    pub use solverforge_solver::manager::{
        KOptPhaseBuilder, ListConstructionPhaseBuilder, PhaseFactory, SolverFactory,
    };
    pub use solverforge_solver::scope::{ProgressCallback, SolverScope};
    pub use solverforge_solver::{
        build_descriptor_construction, build_descriptor_move_selector, build_list_construction,
        build_phases, build_unified_local_search, build_unified_move_selector, build_unified_vnd,
        descriptor_has_bindings, log_solve_start, run_solver, DescriptorConstruction,
        DynamicVndPhase, ListConstruction, ListConstructionArgs, ListVariableEntity,
        ListVariableMetadata, Phase, PhaseSequence, RuntimePhase, SeedBestSolutionPhase,
        SolverEvent, SolverTelemetry, UnifiedConstruction, UnifiedLocalSearch, UnifiedMove,
        UnifiedNeighborhood, UnifiedRuntimePhase, UnifiedVnd,
    };

    // Config
    pub use solverforge_config::{PhaseConfig, SolverConfig};

    // Stream types needed for macro-generated extension traits
    pub use solverforge_scoring::stream::filter::{
        AndUniFilter, FnUniFilter, TrueFilter, UniFilter,
    };
    pub use solverforge_scoring::stream::{UniConstraintBuilder, UniConstraintStream};
}
