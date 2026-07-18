/* Solver entry point. */

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::time::Duration;

#[cfg(test)]
use std::path::Path;

use solverforge_config::{SolverConfig, TerminationConfig};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector};
use tracing::info;

use crate::builder::{RuntimeExtensionRegistry, Search};
use crate::manager::{SolverRuntime, SolverTerminalReason};
use crate::phase::Phase;
use crate::runtime::compiler::executor::{
    take_runtime_execution_failure, CompiledRuntimePhaseRunner,
};
use crate::runtime::compiler::{compile_runtime_graph, CompiledRuntimeExecutor, RuntimeGraphInput};
use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
use crate::scope::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
use crate::solver::{NoTermination, Solver};
use crate::stats::{
    format_duration, whole_units_per_second, CandidateTraceExecutionPolicy,
    QualifiedCandidateTraceRunProvenance,
};
use crate::termination::{
    BestScoreTermination, OrTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Monomorphized termination enum for config-driven solver configurations.
///
/// Avoids repeated branching across termination overloads by capturing the
/// selected termination variant upfront.
pub enum AnyTermination<S: PlanningSolution, D: Director<S>> {
    None(NoTermination),
    Default(OrTermination<(TimeTermination,), S, D>),
    WithBestScore(OrTermination<(TimeTermination, BestScoreTermination<S::Score>), S, D>),
    WithStepCount(OrTermination<(TimeTermination, StepCountTermination), S, D>),
    WithUnimprovedStep(OrTermination<(TimeTermination, UnimprovedStepCountTermination<S>), S, D>),
    WithUnimprovedTime(OrTermination<(TimeTermination, UnimprovedTimeTermination<S>), S, D>),
}

#[derive(Clone)]
pub struct ChannelProgressCallback<S: PlanningSolution> {
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

impl<S: PlanningSolution, D: Director<S>> fmt::Debug for AnyTermination<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None(_) => write!(f, "AnyTermination::None"),
            Self::Default(_) => write!(f, "AnyTermination::Default"),
            Self::WithBestScore(_) => write!(f, "AnyTermination::WithBestScore"),
            Self::WithStepCount(_) => write!(f, "AnyTermination::WithStepCount"),
            Self::WithUnimprovedStep(_) => write!(f, "AnyTermination::WithUnimprovedStep"),
            Self::WithUnimprovedTime(_) => write!(f, "AnyTermination::WithUnimprovedTime"),
        }
    }
}

impl<S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    Termination<S, D, ProgressCb> for AnyTermination<S, D>
where
    S::Score: Score,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D, ProgressCb>) -> bool {
        match self {
            Self::None(t) => t.is_terminated(solver_scope),
            Self::Default(t) => t.is_terminated(solver_scope),
            Self::WithBestScore(t) => t.is_terminated(solver_scope),
            Self::WithStepCount(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedStep(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedTime(t) => t.is_terminated(solver_scope),
        }
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        match self {
            Self::None(t) => t.install_inphase_limits(solver_scope),
            Self::Default(t) => t.install_inphase_limits(solver_scope),
            Self::WithBestScore(t) => t.install_inphase_limits(solver_scope),
            Self::WithStepCount(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedStep(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedTime(t) => t.install_inphase_limits(solver_scope),
        }
    }
}

/// Parsed solver termination policy shared by runtime phase assembly and the
/// top-level termination builder.
///
/// `TerminationConfig` historically chooses the first configured score/work
/// criterion in this order: best score, step count, unimproved steps,
/// unimproved time. A configured time limit is paired with that criterion, or
/// is the policy itself when no other criterion is present. Keeping that
/// precedence here prevents phase assembly from treating an empty or
/// unparsable configuration as a finite solver boundary.
#[derive(Clone, Copy)]
pub(crate) struct ConfiguredTermination<Sc> {
    time_limit: Option<Duration>,
    criterion: Option<ConfiguredTerminationCriterion<Sc>>,
}

#[derive(Clone, Copy)]
enum ConfiguredTerminationCriterion<Sc> {
    BestScore(Sc),
    StepCount(u64),
    UnimprovedStepCount(u64),
    UnimprovedTime(Duration),
}

impl<Sc> ConfiguredTermination<Sc> {
    pub(crate) fn has_effective_limit(&self) -> bool {
        self.time_limit.is_some() || self.criterion.is_some()
    }
}

pub(crate) fn parse_configured_termination<S>(
    config: Option<&TerminationConfig>,
) -> ConfiguredTermination<S::Score>
where
    S: PlanningSolution,
    S::Score: ParseableScore,
{
    let time_limit = config.and_then(TerminationConfig::time_limit);
    let criterion = config.and_then(|config| {
        config
            .best_score_limit
            .as_deref()
            .and_then(|score| S::Score::parse(score).ok())
            .map(ConfiguredTerminationCriterion::BestScore)
            .or_else(|| {
                config
                    .step_count_limit
                    .map(ConfiguredTerminationCriterion::StepCount)
            })
            .or_else(|| {
                config
                    .unimproved_step_count_limit
                    .map(ConfiguredTerminationCriterion::UnimprovedStepCount)
            })
            .or_else(|| {
                config
                    .unimproved_time_limit()
                    .map(ConfiguredTerminationCriterion::UnimprovedTime)
            })
    });
    ConfiguredTermination {
        time_limit,
        criterion,
    }
}

/// Builds a termination from config, returning both the termination and the time limit.
pub fn build_termination<S, C>(
    config: &SolverConfig,
    default_secs: u64,
) -> (AnyTermination<S, ScoreDirector<S, C>>, Option<Duration>)
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    let ConfiguredTermination {
        time_limit: configured_time_limit,
        criterion,
    } = parse_configured_termination::<S>(config.termination.as_ref());
    let fallback_time_limit = Duration::from_secs(default_secs);

    let (termination, effective_time_limit) = match criterion {
        Some(ConfiguredTerminationCriterion::BestScore(target)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithBestScore(OrTermination::new((
                    time,
                    BestScoreTermination::new(target),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::StepCount(step_limit)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithStepCount(OrTermination::new((
                    time,
                    StepCountTermination::new(step_limit),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::UnimprovedStepCount(unimproved_step_limit)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithUnimprovedStep(OrTermination::new((
                    time,
                    UnimprovedStepCountTermination::<S>::new(unimproved_step_limit),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::UnimprovedTime(unimproved_time)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithUnimprovedTime(OrTermination::new((
                    time,
                    UnimprovedTimeTermination::<S>::new(unimproved_time),
                ))),
                Some(effective_time_limit),
            )
        }
        None => configured_time_limit.map_or_else(
            || (AnyTermination::None(NoTermination), None),
            |limit| {
                let time = TimeTermination::new(limit);
                (
                    AnyTermination::Default(OrTermination::new((time,))),
                    Some(limit),
                )
            },
        ),
    };

    (termination, effective_time_limit)
}

/// Records the termination policy the configured runtime actually installed.
///
/// This deliberately derives its time guard from `build_termination`'s
/// returned effective limit rather than from the input TOML.  In particular,
/// a score/work criterion without an explicit time limit gets the configured
/// entrypoint's fallback guard, and that injected guard is material to both
/// bounded-work and fixed-budget comparisons.
pub(crate) fn configured_execution_policy<S>(
    config: &SolverConfig,
    default_secs: u64,
    effective_time_limit: Option<Duration>,
) -> CandidateTraceExecutionPolicy
where
    S: PlanningSolution,
    S::Score: ParseableScore + std::fmt::Display,
{
    let configured = parse_configured_termination::<S>(config.termination.as_ref());
    let configured_time_limit = configured.time_limit;
    let criterion = configured.criterion;
    let fallback_time_limit = Duration::from_secs(default_secs);

    let time_limit_source = match (configured_time_limit, effective_time_limit) {
        (Some(_), Some(_)) => "configured",
        (None, Some(_)) if criterion.is_some() => "configured_entrypoint_fallback",
        (None, Some(_)) => "internal",
        (_, None) => "not_installed",
    };
    let mut attributes = vec![
        ("entrypoint".to_string(), "configured_runtime".to_string()),
        (
            "configured_time_limit_ns".to_string(),
            configured_time_limit.map_or_else(|| "none".to_string(), duration_nanos),
        ),
        (
            "configured_entrypoint_default_time_limit_ns".to_string(),
            duration_nanos(fallback_time_limit),
        ),
        (
            "effective_time_limit_ns".to_string(),
            effective_time_limit.map_or_else(|| "none".to_string(), duration_nanos),
        ),
        (
            "time_limit_source".to_string(),
            time_limit_source.to_string(),
        ),
    ];

    match criterion {
        Some(ConfiguredTerminationCriterion::BestScore(target)) => {
            attributes.push(("criterion".to_string(), "best_score".to_string()));
            attributes.push(("criterion_target".to_string(), target.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_best_score".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::StepCount(limit)) => {
            attributes.push(("criterion".to_string(), "step_count".to_string()));
            attributes.push(("criterion_target".to_string(), limit.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_step_count".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::UnimprovedStepCount(limit)) => {
            attributes.push(("criterion".to_string(), "unimproved_step_count".to_string()));
            attributes.push(("criterion_target".to_string(), limit.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_unimproved_step_count".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::UnimprovedTime(limit)) => {
            attributes.push(("criterion".to_string(), "unimproved_time".to_string()));
            attributes.push(("criterion_target_ns".to_string(), duration_nanos(limit)));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_unimproved_time".to_string(),
            ));
        }
        None if effective_time_limit.is_some() => {
            attributes.push(("criterion".to_string(), "none".to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_only".to_string(),
            ));
        }
        None => {
            attributes.push(("criterion".to_string(), "none".to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "unbounded".to_string(),
            ));
        }
    }

    CandidateTraceExecutionPolicy::known("solverforge.execution_policy", attributes)
}

fn duration_nanos(duration: Duration) -> String {
    duration.as_nanos().to_string()
}

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

#[cfg(test)]
fn load_solver_config_from(path: impl AsRef<Path>) -> SolverConfig {
    SolverConfig::load(path).unwrap_or_default()
}

/// Runs one configured model through the immutable runtime graph compiler and
/// retained compiled runner.
///
/// `build_search` creates the one descriptor-resolved declaration consumed by
/// the compiled graph. Every model, including a zero-work model, follows this
/// one lifecycle; the public API never exposes a graph, prepared source
/// catalog, or phase-builder fallback.
#[allow(clippy::too_many_arguments)]
pub fn try_run_solver_with_config_and_search<S, C, V, DM, IDM, Declaration, BuildSearch>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
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
    try_run_solver_with_config_and_search_request(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        default_time_limit_secs,
        log_scale,
        qualified_candidate_trace_provenance,
        build_search,
    )
}

#[allow(clippy::too_many_arguments)]
fn try_run_solver_with_config_and_search_request<S, C, V, DM, IDM, Declaration, BuildSearch>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
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
    try_run_solver_with_candidate_trace_request(
        solution,
        constraints,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        default_time_limit_secs,
        log_scale,
        qualified_candidate_trace_provenance,
        move |config, descriptor| {
            let declaration = build_search(config, descriptor.clone())?;
            let (context, extensions) = declaration.into_runtime_parts();
            let graph = compile_runtime_graph(config, RuntimeGraphInput::new(context, extensions))
                .map_err(|error| {
                    let message = error.to_string();
                    RuntimeBuildError::Compilation {
                        path: error.path,
                        message,
                    }
                })?;
            let executor = CompiledRuntimeExecutor::new(graph);
            CompiledRuntimePhaseRunner::try_new(&executor)
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn try_run_solver_with_candidate_trace_request<S, C, Runner, BuildRunner>(
    solution: S,
    constraints: C,
    descriptor: SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    log_scale: fn(&S),
    qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    build_runner: BuildRunner,
) -> RuntimeBuildResult<S>
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    Runner: Phase<S, ScoreDirector<S, C>, ChannelProgressCallback<S>> + Send + std::fmt::Debug,
    BuildRunner: FnOnce(&SolverConfig, &SolutionDescriptor) -> RuntimeBuildResult<Runner>,
{
    log_scale(&solution);
    let director = ScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor.clone(),
        entity_count_by_descriptor,
    );

    let (termination, time_limit) = build_termination::<S, C>(&config, default_time_limit_secs);
    let execution_policy =
        configured_execution_policy::<S>(&config, default_time_limit_secs, time_limit);

    let callback = ChannelProgressCallback::new(runtime);

    let runner = match build_runner(&config, &descriptor) {
        Ok(runner) => runner,
        Err(error) => {
            runtime.emit_failed(error.to_string());
            return Err(error);
        }
    };
    let mut solver = Solver::new((runner,))
        .with_config(config.clone())
        .with_candidate_trace_execution_policy(execution_policy)
        .with_termination(termination)
        .with_runtime(runtime)
        .with_progress_callback(callback);
    if let Some(provenance) = qualified_candidate_trace_provenance {
        solver = solver.with_qualified_candidate_trace_run_provenance(provenance);
    }
    if let Some(time_limit) = time_limit {
        solver = solver.with_time_limit(time_limit);
    }

    let result = match catch_unwind(AssertUnwindSafe(|| {
        solver.with_terminate(runtime.cancel_flag()).solve(director)
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
    let final_move_speed = whole_units_per_second(stats.moves_evaluated, stats.elapsed());
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
        steps = stats.step_count,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        moves_speed = final_move_speed,
        acceptance_rate = format!("{:.1}%", stats.acceptance_rate() * 100.0),
    );
    Ok(solution)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
