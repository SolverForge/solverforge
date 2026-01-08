//! SolverForge - A Constraint Solver in Rust
//!
//! SolverForge is a constraint satisfaction/optimization solver inspired by Timefold.
//! It helps you optimize planning and scheduling problems.
//!
//! # Architecture
//!
//! SolverForge uses **zero-erasure** constraint evaluation - all scoring code is
//! fully monomorphized with no `Box<dyn Trait>` in hot paths.
//!
//! # Quick Start
//!
//! ```
//! use solverforge::prelude::*;
//!
//! #[problem_fact]
//! pub struct Employee {
//!     #[planning_id]
//!     pub id: i64,
//!     pub name: String,
//! }
//!
//! #[planning_entity]
//! pub struct Shift {
//!     #[planning_id]
//!     pub id: i64,
//!     #[planning_variable]
//!     pub employee: Option<i64>,
//! }
//!
//! #[planning_solution]
//! pub struct Schedule {
//!     #[problem_fact_collection]
//!     pub employees: Vec<Employee>,
//!     #[planning_entity_collection]
//!     pub shifts: Vec<Shift>,
//!     #[planning_score]
//!     pub score: Option<HardSoftScore>,
//! }
//! ```
//!
//! # Crate Organization
//!
//! - `solverforge-core`: Core types (Score, domain traits)
//! - `solverforge-macros`: Attribute macros
//! - `solverforge-scoring`: Zero-erasure typed constraint infrastructure
//! - `solverforge-solver`: Solver implementation
//! - `solverforge-config`: Configuration system

// Re-export core types
pub use solverforge_core::{
    // Value range providers
    domain::FieldValueRangeProvider,
    // Descriptors
    domain::{
        EntityDescriptor, ProblemFactDescriptor, ShadowVariableKind, SolutionDescriptor,
        TypedEntityExtractor, VariableDescriptor,
    },
    // Domain traits (as trait names, not macros)
    domain::{
        PlanningEntity as PlanningEntityTrait, PlanningId,
        PlanningSolution as PlanningSolutionTrait, ProblemFact as ProblemFactTrait,
    },
    // Score types
    score::{
        BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SimpleScore,
    },
    // Constraint reference
    ConstraintRef,
    ImpactType,
    // Error
    SolverForgeError,
};

// Re-export macros
pub use solverforge_macros::{
    planning_entity, planning_solution, problem_fact, PlanningEntityImpl, PlanningSolutionImpl,
    ProblemFactImpl,
};

// Re-export configuration
pub use solverforge_config::{
    AcceptorConfig, ConstructionHeuristicConfig, ConstructionHeuristicType, EnvironmentMode,
    ForagerConfig, LateAcceptanceConfig, LocalSearchConfig, PhaseConfig, SolverConfig,
    TerminationConfig,
};

// Re-export solver
pub use solverforge_solver::{Solver, SolverFactory};

// Re-export solver manager and phase factories
pub use solverforge_solver::manager::{
    CloneablePhaseFactory, ClosurePhaseFactory, ConstructionPhaseFactory, ConstructionType,
    KOptPhaseBuilder, LocalSearchPhaseFactory, LocalSearchType, SolverManager,
    SolverManagerBuilder, SolverPhaseFactory,
};

// Re-export statistics
pub use solverforge_solver::statistics::{
    PhaseStatistics, ScoreImprovement, SolverStatistics, StatisticsCollector,
};

// Re-export phases
pub use solverforge_solver::{
    AcceptedCountForager,
    Acceptor,
    AcceptorType,
    BestFitForager,
    ConstructionForager,
    ConstructionHeuristicPhase,
    // All acceptors
    DiversifiedLateAcceptanceAcceptor,
    EntityPlacer,
    EntityTabuAcceptor,
    FirstAcceptedForager,
    FirstFitForager,
    ForagerType,
    GreatDelugeAcceptor,
    HillClimbingAcceptor,
    LateAcceptanceAcceptor,
    LocalSearchForager,
    LocalSearchPhase,
    MoveTabuAcceptor,
    Phase,
    QueuedEntityPlacer,
    SimulatedAnnealingAcceptor,
    StepCountingHillClimbingAcceptor,
    TabuSearchAcceptor,
    ValueTabuAcceptor,
    // VND phase
    VndPhase,
};

// Re-export selectors
pub use solverforge_solver::{
    FromSolutionEntitySelector, StaticTypedValueSelector, TypedValueSelector,
};

// Re-export termination
pub use solverforge_solver::{
    AndCompositeTermination, BestScoreFeasibleTermination, BestScoreTermination,
    DiminishedReturnsTermination, OrCompositeTermination, StepCountTermination, Termination,
    TimeTermination, UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

// Re-export scopes and moves
pub use solverforge_solver::{
    k_opt_reconnection,
    ChangeMove,
    ChangeMoveSelector,
    // K-opt move and selectors
    CutPoint,
    EntitySelector,
    KOptConfig,
    KOptMove,
    KOptMoveSelector,
    // List moves and selectors
    ListChangeMove,
    ListChangeMoveSelector,
    ListPositionDistanceMeter,
    Move,
    MoveSelector,
    NearbyKOptMoveSelector,
    PhaseScope,
    SolverScope,
    StepScope,
    SwapMove,
    SwapMoveSelector,
};

// Re-export heuristic module for advanced move/selector access
pub use solverforge_solver::heuristic;

// Re-export zero-erasure constraint infrastructure
pub use solverforge_scoring::{
    ConstraintAnalysis,
    ConstraintJustification,
    ConstraintResult,
    // Constraint set (tuple-based)
    ConstraintSet,
    GroupedUniConstraint,
    IncrementalBiConstraint,
    IncrementalConstraint,
    IncrementalCrossBiConstraint,
    IncrementalPentaConstraint,
    IncrementalQuadConstraint,
    IncrementalTriConstraint,
    // Zero-erasure incremental constraints
    IncrementalUniConstraint,
    RecordingScoreDirector,
    // Score directors
    ScoreDirector,
    // Shadow variable support
    ShadowAwareScoreDirector,
    ShadowVariableSupport,
    // Analysis
    ScoreExplanation,
    SimpleScoreDirector,
    TypedScoreDirector,
};

// Re-export fluent constraint stream API
pub use solverforge_scoring::stream;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use super::{
        // Macros
        planning_entity,
        planning_solution,
        problem_fact,
        BendableScore,
        // Constraints
        ConstraintRef,
        ConstraintSet,
        EntityDescriptor,
        HardMediumSoftScore,
        HardSoftDecimalScore,
        HardSoftScore,
        ImpactType,
        IncrementalBiConstraint,
        IncrementalConstraint,
        IncrementalCrossBiConstraint,
        IncrementalPentaConstraint,
        IncrementalQuadConstraint,
        IncrementalTriConstraint,
        IncrementalUniConstraint,
        PlanningEntityTrait,
        PlanningId,
        // Domain traits
        PlanningSolutionTrait,
        ProblemFactTrait,
        // Scores
        Score,
        SimpleScore,
        SimpleScoreDirector,
        // Descriptors
        SolutionDescriptor,
        // Solver
        Solver,
        SolverConfig,
        SolverFactory,
        // Score directors
        TypedScoreDirector,
    };
    // Fluent constraint API
    pub use super::stream::{joiner, ConstraintFactory};
}
