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
pub mod descriptor_standard;
pub mod heuristic;
pub mod list_solver;
pub mod manager;
pub mod phase;
pub mod realtime;
pub mod run;
pub mod runtime;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;
pub mod unified_search;

pub use builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListLeafSelector,
    ListMoveSelectorBuilder,
};
pub use descriptor_standard::{
    build_descriptor_construction, build_descriptor_move_selector, descriptor_has_bindings,
    DescriptorConstruction, DescriptorEitherMove, DescriptorLeafSelector,
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
    CrossEntityDistanceMeter,
    CutPoint,
    DefaultCrossEntityDistanceMeter,
    DefaultDistanceMeter,
    DefaultPillarSelector,
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
    Pillar,
    PillarChangeMove,
    PillarSelector,
    PillarSwapMove,
    RuinMove,
    RuinMoveSelector,
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
    analyze, Analyzable, ConstraintAnalysis, ConstructionPhaseFactory, ConstructionType, KOptPhase,
    KOptPhaseBuilder, ListCheapestInsertionPhase, ListClarkeWrightPhase, ListConstructionPhase,
    ListConstructionPhaseBuilder, ListKOptPhase, ListRegretInsertionPhase, LocalSearchPhaseFactory,
    LocalSearchType, PhaseFactory, ScoreAnalysis, Solvable, SolverEvent, SolverEventMetadata,
    SolverFactory, SolverFactoryBuilder, SolverLifecycleState, SolverManager, SolverManagerError,
    SolverRuntime, SolverSnapshot, SolverSnapshotAnalysis, SolverStatus, SolverTerminalReason,
};
pub use phase::{
    construction::{
        BestFitForager, ConstructionForager, ConstructionHeuristicConfig,
        ConstructionHeuristicPhase, EntityPlacer, FirstFeasibleForager, FirstFitForager,
        ForagerType, Placement, QueuedEntityPlacer,
    },
    dynamic_vnd::DynamicVndPhase,
    exhaustive::{
        BounderType, ExhaustiveSearchConfig, ExhaustiveSearchDecider, ExhaustiveSearchNode,
        ExhaustiveSearchPhase, ExplorationType, FixedOffsetBounder, MoveSequence, ScoreBounder,
        SimpleDecider, SoftScoreBounder,
    },
    localsearch::{
        AcceptedCountForager, Acceptor, AcceptorType, BestScoreForager,
        DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, FirstAcceptedForager,
        FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager, GreatDelugeAcceptor,
        HillClimbingAcceptor, LateAcceptanceAcceptor, LocalSearchConfig, LocalSearchForager,
        LocalSearchPhase, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
        StepCountingHillClimbingAcceptor, TabuSearchAcceptor, ValueTabuAcceptor,
    },
    partitioned::{
        ChildPhases, FunctionalPartitioner, PartitionedSearchConfig, PartitionedSearchPhase,
        SolutionPartitioner, ThreadCount,
    },
    sequence::PhaseSequence,
    vnd::VndPhase,
    Phase,
};
pub use scope::{PhaseScope, SolverScope, StepScope};
pub use solver::{MaybeTermination, NoTermination, SolveResult, Solver};
pub use stats::{PhaseStats, SolverStats, SolverTelemetry};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};
pub use unified_search::{
    build_unified_local_search, build_unified_move_selector, build_unified_vnd, UnifiedLocalSearch,
    UnifiedMove, UnifiedNeighborhood, UnifiedVnd,
};

pub use list_solver::{
    build_list_construction, ListConstruction, ListVariableEntity, ListVariableMetadata,
};
pub use run::{log_solve_start, run_solver};
pub use runtime::{
    build_phases, ListConstructionArgs, RuntimePhase, UnifiedConstruction, UnifiedRuntimePhase,
};
