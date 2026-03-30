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
pub mod mixed_stock;
pub mod phase;
pub mod problem_spec;
pub mod realtime;
pub mod run;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod stock;
pub mod termination;

pub use builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder, ListContext, ListLeafSelector,
    ListMoveSelectorBuilder, StandardContext, StandardLeafSelector, StandardMoveSelectorBuilder,
};
pub use descriptor_standard::{
    build_descriptor_construction, build_descriptor_local_search, build_descriptor_move_selector,
    build_descriptor_vnd, descriptor_has_bindings, DescriptorConstruction, DescriptorEitherMove,
    DescriptorLeafSelector, DescriptorLocalSearch, DescriptorVnd, SeedBestSolutionPhase,
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
    FromSolutionTypedValueSelector,
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
    StaticTypedValueSelector,
    SubPillarConfig,
    SwapMove,
    SwapMoveSelector,
    TypedValueSelector,
    // Vec union selector
    VecUnionSelector,
};
pub use manager::{
    analyze, Analyzable, ConstraintAnalysis, ConstructionPhaseFactory, ConstructionType, KOptPhase,
    KOptPhaseBuilder, ListCheapestInsertionPhase, ListClarkeWrightPhase, ListConstructionPhase,
    ListConstructionPhaseBuilder, ListKOptPhase, ListRegretInsertionPhase, LocalSearchPhaseFactory,
    LocalSearchType, PhaseFactory, ScoreAnalysis, Solvable, SolverEvent, SolverFactory,
    SolverFactoryBuilder, SolverManager, SolverStatus,
};
pub use mixed_stock::{
    build_mixed_local_search, build_mixed_move_selector, build_mixed_vnd, MixedNeighborhood,
    MixedStockLocalSearch, MixedStockMove, MixedStockVnd,
};
pub use phase::{
    construction::{
        BestFitForager, ConstructionForager, ConstructionHeuristicConfig,
        ConstructionHeuristicPhase, EntityPlacer, FirstFeasibleForager, FirstFitForager,
        ForagerType, Placement, QueuedEntityPlacer,
    },
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
    stock_vnd::StockVndPhase,
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

pub use list_solver::{
    build_list_construction, build_list_local_search, ListConstruction, ListLocalSearch,
    StockListEntity, StockListVariableMetadata,
};
pub use problem_spec::ProblemSpec;
pub use run::{log_stock_solve_start, run_solver, run_stock_solver};
pub use stock::{
    build_mixed_stock_phases, build_standard_stock_phases, MixedStockConstructionArgs,
    StandardStockPhase, StockPhase, UnifiedMixedStockPhase,
};
