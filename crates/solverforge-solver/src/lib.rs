/* SolverForge Solver Engine

This crate provides the main solver implementation including:
- Solver and SolverFactory
- Phases (construction heuristic, local search, exhaustive search)
- Move system
- Termination conditions
- Tracing-based structured logging
- Configuration wiring (builder module)
*/

/* PhantomData<(fn() -> T, ...)> is an intentional pattern to avoid inheriting
trait bounds from phantom type parameters. Clippy's type_complexity lint
triggers on these tuples but the pattern is architecturally required.
*/
#![allow(clippy::type_complexity)]

#[cfg(test)]
pub mod test_utils;

pub mod builder;
pub mod descriptor;
pub mod heuristic;
pub(crate) mod list_placement;
pub mod manager;
pub mod model_support;
pub mod phase;
pub mod planning;
pub mod realtime;
pub mod run;
pub mod runtime;
mod runtime_build_error;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;

pub use builder::{
    local_search, AcceptorBuilder, AnyAcceptor, AnyForager, CustomSearchPhase, ForagerBuilder,
    IntraDistanceAdapter, ListVariableSlot, RuntimeCandidateMetric, RuntimeCandidateMetricBinding,
    RuntimeCandidateMetricRegistry, RuntimeModel, ScalarGroupBinding, ScalarGroupMemberBinding,
    ScalarVariableSlot, Search, SearchContext, ValueSource, VariableSlot,
};
pub use descriptor::{
    build_descriptor_move_selector, descriptor_has_bindings, DescriptorFlatSelector,
    DescriptorLeafSelector, DescriptorMoveUnion, DescriptorPillarChangeMove,
    DescriptorPillarSwapMove, DescriptorRuinRecreateMove, DescriptorSelector,
    DescriptorSelectorNode,
};
pub use heuristic::{
    // K-opt reconnection patterns
    k_opt_reconnection,
    // Selectors
    AllEntitiesSelector,
    // Move types
    ChangeMove,
    ChangeMoveSelector,
    CompositeMove,
    CompoundScalarEdit,
    CompoundScalarMove,
    CrossEntityDistanceMeter,
    CutPoint,
    DefaultCrossEntityDistanceMeter,
    DefaultDistanceMeter,
    DefaultPillarSelector,
    DynamicListChangeMove,
    DynamicListChangeMoveSelector,
    DynamicScalarChangeMove,
    DynamicScalarChangeMoveSelector,
    DynamicScalarNearbyChangeMoveSelector,
    DynamicScalarNearbySwapMoveSelector,
    DynamicScalarSwapMove,
    EntityReference,
    EntitySelector,
    FromSolutionEntitySelector,
    FromSolutionValueSelector,
    KOptConfig,
    KOptMove,
    KOptMoveSelector,
    ListPositionDistanceMeter,
    ListRuinMove,
    ListRuinMoveSelector,
    MimicRecorder,
    MimicRecordingEntitySelector,
    MimicReplayingEntitySelector,
    Move,
    MoveArena,
    MoveSelector,
    NearbyDistanceMeter,
    NearbyEntitySelector,
    NearbyKOptMoveSelector,
    NearbySelectionConfig,
    PerEntitySliceValueSelector,
    Pillar,
    PillarChangeMove,
    PillarSelector,
    PillarSwapMove,
    RuinMove,
    RuinMoveSelector,
    RuinVariableAccess,
    ScalarNeighborhoodBindingError,
    SelectionOrder,
    StaticValueSelector,
    SubPillarConfig,
    SwapMove,
    SwapMoveSelector,
    ValueSelector,
    // Vec union selector
    VecUnionSelector,
};
pub use manager::{
    analyze, Analyzable, ConstraintAnalysis, ConstructionPhaseFactory, KOptPhase, KOptPhaseBuilder,
    ListCheapestInsertionPhase, ListClarkeWrightPhase, ListConstructionPhase,
    ListConstructionPhaseBuilder, ListKOptPhase, ListRegretInsertionPhase, LocalSearchPhaseFactory,
    PhaseFactory, ScoreAnalysis, Solvable, SolverEvent, SolverEventMetadata, SolverFactory,
    SolverFactoryBuilder, SolverLifecycleState, SolverManager, SolverManagerError,
    SolverPanicPayload, SolverRuntime, SolverSnapshot, SolverSnapshotAnalysis, SolverStatus,
    SolverTelemetryDetail, SolverTerminalReason,
};
pub use model_support::PlanningModelSupport;
pub use phase::{
    construction::{
        BestFitForager, ConstructionChoice, ConstructionForager, ConstructionHeuristicConfig,
        ConstructionHeuristicPhase, EntityPlacer, EntityPlacerCursor, FirstFeasibleForager,
        FirstFitForager, ForagerType, Placement, QueuedEntityPlacer,
    },
    exhaustive::{
        BounderType, ExhaustiveSearchConfig, ExhaustiveSearchDecider, ExhaustiveSearchNode,
        ExhaustiveSearchPhase, ExplorationType, FixedOffsetBounder, ScoreBounder, SimpleDecider,
        SoftScoreBounder,
    },
    localsearch::{
        AcceptedCountForager, Acceptor, BestScoreForager, DiversifiedLateAcceptanceAcceptor,
        FirstAcceptedForager, FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager,
        GreatDelugeAcceptor, HardRegressionPolicy, HillClimbingAcceptor, LateAcceptanceAcceptor,
        LocalSearchForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
        SimulatedAnnealingCalibration, StepCountingHillClimbingAcceptor, TabuSearchAcceptor,
    },
    partitioned::{
        ChildPhases, FunctionalPartitioner, PartitionedSearchConfig, PartitionedSearchPhase,
        SolutionPartitioner, ThreadCount,
    },
    sequence::PhaseSequence,
    Phase,
};
pub use planning::{
    ConflictRepair, RepairCandidate, RepairLimits, RepairProvider, ScalarAssignmentRule,
    ScalarCandidate, ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupLimits,
    ScalarTarget,
};
pub use run::{log_solve_start, try_run_solver_with_config_and_search};
pub use runtime::{ListVariableEntity, ListVariableMetadata};
pub use runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
pub use scope::{PhaseScope, SolverScope, StepScope};
pub use solver::{MaybeTermination, NoTermination, SolveResult, Solver};
pub use stats::{
    AppliedMoveTelemetry, MoveTelemetry, PhaseStats, PhaseTelemetry, SelectorTelemetry,
    SolverStats, SolverTelemetry,
};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};
