//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Tracing-based structured logging
//! - Configuration wiring (builder module)

// PhantomData<(fn() -> T, ...)> is an intentional pattern to avoid inheriting
// trait bounds from phantom type parameters. Clippy's type_complexity lint
// triggers on these tuples but the pattern is architecturally required.
#![allow(clippy::type_complexity)]

#[cfg(test)]
pub mod test_utils;

pub mod basic;
pub mod builder;
pub mod heuristic;
pub mod list_solver;
pub mod manager;
pub mod phase;
pub mod problem_spec;
pub mod realtime;
pub mod run;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;

pub use builder::{
    AcceptorBuilder, AnyAcceptor, AnyForager, BasicContext, BasicLeafSelector,
    BasicMoveSelectorBuilder, ForagerBuilder, ListContext, ListLeafSelector,
    ListMoveSelectorBuilder,
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
    ListConstructionPhaseBuilder, ListRegretInsertionPhase, LocalSearchPhaseFactory,
    LocalSearchType, PhaseFactory, ScoreAnalysis, Solvable, SolverFactory, SolverFactoryBuilder,
    SolverManager, SolverStatus,
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
    vnd::VndPhase,
    Phase,
};
pub use scope::{PhaseScope, SolverScope, StepScope};
pub use solver::{MaybeTermination, NoTermination, SolveResult, Solver};
pub use stats::{PhaseStats, SolverStats};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

pub use basic::BasicSpec;
pub use list_solver::ListSpec;
pub use problem_spec::ProblemSpec;
pub use run::run_solver;
