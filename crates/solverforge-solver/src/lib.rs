//! The SolverForge solver engine.
//!
//! Applications use it through the `solverforge` facade. It contains:
//! - [`SolverManager`], which runs solve jobs and streams [`SolverEvent`]s;
//! - the phases: construction heuristics, local search, exhaustive search,
//!   partitioned search and the list construction heuristics;
//! - the [`Move`] and [`MoveSelector`] contracts custom search builds on;
//! - termination conditions, and [`SearchContext`], which turns a
//!   `solverforge_config::SolverConfig` into phases;
//! - the compiled [`RuntimeModel`] the generated model code solves through.
//!
//! Every supported item is exported once, at the crate root.

/* PhantomData<(fn() -> T, ...)> is an intentional pattern to avoid inheriting
trait bounds from phantom type parameters. Clippy's type_complexity lint
triggers on these tuples but the pattern is architecturally required.
*/
#![allow(clippy::type_complexity)]
#![warn(unreachable_pub)]

#[cfg(test)]
mod test_utils;

mod builder;
mod constraint_weights;
mod descriptor;
mod heuristic;
mod list_placement;
mod manager;
mod model_support;
mod phase;
mod pinning;
mod planning;
mod realtime;
mod run;
mod runtime;
mod runtime_build_error;
mod scope;
mod solver;
mod stats;
mod termination;

pub use builder::context::list::ListVariableSlot;
pub use builder::context::model::RuntimeModel;
pub use builder::context::model::VariableSlot;
pub use builder::context::scalar::group::bind_scalar_groups;
pub use builder::context::scalar::group::ScalarGroupBinding;
pub use builder::context::scalar::value_source::ValueSource;
pub use builder::context::scalar::variable::ScalarVariableSlot;
pub use builder::search::custom::CustomSearchPhase;
pub use builder::search::local_search;
pub use builder::search::Search;
pub use builder::search::SearchContext;
pub use heuristic::r#move::arena::MoveArena;
pub use heuristic::r#move::composite::SequentialCompositeMoveRef;
pub use heuristic::r#move::metadata::MoveTabuScope;
pub use heuristic::r#move::metadata::MoveTabuSignature;
pub use heuristic::r#move::metadata::MoveUndoIdentity;
pub use heuristic::r#move::metadata::ScopedEntityTabuToken;
pub use heuristic::r#move::metadata::ScopedValueTabuToken;
pub use heuristic::r#move::traits::Move;
pub use heuristic::r#move::traits::MoveAffectedEntity;
pub use heuristic::selector::k_opt::distance_meter::DefaultDistanceMeter;
pub use heuristic::selector::move_selector::CandidateId;
pub use heuristic::selector::move_selector::MoveCandidateRef;
pub use heuristic::selector::move_selector::MoveCandidateUndo;
pub use heuristic::selector::move_selector::MoveCursor;
pub use heuristic::selector::move_selector::MoveSelector;
pub use heuristic::selector::move_selector::MoveSelectorIter;
pub use heuristic::selector::move_selector::MoveStreamContext;
pub use heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
pub use heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
pub use manager::solution_manager::analyze;
pub use manager::solution_manager::Analyzable;
pub use manager::solution_manager::ConstraintAnalysis;
pub use manager::solution_manager::ScoreAnalysis;
pub use manager::solver_manager::manager::Solvable;
pub use manager::solver_manager::manager::SolverManager;
pub use manager::solver_manager::problem_change::ProblemChangeId;
pub use manager::solver_manager::runtime::SolverPanicPayload;
pub use manager::solver_manager::runtime::SolverRuntime;
pub use manager::solver_manager::types::SolverEvent;
pub use manager::solver_manager::types::SolverEventMetadata;
pub use manager::solver_manager::types::SolverLifecycleState;
pub use manager::solver_manager::types::SolverManagerError;
pub use manager::solver_manager::types::SolverSnapshot;
pub use manager::solver_manager::types::SolverSnapshotAnalysis;
pub use manager::solver_manager::types::SolverStatus;
pub use manager::solver_manager::types::SolverTelemetryDetail;
pub use manager::solver_manager::types::SolverTerminalReason;
#[doc(hidden)]
pub use model_support::PlanningModelSupport;
pub use phase::exhaustive::bounder::ScoreBounder;
pub use phase::exhaustive::config::ExhaustiveSearchConfig;
pub use phase::exhaustive::decider::ExhaustiveSearchDecider;
pub use phase::exhaustive::decider::SimpleDecider;
pub use phase::exhaustive::exploration_type::ExplorationType;
pub use phase::exhaustive::node::ExhaustiveSearchNode;
pub use phase::exhaustive::phase::ExhaustiveSearchPhase;
pub use phase::localsearch::acceptor::traits::Acceptor;
pub use phase::localsearch::forager::ForagerDecision;
pub use phase::localsearch::forager::LocalSearchForager;
pub use phase::partitioned::child_phases::ChildPhases;
pub use phase::partitioned::config::PartitionedSearchConfig;
pub use phase::partitioned::partitioner::FunctionalPartitioner;
pub use phase::partitioned::partitioner::SolutionPartitioner;
pub use phase::partitioned::partitioner::ThreadCount;
pub use phase::partitioned::phase::PartitionedSearchPhase;
pub use phase::traits::Phase;
pub use planning::conflict_repair::ConflictRepair;
pub use planning::conflict_repair::RepairCandidate;
pub use planning::conflict_repair::RepairLimits;
pub use planning::conflict_repair::RepairProvider;
pub use planning::scalar::assignment::ScalarAssignmentRule;
pub use planning::scalar::candidate::ScalarCandidate;
pub use planning::scalar::candidate::ScalarCandidateProvider;
pub use planning::scalar::candidate::ScalarEdit;
pub use planning::scalar::group::ScalarGroup;
pub use planning::scalar::group::ScalarGroupLimits;
pub use planning::scalar::target::ScalarTarget;
pub use realtime::problem_change::ClosureProblemChange;
pub use realtime::problem_change::ProblemChange;
pub use run::log_solve_start;
pub use run::try_run_solver_with_config_and_search;
pub use run::weight_modes::try_run_weighted_solver_with_config_and_search;
pub use runtime::ListVariableEntity;
pub use runtime::ListVariableMetadata;
pub use runtime_build_error::RuntimeBuildError;
pub use runtime_build_error::RuntimeBuildResult;
pub use scope::phase::PhaseScope;
pub use scope::solver::ProgressCallback;
pub use scope::solver::SolverProgressKind;
pub use scope::solver::SolverProgressRef;
pub use scope::solver::SolverScope;
pub use scope::step::StepScope;
pub use stats::candidate_trace::identity::CandidateTraceCompositeIdentity;
pub use stats::candidate_trace::identity::CandidateTraceConstructionTarget;
pub use stats::candidate_trace::identity::CandidateTraceCoordinate;
pub use stats::candidate_trace::identity::CandidateTraceIdentity;
pub use stats::candidate_trace::identity::CandidateTraceOperationIdentity;
pub use stats::candidate_trace::plan::CandidateTraceExecutionPolicy;
pub use stats::candidate_trace::plan::CandidateTracePhaseAttribute;
pub use stats::candidate_trace::plan::CandidateTracePhasePlan;
pub use stats::candidate_trace::provenance::CandidateTraceExternalDigest;
pub use stats::candidate_trace::provenance::CandidateTraceInputAttestation;
pub use stats::candidate_trace::provenance::CandidateTraceInputProvenance;
pub use stats::candidate_trace::provenance::CandidateTraceInputProvenanceStatus;
pub use stats::candidate_trace::provenance::CandidateTraceProvenanceStatus;
pub use stats::candidate_trace::pull::CandidatePullTelemetry;
pub use stats::candidate_trace::pull::CandidateTraceDisposition;
pub use stats::candidate_trace::pull::CandidateTraceSource;
pub use stats::candidate_trace::CandidateTraceDigest;
pub use stats::candidate_trace::CandidateTraceHeader;
pub use stats::candidate_trace::CandidateTraceTelemetry;
pub use stats::candidate_trace::CANDIDATE_TRACE_FORMAT_VERSION;
pub use stats::candidate_trace_qualified::CandidateTraceQualificationError;
pub use stats::candidate_trace_qualified::CandidateTraceQualificationStatus;
pub use stats::candidate_trace_qualified::QualifiedCandidateTraceRunProvenance;
pub use stats::phase::PhaseStats;
pub use stats::solver::SolverStats;
pub use stats::telemetry::AppliedMoveTelemetry;
pub use stats::telemetry::MoveTelemetry;
pub use stats::telemetry::PhaseTelemetry;
pub use stats::telemetry::SelectorTelemetry;
pub use stats::telemetry::SolverTelemetry;
pub use stats::telemetry::Throughput;

/// Internal types the selector benchmark drives directly. Hidden and not
/// part of the supported API: no compatibility promise covers them.
#[doc(hidden)]
pub mod __bench {
    pub use crate::heuristic::r#move::change::ChangeMove;
    pub use crate::heuristic::r#move::list_change::ListChangeMove;
    pub use crate::heuristic::selector::decorator::cartesian_product::CartesianProductSelector;
    pub use crate::heuristic::selector::decorator::filtering::FilteringMoveSelector;
    pub use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    pub use crate::heuristic::selector::k_opt::config::KOptConfig;
    pub use crate::heuristic::selector::k_opt::selector::KOptMoveSelector;
    pub use crate::heuristic::selector::list_change::ListChangeMoveSelector;
    pub use crate::heuristic::selector::list_swap::ListSwapMoveSelector;
    pub use crate::heuristic::selector::move_selector::ChangeMoveSelector;
    pub use crate::heuristic::selector::nearby_list_change::NearbyListChangeMoveSelector;
    pub use crate::heuristic::selector::nearby_list_swap::NearbyListSwapMoveSelector;
    pub use crate::heuristic::selector::ruin::RuinMoveSelector;
    pub use crate::heuristic::selector::ruin::RuinVariableAccess;
    pub use crate::heuristic::selector::sublist_change::SublistChangeMoveSelector;
    pub use crate::heuristic::selector::sublist_swap::SublistSwapMoveSelector;
    pub use crate::heuristic::selector::value_selector::StaticValueSelector;
    pub use crate::phase::construction::forager::FirstFitForager;
    pub use crate::phase::construction::phase::ConstructionHeuristicPhase;
    pub use crate::phase::construction::placer::EntityPlacer;
    pub use crate::phase::construction::placer::EntityPlacerCursor;
    pub use crate::phase::construction::placer::QueuedEntityPlacer;
}

/// Solver internals other SolverForge crates build on: the host-provider
/// contracts `solverforge-bridge` re-exports for host-language bindings.
/// Hidden and not part of the supported API.
#[doc(hidden)]
pub mod __private {
    pub use crate::builder::context::candidate_metric::RuntimeCandidateMetric;
    pub use crate::builder::context::candidate_metric::RuntimeCandidateMetricBinding;
    pub use crate::builder::context::candidate_metric::RuntimeCandidateMetricRegistry;
    pub use crate::builder::context::provider::registry::RuntimeProviderRegistry;
    pub use crate::builder::context::provider::types::ProviderResolutionError;
    pub use crate::builder::context::provider::types::RawProviderCandidate;
    pub use crate::builder::context::provider::types::RawProviderEdit;
    pub use crate::builder::context::provider::types::RuntimeConflictRepairProviderBinding;
    pub use crate::builder::context::provider::types::RuntimeHostCompoundProvider;
    pub use crate::builder::context::provider::types::RuntimeHostProviderErrorBoundary;
    pub use crate::builder::context::provider::types::RuntimeProviderLimits;
    pub use crate::builder::context::provider::types::RuntimeScalarGroupProviderBinding;
    pub use crate::builder::search::custom::NoDynamicExtensions;
}
