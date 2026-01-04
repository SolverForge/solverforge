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
    // Score types
    score::{Score, SimpleScore, HardSoftScore, HardSoftDecimalScore, HardMediumSoftScore, BendableScore},
    // Domain traits (as trait names, not macros)
    domain::{
        PlanningSolution as PlanningSolutionTrait,
        PlanningEntity as PlanningEntityTrait,
        ProblemFact as ProblemFactTrait,
        PlanningId,
    },
    // Descriptors
    domain::{SolutionDescriptor, EntityDescriptor, VariableDescriptor, ProblemFactDescriptor, TypedEntityExtractor},
    // Value range providers
    domain::FieldValueRangeProvider,
    // Constraint reference
    ConstraintRef, ImpactType,
    // Error
    SolverForgeError,
};

// Re-export macros
pub use solverforge_macros::{
    planning_entity, planning_solution, problem_fact,
    PlanningEntityImpl, PlanningSolutionImpl, ProblemFactImpl,
};

// Re-export configuration
pub use solverforge_config::{
    SolverConfig, TerminationConfig, PhaseConfig,
    ConstructionHeuristicConfig, LocalSearchConfig,
    AcceptorConfig, ForagerConfig, LateAcceptanceConfig,
    EnvironmentMode, ConstructionHeuristicType,
};

// Re-export solver
pub use solverforge_solver::{Solver, SolverFactory};

// Re-export solver manager and phase factories
pub use solverforge_solver::manager::{
    SolverManager, SolverManagerBuilder, SolverPhaseFactory,
    ConstructionPhaseFactory, LocalSearchPhaseFactory,
    LocalSearchType, ConstructionType,
    CloneablePhaseFactory, ClosurePhaseFactory,
};

// Re-export statistics
pub use solverforge_solver::statistics::{
    PhaseStatistics, ScoreImprovement, SolverStatistics, StatisticsCollector,
};

// Re-export phases
pub use solverforge_solver::{
    Phase,
    ConstructionHeuristicPhase, ConstructionForager, FirstFitForager, BestFitForager,
    ForagerType, QueuedEntityPlacer, EntityPlacer,
    LocalSearchPhase, Acceptor, HillClimbingAcceptor, SimulatedAnnealingAcceptor,
    LocalSearchForager, AcceptedCountForager, AcceptorType,
    // All acceptors
    DiversifiedLateAcceptanceAcceptor, EntityTabuAcceptor, GreatDelugeAcceptor,
    LateAcceptanceAcceptor, MoveTabuAcceptor, StepCountingHillClimbingAcceptor,
    TabuSearchAcceptor, ValueTabuAcceptor,
    // VND phase
    VndPhase,
};

// Re-export selectors
pub use solverforge_solver::{
    FromSolutionEntitySelector, StaticTypedValueSelector, TypedValueSelector,
};

// Re-export termination
pub use solverforge_solver::{Termination, TimeTermination, StepCountTermination, OrCompositeTermination};

// Re-export scopes and moves
pub use solverforge_solver::{
    SolverScope, PhaseScope, StepScope,
    Move, ChangeMove, SwapMove, MoveSelector, ChangeMoveSelector, SwapMoveSelector,
    EntitySelector,
    // K-opt move and selectors
    CutPoint, KOptMove, KOptConfig, KOptMoveSelector, NearbyKOptMoveSelector,
    ListPositionDistanceMeter, k_opt_reconnection,
};

// Re-export heuristic module for advanced move/selector access
pub use solverforge_solver::heuristic;

// Re-export zero-erasure constraint infrastructure
pub use solverforge_scoring::{
    // Zero-erasure incremental constraints
    IncrementalUniConstraint, IncrementalBiConstraint,
    IncrementalCrossBiConstraint, IncrementalTriConstraint,
    IncrementalQuadConstraint, IncrementalPentaConstraint,
    GroupedUniConstraint,
    // Constraint set (tuple-based)
    ConstraintSet, IncrementalConstraint, ConstraintResult,
    // Score directors
    ScoreDirector, SimpleScoreDirector, TypedScoreDirector,
    RecordingScoreDirector,
    // Analysis
    ScoreExplanation, ConstraintAnalysis, ConstraintJustification,
};

// Re-export fluent constraint stream API
pub use solverforge_scoring::stream;

/// Prelude module for convenient imports.
pub mod prelude {
    pub use super::{
        // Scores
        Score, SimpleScore, HardSoftScore, HardSoftDecimalScore, HardMediumSoftScore, BendableScore,
        // Domain traits
        PlanningSolutionTrait, PlanningEntityTrait, ProblemFactTrait, PlanningId,
        // Solver
        Solver, SolverFactory, SolverConfig,
        // Descriptors
        SolutionDescriptor, EntityDescriptor,
        // Macros
        planning_entity, planning_solution, problem_fact,
        // Constraints
        ConstraintRef, ImpactType,
        IncrementalUniConstraint, IncrementalBiConstraint,
        IncrementalCrossBiConstraint, IncrementalTriConstraint,
        IncrementalQuadConstraint, IncrementalPentaConstraint,
        IncrementalConstraint, ConstraintSet,
        // Score directors
        TypedScoreDirector, SimpleScoreDirector,
    };
    // Fluent constraint API
    pub use super::stream::{ConstraintFactory, joiner};
}
