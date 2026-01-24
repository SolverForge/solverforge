//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Tracing-based structured logging
//! - Configuration wiring (builder module)

pub mod config_bridge;
pub mod heuristic;
pub mod manager;
pub mod phase;
pub mod public_api;
pub mod realtime;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;

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
    Analyzable, ChangeConstructionPhase, ChangeConstructionPhaseBuilder, ChangeLocalSearchPhase,
    ChangeLocalSearchPhaseBuilder, ConstraintAnalysis, ConstructionPhaseFactory, ConstructionType,
    KOptPhase, KOptPhaseBuilder, ListConstructionPhase, ListConstructionPhaseBuilder,
    LocalSearchPhaseFactory, LocalSearchType, PhaseFactory, ScoreAnalysis, SolutionManager,
    Solvable, SolverFactory, SolverFactoryBuilder, SolverManager, SolverStatus,
};
pub use phase::{
    construction::{
        BestFitForager, ConstructionForager, ConstructionForagerImpl, ConstructionHeuristicPhase,
        EntityPlacer, FirstFeasibleForager, FirstFitForager, Placement, QueuedEntityPlacer,
    },
    exhaustive::{
        BounderType, ExhaustiveSearchConfig, ExhaustiveSearchDecider, ExhaustiveSearchNode,
        ExhaustiveSearchPhase, ExplorationType, FixedOffsetBounder, MoveSequence, ScoreBounder,
        SimpleDecider, SimpleScoreBounder,
    },
    localsearch::{
        AcceptedCountForager, Acceptor, AcceptorImpl, DiversifiedLateAcceptanceAcceptor,
        EntityTabuAcceptor, FirstAcceptedForager, FirstBestScoreImprovingForager,
        FirstLastStepScoreImprovingForager, GreatDelugeAcceptor, HillClimbingAcceptor,
        LateAcceptanceAcceptor, LocalSearchForager, LocalSearchForagerImpl, LocalSearchPhase,
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
pub use solver::{MaybeTermination, NoTermination, Solver};
pub use stats::{PhaseStats, SolverStats};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};
