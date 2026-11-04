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
use solverforge_solver::__private::NoDynamicExtensions;
use solverforge_solver::QualifiedCandidateTraceRunProvenance;
use solverforge_solver::{
    try_run_solver_with_config_and_search, CrossEntityDistanceMeter, RuntimeBuildError,
    RuntimeBuildResult, RuntimeModel, ScoreAnalysis, SolverRuntime,
};
use solverforge_solver::{Search, SearchContext};

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
        context: SearchContext::new(descriptor, model, config.resolved_random_seed())?,
    })
}

/// One named constraint set a dynamic solve can score with.
///
/// `score_director.constraint_provider` selects a provider by [`Self::name`];
/// when it is omitted the first provider passed to
/// [`try_run_dynamic_solver_with_config_parts`] is used.
pub struct DynamicConstraintProvider<'a, S: PlanningSolution, C> {
    /// The name `score_director.constraint_provider` selects this provider by.
    pub name: &'a str,
    /// Builds the constraint set for a solution.
    pub constraints: fn(&S) -> C,
    /// Analyzes a solution with this provider's constraint set; used by
    /// `SolverManager::analyze_snapshot` for solves that select it.
    pub analyze: fn(&S) -> ScoreAnalysis<S::Score>,
}

impl<S: PlanningSolution, C> Clone for DynamicConstraintProvider<'_, S, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: PlanningSolution, C> Copy for DynamicConstraintProvider<'_, S, C> {}

/// The provider `config` selects: the named one, or the first when
/// `score_director.constraint_provider` is omitted.
fn selected_provider<'p, 'n, S: PlanningSolution, C>(
    config: &SolverConfig,
    providers: &'p [DynamicConstraintProvider<'n, S, C>],
) -> RuntimeBuildResult<&'p DynamicConstraintProvider<'n, S, C>> {
    let Some(default) = providers.first() else {
        return Err(RuntimeBuildError::declaration(
            "a dynamic solve needs at least one constraint provider",
        ));
    };
    let Some(requested) = config.constraint_provider() else {
        return Ok(default);
    };
    providers
        .iter()
        .find(|provider| provider.name == requested)
        .ok_or_else(|| RuntimeBuildError::UnknownConstraintProvider {
            requested: requested.to_string(),
            declared: providers
                .iter()
                .map(|provider| provider.name.to_string())
                .collect(),
        })
}

/// Runs a dynamic model through the one compiled runtime path.
///
/// The one model value is consumed by the canonical compiled runner. There is
/// no host-language construction branch or deferred alternate execution path.
///
/// `constraint_providers` lists the constraint sets the solve can score with;
/// `score_director.constraint_provider` selects one by name, and the first is
/// used when it is omitted. A name that matches no provider fails with
/// [`RuntimeBuildError::UnknownConstraintProvider`] before solving starts. The
/// selected provider builds the constraint set for the working solution and
/// for every evaluator replica, so `move_thread_count` evaluates dynamic
/// models in parallel, and its `analyze` serves snapshot analysis unless
/// `score_director.constraint_match_enabled` is `false`.
#[allow(clippy::too_many_arguments)]
pub fn try_run_dynamic_solver_with_config_parts<S, C, V, DM, IDM>(
    solution: S,
    constraint_providers: &[DynamicConstraintProvider<'_, S, C>],
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
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
    let provider = *selected_provider(&config, constraint_providers)?;
    runtime.bind_snapshot_analysis(
        config
            .constraint_match_enabled()
            .then_some(provider.analyze),
    );
    try_run_solver_with_config_and_search(
        solution,
        provider.constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        log_scale,
        qualified_candidate_trace_provenance,
        move |config, descriptor| dynamic_search_declaration(config, descriptor, model),
    )
}
