//! SolverForge Solver Engine
//!
//! This crate provides the main solver implementation including:
//! - Solver and SolverFactory
//! - Phases (construction heuristic, local search, exhaustive search)
//! - Move system
//! - Termination conditions
//! - Tracing-based structured logging
//! - Configuration wiring (builder module)

pub mod builder;
pub mod operations;
pub mod heuristic;
pub mod manager;
pub mod phase;
pub mod realtime;
pub mod scope;
pub mod solver;
pub mod stats;
pub mod termination;

pub use builder::AcceptorBuilder;
pub use heuristic::{Move, MoveArena, MoveImpl, MoveSelector, MoveSelectorImpl};
pub use manager::{
    Analyzable, ConstraintAnalysis, ConstructionPhaseFactory, KOptPhase,
    KOptPhaseBuilder, ListConstructionPhase, ListConstructionPhaseBuilder, LocalSearchPhaseFactory,
    PhaseFactory, ScoreAnalysis, SolutionManager, Solvable, SolverFactory,
    SolverFactoryBuilder, SolverManager, SolverStatus,
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
        AcceptedCountForager, Acceptor, AcceptorImpl, AcceptorType,
        DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, FirstAcceptedForager,
        GreatDelugeAcceptor, HillClimbingAcceptor, LateAcceptanceAcceptor, LocalSearchConfig,
        LocalSearchForager, LocalSearchPhase, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
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
pub use solver::{MaybeTermination, NoTermination, Solver};
pub use stats::{PhaseStats, SolverStats};
pub use termination::{
    AndTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, MoveCountTermination, OrTermination,
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

pub use operations::VariableOperations;
pub use solver::run_solver;
