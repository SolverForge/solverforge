/* Solver entry point. */

use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector, ShadowVariableSupport};
use tracing::info;

use crate::manager::{SolverRuntime, SolverTerminalReason};
use crate::phase::{Phase, PhaseSequence};
use crate::scope::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
use crate::solver::Solver;
use crate::stats::{format_duration, whole_units_per_second};
use crate::termination::{
    BestScoreTermination, OrTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Monomorphized termination enum for config-driven solver configurations.
///
/// Avoids repeated branching across termination overloads by capturing the
/// selected termination variant upfront.
pub enum AnyTermination<S: PlanningSolution, D: Director<S>> {
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
                    progress.telemetry,
                );
            }
            SolverProgressKind::BestSolution => {
                if let (Some(solution), Some(score)) = (progress.solution, progress.best_score) {
                    self.runtime.emit_best_solution(
                        (*solution).clone(),
                        progress.current_score.copied(),
                        *score,
                        progress.telemetry,
                    );
                }
            }
        }
    }
}

impl<S: PlanningSolution, D: Director<S>> fmt::Debug for AnyTermination<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::Default(t) => t.is_terminated(solver_scope),
            Self::WithBestScore(t) => t.is_terminated(solver_scope),
            Self::WithStepCount(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedStep(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedTime(t) => t.is_terminated(solver_scope),
        }
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        match self {
            Self::Default(t) => t.install_inphase_limits(solver_scope),
            Self::WithBestScore(t) => t.install_inphase_limits(solver_scope),
            Self::WithStepCount(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedStep(t) => t.install_inphase_limits(solver_scope),
            Self::WithUnimprovedTime(t) => t.install_inphase_limits(solver_scope),
        }
    }
}

/// Builds a termination from config, returning both the termination and the time limit.
pub fn build_termination<S, C>(
    config: &SolverConfig,
    default_secs: u64,
) -> (AnyTermination<S, ScoreDirector<S, C>>, Duration)
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    let term_config = config.termination.as_ref();
    let time_limit = term_config
        .and_then(|c| c.time_limit())
        .unwrap_or(Duration::from_secs(default_secs));
    let time = TimeTermination::new(time_limit);

    let best_score_target: Option<S::Score> = term_config
        .and_then(|c| c.best_score_limit.as_ref())
        .and_then(|s| S::Score::parse(s).ok());

    let termination = if let Some(target) = best_score_target {
        AnyTermination::WithBestScore(OrTermination::new((
            time,
            BestScoreTermination::new(target),
        )))
    } else if let Some(step_limit) = term_config.and_then(|c| c.step_count_limit) {
        AnyTermination::WithStepCount(OrTermination::new((
            time,
            StepCountTermination::new(step_limit),
        )))
    } else if let Some(unimproved_step_limit) =
        term_config.and_then(|c| c.unimproved_step_count_limit)
    {
        AnyTermination::WithUnimprovedStep(OrTermination::new((
            time,
            UnimprovedStepCountTermination::<S>::new(unimproved_step_limit),
        )))
    } else if let Some(unimproved_time) = term_config.and_then(|c| c.unimproved_time_limit()) {
        AnyTermination::WithUnimprovedTime(OrTermination::new((
            time,
            UnimprovedTimeTermination::<S>::new(unimproved_time),
        )))
    } else {
        AnyTermination::Default(OrTermination::new((time,)))
    };

    (termination, time_limit)
}

pub fn log_solve_start(
    entity_count: usize,
    element_count: Option<usize>,
    has_standard: Option<bool>,
    variable_count: Option<usize>,
) {
    info!(
        event = "solve_start",
        entity_count = entity_count,
        value_count = element_count.or(variable_count).unwrap_or(0),
        solve_shape = if element_count.is_some() {
            "list"
        } else if has_standard.unwrap_or(false) {
            "standard"
        } else {
            "solution"
        },
    );
}

fn load_solver_config_from(path: impl AsRef<Path>) -> SolverConfig {
    SolverConfig::load(path).unwrap_or_default()
}

fn load_solver_config() -> SolverConfig {
    load_solver_config_from("solver.toml")
}

#[allow(clippy::too_many_arguments)]
pub fn run_solver<S, C, P>(
    solution: S,
    constraints_fn: fn() -> C,
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    default_time_limit_secs: u64,
    is_trivial: fn(&S) -> bool,
    log_scale: fn(&S),
    build_phases: fn(&SolverConfig) -> PhaseSequence<P>,
) -> S
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    P: Send + std::fmt::Debug,
    PhaseSequence<P>: Phase<S, ScoreDirector<S, C>, ChannelProgressCallback<S>>,
{
    let config = load_solver_config();
    run_solver_with_config(
        solution,
        constraints_fn,
        descriptor,
        entity_count_by_descriptor,
        runtime,
        config,
        default_time_limit_secs,
        is_trivial,
        log_scale,
        build_phases,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_solver_with_config<S, C, P>(
    solution: S,
    constraints_fn: fn() -> C,
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    runtime: SolverRuntime<S>,
    config: SolverConfig,
    default_time_limit_secs: u64,
    is_trivial: fn(&S) -> bool,
    log_scale: fn(&S),
    build_phases: fn(&SolverConfig) -> PhaseSequence<P>,
) -> S
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    P: Send + std::fmt::Debug,
    PhaseSequence<P>: Phase<S, ScoreDirector<S, C>, ChannelProgressCallback<S>>,
{
    log_scale(&solution);
    let trivial = is_trivial(&solution);

    let constraints = constraints_fn();
    let director = ScoreDirector::with_descriptor_and_shadow_support(
        solution,
        constraints,
        descriptor(),
        entity_count_by_descriptor,
    );

    if trivial {
        let mut solver_scope = SolverScope::new(director);
        solver_scope = solver_scope.with_runtime(Some(runtime));
        if let Some(seed) = config.random_seed {
            solver_scope = solver_scope.with_seed(seed);
        }
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution.clone(), score);
        solver_scope.report_best_solution();
        solver_scope.pause_if_requested();
        info!(event = "solve_end", score = %score);
        let telemetry = solver_scope.stats().snapshot();
        if runtime.is_cancel_requested() {
            runtime.emit_cancelled(Some(score), Some(score), telemetry);
        } else {
            runtime.emit_completed(
                solution.clone(),
                Some(score),
                score,
                telemetry,
                SolverTerminalReason::Completed,
            );
        }
        return solution;
    }

    let (termination, time_limit) = build_termination::<S, C>(&config, default_time_limit_secs);

    let callback = ChannelProgressCallback::new(runtime);

    let phases = build_phases(&config);
    let solver = Solver::new((phases,))
        .with_config(config.clone())
        .with_termination(termination)
        .with_time_limit(time_limit)
        .with_runtime(runtime)
        .with_progress_callback(callback);

    let result = solver.with_terminate(runtime.cancel_flag()).solve(director);

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
        SolverTerminalReason::Failed => unreachable!("solver completion cannot report failure"),
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
    solution
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
