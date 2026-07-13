//! Public dynamic runner helpers backed by the compiled runtime graph.
//!
//! Bindings pass one already-compiled runtime model. This bridge never accepts
//! a phase-builder closure and exposes one solver runner.

use std::fmt::Debug;
use std::hash::Hash;

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::ConstraintSet;
use solverforge_solver::builder::{NoDynamicExtensions, Search, SearchContext};
use solverforge_solver::stats::QualifiedCandidateTraceRunProvenance;
use solverforge_solver::{
    try_run_solver_with_config_and_search, CrossEntityDistanceMeter, RuntimeBuildResult,
    RuntimeModel, SolverRuntime,
};

/// Dynamic authoring transferred to the shared graph compiler. The distinct
/// empty extension registry makes custom and partitioned declarations a
/// structural error rather than a host-language fallback.
struct DynamicSearchDeclaration<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    context: SearchContext<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> Search<S, V, DM, IDM> for DynamicSearchDeclaration<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
{
    type Extensions = NoDynamicExtensions;

    fn into_runtime_parts(self) -> (SearchContext<S, V, DM, IDM>, Self::Extensions) {
        (self.context, NoDynamicExtensions)
    }
}

fn dynamic_search_declaration<S, V, DM, IDM>(
    config: &SolverConfig,
    descriptor: SolutionDescriptor,
    model: RuntimeModel<S, V, DM, IDM>,
) -> RuntimeBuildResult<DynamicSearchDeclaration<S, V, DM, IDM>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
{
    Ok(DynamicSearchDeclaration {
        context: SearchContext::try_new(descriptor, model, config.random_seed)?,
    })
}

/// Runs a dynamic model through the one compiled runtime path.
///
/// The one model value is consumed by the canonical compiled runner. There is
/// no host-language construction branch or deferred alternate execution path.
#[allow(clippy::too_many_arguments)]
pub fn try_run_dynamic_solver_with_config_parts<S, C, V, DM, IDM>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    model: RuntimeModel<S, V, DM, IDM>,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + Sync + 'static,
{
    try_run_solver_with_config_and_search(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        default_time_limit_secs,
        log_scale,
        qualified_candidate_trace_provenance,
        move |config, descriptor| dynamic_search_declaration(config, descriptor, model),
    )
}
