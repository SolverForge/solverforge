use solverforge_config::SolverConfig as FacadeSolverConfig;

// Initializes console output if the feature is enabled.
#[inline]
pub fn init_console() {
    #[cfg(feature = "console")]
    solverforge_console::init();
}

#[inline]
pub fn load_solver_config() -> FacadeSolverConfig {
    FacadeSolverConfig::load("solver.toml").unwrap_or_default()
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
pub use solverforge_scoring::{Director, ScoreDirector, SolvableSolution};
pub use tokio::sync::mpsc::UnboundedSender;

// Solver infrastructure
pub use solverforge_solver::builder::{
    bind_scalar_groups, build_search, local_search, CustomSearchPhase, ListVariableSlot,
    LocalSearch, LocalSearchStrategy, RuntimeModel, ScalarGroupBinding, ScalarGroupMemberBinding,
    ScalarVariableSlot, Search, SearchContext, ValueSource, VariableSlot,
};
pub use solverforge_solver::heuristic::selector::{
    DefaultCrossEntityDistanceMeter, DefaultDistanceMeter, FromSolutionEntitySelector,
};
pub use solverforge_solver::manager::{
    KOptPhaseBuilder, ListConstructionPhaseBuilder, PhaseFactory, SolverFactory,
};
pub use solverforge_solver::model_support::PlanningModelSupport;
pub use solverforge_solver::runtime::{build_phases, Construction, RuntimePhase};
pub use solverforge_solver::runtime::{ListVariableEntity, ListVariableMetadata};
pub use solverforge_solver::scope::{ProgressCallback, SolverScope};
pub use solverforge_solver::{
    descriptor_has_bindings, log_solve_start, run_solver, run_solver_with_config, Phase,
    PhaseSequence, SolverEvent, SolverRuntime, SolverTelemetry,
};

// Config
pub use solverforge_config::{PhaseConfig, SolverConfig};

// Stream types needed for macro-generated source methods
pub use solverforge_scoring::stream::filter::{AndUniFilter, FnUniFilter, TrueFilter, UniFilter};
pub use solverforge_scoring::stream::{
    source, ChangeSource, CollectionExtract, SourceExtract, UnassignedEntity, UniConstraintBuilder,
    UniConstraintStream,
};
