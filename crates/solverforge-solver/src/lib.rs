//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Event system for monitoring
//! - Configuration wiring (builder module)

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

pub use builder::{AcceptorBuilder, SolverBuilder, TerminationBuilder};
pub use event::{
    CountingEventListener, LoggingEventListener, PhaseLifecycleListener, SolverEventListener,
    SolverEventSupport, StepLifecycleListener,
};
pub use heuristic::{
    // Move types
    ChangeMove, CompositeMove, CutPoint, KOptMove, ListRuinMove, Move, MoveArena,
    PillarChangeMove, PillarSwapMove, RuinMove, SwapMove,
    // K-opt reconnection patterns
    k_opt_reconnection,
    // Selectors
    AllEntitiesSelector, ChangeMoveSelector, DefaultPillarSelector, EntityReference,
    EntitySelector, FromSolutionEntitySelector, FromSolutionTypedValueSelector,
    KOptConfig, KOptMoveSelector, ListPositionDistanceMeter, ListRuinMoveSelector,
    MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector, MoveSelector,
    NearbyDistanceMeter, NearbyEntitySelector, NearbyKOptMoveSelector, NearbySelectionConfig,
    Pillar, PillarSelector, RuinMoveSelector, SelectionOrder, StaticTypedValueSelector,
    SubPillarConfig, SwapMoveSelector, TypedValueSelector,
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
        FunctionalPartitioner, PartitionedSearchConfig, PartitionedSearchPhase, PhaseFactory,
        ScoreDirectorFactory, SolutionPartitioner, ThreadCount,
    },
    vnd::VndPhase,
    Phase,
};
pub use scope::{PhaseScope, SolverScope, StepScope};
pub use solver::{Solver, SolverFactory};
pub use termination::{
    AndCompositeTermination, BestScoreFeasibleTermination, BestScoreTermination,
    OrCompositeTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};
pub use manager::{
    SolverManager, SolverManagerBuilder, LocalSearchType, ConstructionType,
    SolverPhaseFactory, CloneablePhaseFactory, ClosurePhaseFactory,
};
pub use statistics::{PhaseStatistics, ScoreImprovement, SolverStatistics, StatisticsCollector};
