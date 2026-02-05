//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Tracing-based structured logging
//! - Configuration wiring (builder module)

#[cfg(test)]
pub mod test_utils;

pub mod basic;
pub mod builder;
pub mod heuristic;
pub mod manager;
pub mod phase;
pub mod realtime;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;

#[cfg(test)]
pub mod test_utils;

pub use builder::AcceptorBuilder;
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
    Analyzable, ConstraintAnalysis, ConstructionPhaseFactory, ConstructionType, KOptPhase,
    KOptPhaseBuilder, ListConstructionPhase, ListConstructionPhaseBuilder, LocalSearchPhaseFactory,
    LocalSearchType, PhaseFactory, ScoreAnalysis, SolutionManager, Solvable, SolverFactory,
    SolverFactoryBuilder, SolverManager, SolverStatus,
};
pub use phase::basic::{BasicConstructionPhase, BasicLocalSearchPhase};
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
pub use solver::{MaybeTermination, NoTermination, Solver};
pub use stats::{PhaseStats, SolverStats};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

pub use basic::{run_solver, run_solver_with_channel};
