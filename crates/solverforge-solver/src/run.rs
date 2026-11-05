//! Solver entry point.

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::time::Duration;

use solverforge_config::{SolverConfig, TerminationConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{
    requires_weighted_mode, ConstraintSet, Director, ReplicaDescriptor, ScoreDirector,
    UnweightedMode, WeightMode, WeightedMode,
};
use tracing::info;

use crate::builder::{RuntimeExtensionRegistry, Search, SearchContext};
use crate::constraint_weights::validate_constraint_weights;
use crate::manager::{SolverRuntime, SolverTerminalReason};
use crate::phase::Phase;
use crate::runtime::compiler::executor::{
    take_runtime_execution_failure, CompiledRuntimePhaseRunner,
};
use crate::runtime::compiler::{compile_runtime_graph, CompiledRuntimeExecutor, RuntimeGraphInput};
use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
use crate::scope::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
use crate::solver::Solver;
use crate::stats::{
    format_duration, whole_units_per_second, CandidateTraceExecutionPolicy,
    QualifiedCandidateTraceRunProvenance,
};
use crate::termination::{
    BestScoreTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

pub(crate) mod termination;
use termination::{build_solver_termination, ConfiguredSolverTermination};
pub(crate) use termination::{
    configured_best_score_limit, configured_execution_policy, parse_configured_termination,
};

#[derive(Clone)]
pub(crate) struct ChannelProgressCallback<S: PlanningSolution> {
    runtime: SolverRuntime<S>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> ChannelProgressCallback<S> {
    fn new(runtime: SolverRuntime<S>) -> Self {
        Self {
            runtime,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> fmt::Debug for ChannelProgressCallback<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChannelProgressCallback").finish()
    }
}

impl<S: PlanningSolution> ProgressCallback<S> for ChannelProgressCallback<S> {
    fn invoke(&self, progress: SolverProgressRef<'_, S>) {
        match progress.kind {
            SolverProgressKind::Progress => {
                self.runtime.emit_progress(
                    progress.current_score.copied(),
                    progress.best_score.copied(),
                    progress.telemetry.clone(),
                );
            }
            SolverProgressKind::BestSolution => {
                if let (Some(solution), Some(score)) = (progress.solution, progress.best_score) {
                    self.runtime.emit_best_solution(
                        (*solution).clone(),
                        progress.current_score.copied(),
                        *score,
                        progress.telemetry.clone(),
                    );
                }
            }
        }
    }
}

/// Logs the `solve_start` tracing event with the entity count and exactly
/// one solve scale: `element_count` for list models or `candidate_count` for
/// scalar models.
///
/// # Panics
///
/// Panics unless exactly one of `element_count` and `candidate_count` is
/// `Some`.
pub fn log_solve_start(
    entity_count: usize,
    element_count: Option<usize>,
    candidate_count: Option<usize>,
) {
    match (element_count, candidate_count) {
        (Some(element_count), None) => {
            info!(
                event = "solve_start",
                entity_count = entity_count,
                element_count = element_count,
                solve_shape = "list",
            );
        }
        (None, Some(candidate_count)) => {
            info!(
                event = "solve_start",
                entity_count = entity_count,
                candidate_count = candidate_count,
                solve_shape = "scalar",
            );
        }
        _ => {
            panic!(
                "log_solve_start requires exactly one solve scale: list elements or scalar candidates"
            );
        }
    }
}

/// Runs one configured model through the immutable runtime graph compiler and
/// retained compiled runner.
///
/// `build_search` creates the one descriptor-resolved declaration consumed by
/// the compiled graph. Every model, including a zero-work model, follows this
/// one lifecycle; the public API never exposes a graph, prepared source
/// catalog, or phase-builder fallback.
///
/// `constraints` builds a constraint set for a given solution. It builds the
/// working solution's constraint set and every evaluator replica's, so parallel
/// local-search evaluation can fork replicas for any model.
///
/// Scores on the unweighted director: a solution type that carries runtime
/// constraint weights solves through
/// `try_run_weighted_solver_with_config_and_search`, and fails here when
/// some multiplier differs from `1`.
#[allow(clippy::too_many_arguments)]
pub fn try_run_solver_with_config_and_search<S, C, V, DM, IDM, Declaration, BuildSearch>(
    solution: S,
    constraints: fn(&S) -> C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    build_search: BuildSearch,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    IDM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    Declaration: Search<S, V, DM, IDM>,
    Declaration::Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>,
    BuildSearch: FnOnce(&SolverConfig, SolutionDescriptor) -> RuntimeBuildResult<Declaration>,
{
    try_run_solver_with_config_and_search_request::<S, C, UnweightedMode, V, DM, IDM, _, _>(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        log_scale,
        qualified_candidate_trace_provenance,
        build_search,
    )
}

#[allow(clippy::too_many_arguments)]
fn try_run_solver_with_config_and_search_request<S, C, M, V, DM, IDM, Declaration, BuildSearch>(
    solution: S,
    constraints: fn(&S) -> C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    build_search: BuildSearch,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    M: WeightMode,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    IDM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    Declaration: Search<S, V, DM, IDM>,
    Declaration::Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>,
    BuildSearch: FnOnce(&SolverConfig, SolutionDescriptor) -> RuntimeBuildResult<Declaration>,
{
    try_run_solver_with_candidate_trace_request::<S, C, M, _, _>(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        log_scale,
        qualified_candidate_trace_provenance,
        move |config, descriptor| {
            let declaration = build_search(config, descriptor.clone())?;
            let (context, extensions) = declaration.into_runtime_parts();
            compile_runtime_runner(config, context, extensions)
        },
    )
}

/// Compiles one search declaration into the retained phase runner a
/// configured solve executes. The selector benchmark builds its runners here
/// too, so it measures the cursors this exact compilation produces.
pub(crate) fn compile_runtime_runner<S, V, DM, IDM, Registry>(
    config: &SolverConfig,
    context: SearchContext<S, V, DM, IDM>,
    extensions: Registry,
) -> RuntimeBuildResult<CompiledRuntimePhaseRunner<S, V, DM, IDM, Registry::Phase>>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    IDM: crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>
        + Clone
        + Send
        + Sync
        + fmt::Debug
        + 'static,
    Registry: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    let graph = compile_runtime_graph(config, RuntimeGraphInput::new(context, extensions))
        .map_err(|error| {
            let message = error.to_string();
            RuntimeBuildError::Compilation {
                path: error.path,
                message,
            }
        })?;
    CompiledRuntimePhaseRunner::try_new(CompiledRuntimeExecutor::new(graph))
}

/// Shared configured solve. `W` is the mode a solution whose weights require
/// weighting scores in: `WeightedMode` for the weighted entrypoint, which then
/// starts each solve in the mode its weights need and switches modes when a
/// problem change needs the other; `UnweightedMode` for the plain entrypoint,
/// which instantiates only the unweighted solver and fails weights it cannot
/// apply.
#[allow(clippy::too_many_arguments)]
fn try_run_solver_with_candidate_trace_request<S, C, W, Runner, BuildRunner>(
    solution: S,
    constraints: fn(&S) -> C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    build_runner: BuildRunner,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    W: WeightMode,
    Runner: for<'d> Phase<
            S,
            ScoreDirector<S, C, ReplicaDescriptor<'d, S, C>, UnweightedMode>,
            ChannelProgressCallback<S>,
        > + for<'d> Phase<
            S,
            ScoreDirector<S, C, ReplicaDescriptor<'d, S, C>, W>,
            ChannelProgressCallback<S>,
        > + Send
        + std::fmt::Debug,
    BuildRunner: FnOnce(&SolverConfig, &SolutionDescriptor) -> RuntimeBuildResult<Runner>,
{
    // Every configured run entry point arrives here, so a retained job's
    // override applies to generated, weighted and bridge solves alike.
    let config = runtime.apply_config_override(config);
    log_scale(&solution);
    let main_constraints = constraints(&solution);
    if let Err(error) = validate_constraint_weights(&solution, &main_constraints) {
        runtime.emit_failed(error.to_string());
        return Err(error);
    }
    if !W::WEIGHTED && requires_weighted_mode(&solution) {
        let error = RuntimeBuildError::ConstraintWeights {
            message: "this solution type carries constraint weights; solve it through \
                      `try_run_weighted_solver_with_config_and_search`"
                .to_string(),
        };
        runtime.emit_failed(error.to_string());
        return Err(error);
    }

    let runner = match build_runner(&config, &descriptor) {
        Ok(runner) => runner,
        Err(error) => {
            runtime.emit_failed(error.to_string());
            return Err(error);
        }
    };

    let result = match catch_unwind(AssertUnwindSafe(|| {
        weight_modes::solve_in_weight_modes::<S, C, W, Runner>(
            weight_modes::ModeSolve {
                solution,
                constraints: main_constraints,
                descriptor: ReplicaDescriptor::new(&descriptor, constraints),
                entity_count_by_descriptor,
                runtime,
                config: &config,
                qualified_candidate_trace_provenance,
            },
            runner,
        )
    })) {
        Ok(result) => result,
        Err(payload) => match take_runtime_execution_failure(payload) {
            Ok(error) => {
                runtime.emit_failed(error.to_string());
                return Err(error);
            }
            Err(payload) => resume_unwind(payload),
        },
    };

    let crate::solver::SolveResult {
        solution,
        current_score,
        best_score: final_score,
        terminal_reason,
        stats,
    } = result;
    let final_telemetry = stats.snapshot();
    let final_move_speed = whole_units_per_second(stats.moves_evaluated(), stats.elapsed());
    match terminal_reason {
        SolverTerminalReason::Completed | SolverTerminalReason::TerminatedByConfig => {
            runtime.emit_completed(
                solution.clone(),
                current_score,
                final_score,
                final_telemetry,
                terminal_reason,
            );
        }
        SolverTerminalReason::Cancelled => {
            runtime.emit_cancelled(current_score, Some(final_score), final_telemetry);
        }
        SolverTerminalReason::Failed => {
            let error = RuntimeBuildError::Execution {
                phase_index: 0,
                message: "configured solver reported a failed terminal state".to_string(),
            };
            runtime.emit_failed(error.to_string());
            return Err(error);
        }
    }

    info!(
        event = "solve_end",
        score = %final_score,
        steps = stats.step_count(),
        moves_generated = stats.moves_generated(),
        moves_evaluated = stats.moves_evaluated(),
        moves_accepted = stats.moves_accepted(),
        score_calculations = stats.score_calculations(),
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        moves_speed = final_move_speed,
        acceptance_rate = format!("{:.1}%", stats.acceptance_rate() * 100.0),
    );
    Ok(solution)
}

pub(crate) mod weight_modes;

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
