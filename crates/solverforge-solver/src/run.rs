//! Unified solver entry point.
//!
//! This module provides the single `run_solver` function used by both basic
//! variable and list variable problems via the `ProblemSpec` trait.

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector};
use tokio::sync::mpsc;
use tracing::info;

use crate::problem_spec::ProblemSpec;
use crate::scope::{BestSolutionCallback, SolverScope};
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

impl<S: PlanningSolution, D: Director<S>, BestCb: BestSolutionCallback<S>> Termination<S, D, BestCb>
    for AnyTermination<S, D>
where
    S::Score: Score,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D, BestCb>) -> bool {
        match self {
            Self::Default(t) => t.is_terminated(solver_scope),
            Self::WithBestScore(t) => t.is_terminated(solver_scope),
            Self::WithStepCount(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedStep(t) => t.is_terminated(solver_scope),
            Self::WithUnimprovedTime(t) => t.is_terminated(solver_scope),
        }
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, BestCb>) {
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
    S: PlanningSolution,
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

/// Solves a problem using the given `ProblemSpec` for problem-specific logic.
///
/// This is the unified entry point for both basic variable and list variable
/// problems. The shared logic (config loading, director creation, trivial-case
/// handling, termination building, callback setup, final send) lives here.
/// Problem-specific construction and local search are delegated to `spec`.
#[allow(clippy::too_many_arguments)]
pub fn run_solver<S, C, Spec>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    terminate: Option<&AtomicBool>,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
    spec: Spec,
) -> S
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
    Spec: ProblemSpec<S, C>,
{
    finalize_fn(&mut solution);

    let config = SolverConfig::load("solver.toml").unwrap_or_default();

    spec.log_scale(&solution);
    let trivial = spec.is_trivial(&solution);

    let constraints = constraints_fn();
    let director = ScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor(),
        entity_count_by_descriptor,
    );

    if trivial {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(event = "solve_end", score = %score);
        let solution = solver_scope.take_best_or_working_solution();
        let _ = sender.send((solution.clone(), score));
        return solution;
    }

    let (termination, time_limit) =
        build_termination::<S, C>(&config, spec.default_time_limit_secs());

    let callback_sender = sender.clone();
    let callback = move |solution: &S| {
        let score = solution.score().unwrap_or_default();
        let _ = callback_sender.send((solution.clone(), score));
    };

    let result = spec.build_and_solve(
        director,
        &config,
        time_limit,
        termination,
        terminate,
        callback,
    );

    let final_score = result.solution.score().unwrap_or_default();
    let _ = sender.send((result.solution.clone(), final_score));

    info!(
        event = "solve_end",
        score = %final_score,
        steps = result.stats.step_count,
        moves_evaluated = result.stats.moves_evaluated,
    );
    result.solution
}
