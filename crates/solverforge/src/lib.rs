//! SolverForge is a constraint solver for planning and scheduling problems:
//! you describe what may change and what makes a plan good or bad, and the
//! solver searches for the best plan it can find in the time you give it.
//!
//! This crate is the facade: it re-exports the model macros, score types,
//! constraint streams, configuration and the [`SolverManager`] that runs
//! solves. The repository README walks through the concepts, and
//! `examples/` holds complete applications.
//!
//! # Quick Start
//!
//! A model is a directory of ordinary Rust modules (`src/domain` in an
//! application). Problem facts are the inputs the solver reads:
//!
//! ```rust,ignore
#![doc = include_str!("../doc/quick_start/employee.rs")]
//! ```
//!
//! Planning entities hold the planning variables the solver assigns. A scalar
//! planning variable stores the index of its value in a solution collection:
//!
//! ```rust,ignore
#![doc = include_str!("../doc/quick_start/shift.rs")]
//! ```
//!
//! The planning solution owns the collections, the score and the
//! constraints, and links the solver configuration (`solver.toml` holds
//! `[termination] seconds_spent_limit = 5`):
//!
//! ```rust,ignore
#![doc = include_str!("../doc/quick_start/schedule.rs")]
//! ```
//!
//! `planning_model!` in the domain's `mod.rs` lists those modules, and
//! [`SolverManager`] solves a problem on a background thread, streaming
//! progress and the final solution as [`SolverEvent`]s. The three files
//! above are this example's `doc/quick_start` directory, so it compiles
//! against exactly the code shown:
//!
//! ```rust,no_run
//! solverforge::planning_model! {
//!     root = "doc/quick_start";
//!
//! #   #[path = "../doc/quick_start/employee.rs"]
//!     mod employee;
//! #   #[path = "../doc/quick_start/schedule.rs"]
//!     mod schedule;
//! #   #[path = "../doc/quick_start/shift.rs"]
//!     mod shift;
//!
//!     pub use employee::Employee;
//!     pub use schedule::Schedule;
//!     pub use shift::Shift;
//! }
//!
//! use solverforge::{SolverEvent, SolverManager};
//!
//! static MANAGER: SolverManager<Schedule> = SolverManager::new();
//!
//! fn main() {
//!     let schedule = Schedule {
//!         employees: ["Amina", "Bruno"]
//!             .iter()
//!             .enumerate()
//!             .map(|(id, name)| Employee { id, name: name.to_string() })
//!             .collect(),
//!         shifts: (0..4)
//!             .map(|id| Shift { id, day: id as u32 / 2, employee_idx: None })
//!             .collect(),
//!         score: None,
//!     };
//!
//!     let (job_id, mut events) = MANAGER.solve(schedule).expect("solver job should start");
//!     while let Some(event) = events.blocking_recv() {
//!         match event {
//!             SolverEvent::Completed { solution, .. } => {
//!                 println!("score: {:?}", solution.score);
//!                 break;
//!             }
//!             SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
//!             _ => {}
//!         }
//!     }
//!     MANAGER.delete(job_id).expect("finished job should delete");
//! }
//! ```

#![warn(missing_docs)]

/* ============================================================================
Attribute Macros
============================================================================
*/

pub use solverforge_macros::{
    planning_entity, planning_model, planning_solution, problem_fact, solverforge_constraints,
};

/* ============================================================================
Score Types
============================================================================
*/

pub use solverforge_config::{
    AcceptorConfig, CandidateTraceConfig, ConstructionHeuristicType, ConstructionObligation,
    EnvironmentMode, ForagerConfig, HardRegressionPolicyConfig, MoveSelectorConfig,
    MoveThreadCount, PhaseConfig, RecreateHeuristicType, SolverConfig, SolverConfigOverride,
    UnionSelectionOrder,
};
pub use solverforge_core::domain::{EntityClassId, ProblemFactClassId, VariableId};
pub use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, ParseableScore, Score,
    ScoreLevel, ScoreParseError, SoftScore,
};
pub use solverforge_core::PlanningSolution;
pub use solverforge_core::{
    ConstraintRef, ConstraintWeightError, ConstraintWeights, ImpactType,
    MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
};

/// Every solver configuration type: phases, move selectors, acceptors,
/// foragers, termination and the configuration error.
pub mod config {
    pub use solverforge_config::*;
}
/// Stock CVRP profile: problem data, distance meters and route hooks.
pub mod cvrp;
/// Solver and phase statistics, telemetry and candidate-trace types.
pub mod stats {
    pub use solverforge_solver::{
        AppliedMoveTelemetry, CandidatePullTelemetry, CandidateTraceCompositeIdentity,
        CandidateTraceConstructionTarget, CandidateTraceCoordinate, CandidateTraceDigest,
        CandidateTraceDisposition, CandidateTraceExecutionPolicy, CandidateTraceExternalDigest,
        CandidateTraceHeader, CandidateTraceIdentity, CandidateTraceInputAttestation,
        CandidateTraceInputProvenance, CandidateTraceInputProvenanceStatus,
        CandidateTraceOperationIdentity, CandidateTracePhaseAttribute, CandidateTracePhasePlan,
        CandidateTraceProvenanceStatus, CandidateTraceQualificationError,
        CandidateTraceQualificationStatus, CandidateTraceSource, CandidateTraceTelemetry,
        MoveTelemetry, PhaseStats, PhaseTelemetry, QualifiedCandidateTraceRunProvenance,
        SelectorTelemetry, SolverStats, SolverTelemetry, Throughput,
        CANDIDATE_TRACE_FORMAT_VERSION,
    };
}
/// Planning-model traits and runtime descriptors (`PlanningSolution`,
/// `SolutionDescriptor`, entity and variable descriptors).
pub mod domain {
    pub use solverforge_core::domain::*;
}
/// Score analysis types returned by constraints and score directors.
pub mod analysis {
    pub use solverforge_scoring::api::analysis::*;
}
/// Built-in constraint implementations that the stream builders produce.
pub mod constraint {
    pub use solverforge_scoring::constraint::*;
}
/// Score directors and their weight modes.
pub mod director {
    pub use solverforge_scoring::director::*;
}
/// Search declaration building blocks behind `SearchContext` and `Search`.
pub mod builder {
    pub use solverforge_solver::{
        bind_scalar_groups, local_search, CustomSearchPhase, ListVariableSlot, RuntimeModel,
        ScalarGroupBinding, ScalarVariableSlot, Search, SearchContext, ValueSource, VariableSlot,
    };
}
/// Moves, move selectors and move cursors for custom search.
pub mod heuristic {
    pub use solverforge_solver::{
        CandidateId, CrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter,
        DefaultDistanceMeter, Move, MoveAffectedEntity, MoveCandidateRef, MoveCandidateUndo,
        MoveCursor, MoveSelector, MoveSelectorIter, MoveStreamContext, MoveTabuScope,
        MoveTabuSignature, MoveUndoIdentity, ScopedEntityTabuToken, ScopedValueTabuToken,
        SequentialCompositeMoveRef,
    };
}
/// Phases and their parts: local search acceptors and foragers, exhaustive
/// search deciders, partitioned search.
pub mod phase {
    pub use solverforge_solver::{
        Acceptor, ChildPhases, ExhaustiveSearchConfig, ExhaustiveSearchDecider,
        ExhaustiveSearchNode, ExhaustiveSearchPhase, ExplorationType, ForagerDecision,
        FunctionalPartitioner, LocalSearchForager, PartitionedSearchConfig, PartitionedSearchPhase,
        Phase, ScoreBounder, SimpleDecider, SolutionPartitioner, ThreadCount,
    };
}
/// Scalar and repair planning types and source-to-target helpers.
pub mod planning;
/// Glob import for domain models and constraint providers.
pub mod prelude;
/// Fluent constraint stream API.
pub mod stream;

#[doc(hidden)]
pub mod __internal;

/* ============================================================================
Constraint API
============================================================================
*/

pub use solverforge_scoring::{
    fixed_weight, hard_weight, ConstraintMetadata, ConstraintResult, ConstraintSet, FixedWeight,
    HardWeight, IncrementalConstraint, IncrementalUniConstraint, ListPrecedenceMakespanConstraint,
    Projection, ProjectionSink, SharedNodeDiagnostics, SharedNodeId, SharedNodeOperation,
    SingleConstraint,
};

/* ============================================================================
Score Director
============================================================================
*/

pub use solverforge_scoring::{Director, ScoreDirector};

/* ============================================================================
Solver
============================================================================
*/

pub use solverforge_bridge as bridge;
pub use solverforge_solver::CrossEntityDistanceMeter;
pub use solverforge_solver::DefaultDistanceMeter;
pub use solverforge_solver::{
    analyze, local_search, Analyzable, AppliedMoveTelemetry, ClosureProblemChange, ConflictRepair,
    ConstraintAnalysis, CustomSearchPhase, ExhaustiveSearchConfig, ExhaustiveSearchPhase,
    ExplorationType, FunctionalPartitioner, MoveTelemetry, PartitionedSearchPhase, PhaseTelemetry,
    ProblemChange, ProblemChangeId, RepairCandidate, RepairLimits, RepairProvider,
    RuntimeBuildError, RuntimeBuildResult, ScalarAssignmentRule, ScalarCandidate,
    ScalarCandidateProvider, ScalarEdit, ScalarGroup, ScalarGroupLimits, ScalarTarget,
    ScoreAnalysis, Search, SearchContext, SelectorTelemetry, SimpleDecider, SolutionPartitioner,
    Solvable, SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverManager,
    SolverManagerError, SolverRuntime, SolverSnapshot, SolverSnapshotAnalysis, SolverStatus,
    SolverTelemetry, SolverTelemetryDetail, SolverTerminalReason, ThreadCount,
};
pub use solverforge_solver::{
    CandidatePullTelemetry, CandidateTraceCompositeIdentity, CandidateTraceConstructionTarget,
    CandidateTraceCoordinate, CandidateTraceDigest, CandidateTraceDisposition,
    CandidateTraceExecutionPolicy, CandidateTraceExternalDigest, CandidateTraceHeader,
    CandidateTraceIdentity, CandidateTraceInputAttestation, CandidateTraceInputProvenance,
    CandidateTraceInputProvenanceStatus, CandidateTraceOperationIdentity,
    CandidateTracePhaseAttribute, CandidateTracePhasePlan, CandidateTraceProvenanceStatus,
    CandidateTraceQualificationError, CandidateTraceQualificationStatus, CandidateTraceSource,
    CandidateTraceTelemetry, QualifiedCandidateTraceRunProvenance, CANDIDATE_TRACE_FORMAT_VERSION,
};
pub use solverforge_solver::{
    PhaseScope, ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope, StepScope,
};

/* ============================================================================
Console Output (feature-gated)
============================================================================
*/

#[cfg(feature = "console")]
pub use solverforge_console as console;
