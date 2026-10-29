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
    EntityCollectionExtractor, EntityDescriptor, PlanningEntity, PlanningId, PlanningSolution,
    ProblemFact, ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor, ValueRangeType,
    VariableDescriptor,
};

// Scoring
pub use solverforge_scoring::api::{
    ConstraintSet, ConstraintSetChain, ConstraintSetSource, OrderedConstraintSetChain,
};
pub use solverforge_scoring::{Director, ScoreDirector, SolvableSolution};

// Solver infrastructure
pub use solverforge_solver::DefaultCrossEntityDistanceMeter;
pub use solverforge_solver::PlanningModelSupport;
pub use solverforge_solver::{
    bind_scalar_groups, CustomSearchPhase, ListVariableSlot, ProgressCallback, RuntimeModel,
    ScalarGroupBinding, ScalarVariableSlot, Search, SearchContext, SolverScope, ValueSource,
    VariableSlot,
};
pub use solverforge_solver::{
    log_solve_start, try_run_solver_with_config_and_search,
    try_run_weighted_solver_with_config_and_search, RuntimeBuildResult,
};
pub use solverforge_solver::{ListVariableEntity, ListVariableMetadata};

// Config
pub use solverforge_config::SolverConfig;

// Stream types needed for macro-generated source methods
pub use solverforge_scoring::stream::filter::TrueFilter;
pub use solverforge_scoring::stream::{
    source, ChangeSource, SourceExtract, UnassignedEntity, UniConstraintStream,
};
