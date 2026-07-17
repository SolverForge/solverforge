/* High-level solver management with zero-erasure API.

# Zero-Erasure Design

All types flow through generics - no Box, Arc, or dyn anywhere.
Runtime configuration from TOML/YAML is handled by the macro layer
which generates concrete types at compile time.
*/

mod builder;
mod phase_factory;
mod phase_factory_trait;
mod solution_manager;
mod solver_factory;
mod solver_manager;

#[cfg(test)]
mod builder_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod mod_tests_integration;

pub use builder::{SolverBuildError, SolverFactoryBuilder};
pub(crate) use phase_factory::{
    run_cheapest, run_clarke_wright, run_list_k_opt, run_regret, run_round_robin,
    PhaseCheapestInsertionObserver, ScoredListConstructionAccess,
};
#[cfg(test)]
pub(crate) use phase_factory::{CheapestInsertionObserver, CheapestInsertionTrial};
pub use phase_factory::{
    ConstructionPhaseFactory, KOptPhase, KOptPhaseBuilder, ListCheapestInsertionPhase,
    ListClarkeWrightPhase, ListConstructionPhase, ListConstructionPhaseBuilder, ListKOptPhase,
    ListRegretInsertionPhase, LocalSearchPhaseFactory,
};
pub use phase_factory_trait::PhaseFactory;
pub use solution_manager::{analyze, Analyzable, ConstraintAnalysis, ScoreAnalysis};
pub use solver_factory::{solver_factory_builder, SolverFactory};
pub use solver_manager::{
    Solvable, SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverManager,
    SolverManagerError, SolverPanicPayload, SolverRuntime, SolverSnapshot, SolverSnapshotAnalysis,
    SolverStatus, SolverTelemetryDetail, SolverTerminalReason,
};
