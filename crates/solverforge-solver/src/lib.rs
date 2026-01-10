//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Event system for monitoring
//! - Configuration wiring (builder module)

pub mod basic;
pub mod builder;
pub mod event;
pub mod heuristic;
pub mod manager;
pub mod phase;
pub mod realtime;
pub mod scope;
pub mod solver;
pub mod statistics;
pub mod termination;

pub use builder::AcceptorBuilder;
pub use event::{
    CountingEventListener, LoggingEventListener, PhaseLifecycleListener, SolverEventListener,
    SolverEventSupport, StepLifecycleListener,
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
    CutPoint,
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
};
pub use manager::{
    ConstructionPhaseFactory, ConstructionType, KOptPhase, KOptPhaseBuilder, ListConstructionPhase,
    ListConstructionPhaseBuilder, LocalSearchPhaseFactory, LocalSearchType, PhaseFactory,
    SolverManager, SolverManagerBuilder,
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
        SimpleDecider, SimpleScoreBounder,
    },
    localsearch::{
        AcceptedCountForager, Acceptor, AcceptorType, DiversifiedLateAcceptanceAcceptor,
        EntityTabuAcceptor, FirstAcceptedForager, GreatDelugeAcceptor, HillClimbingAcceptor,
        LateAcceptanceAcceptor, LocalSearchConfig, LocalSearchForager, LocalSearchPhase,
        MoveTabuAcceptor, SimulatedAnnealingAcceptor, StepCountingHillClimbingAcceptor,
        TabuSearchAcceptor, ValueTabuAcceptor,
    },
    partitioned::{
        ChildPhases, FunctionalPartitioner, PartitionedSearchConfig, PartitionedSearchPhase,
        SolutionPartitioner, ThreadCount,
    },
    vnd::VndPhase,
    Phase,
};
pub use scope::{PhaseScope, SolverScope, StepScope};
pub use solver::{MaybeTermination, NoTermination, Solver, SolverFactory};
pub use statistics::{PhaseStatistics, ScoreImprovement, SolverStatistics, StatisticsCollector};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

pub use basic::run_solver;
