//! Basic variable solver for simple assignment problems.
//!
//! This module provides `run_solver` for problems using `#[basic_variable_config]`,
//! where each entity has a single planning variable that can be assigned from a
//! fixed value range.
//!
//! Logging levels:
//! - **INFO**: Solver start/end, phase summaries, problem scale
//! - **DEBUG**: Individual steps with timing and scores
//! - **TRACE**: Move evaluation details

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, ScoreDirector, TypedScoreDirector};
use tokio::sync::mpsc;
use tracing::info;

use crate::heuristic::selector::decorator::UnionMoveSelector;
use crate::heuristic::selector::{
    EitherChangeMoveSelector, EitherSwapMoveSelector, FromSolutionEntitySelector,
    StaticTypedValueSelector,
};
use crate::phase::construction::{BestFitForager, ConstructionHeuristicPhase, QueuedEntityPlacer};
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};
use crate::scope::BestSolutionCallback;
use crate::scope::SolverScope;
use crate::solver::Solver;
use crate::termination::{
    BestScoreTermination, OrTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Default time limit in seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

/// Monomorphized termination enum for basic solver configurations.
///
/// Avoids repeated branching across `solve_with_termination` overloads by
/// capturing the selected termination variant upfront.
pub(crate) enum AnyBasicTermination<S: PlanningSolution, D: ScoreDirector<S>> {
    Default(OrTermination<(TimeTermination,), S, D>),
    WithBestScore(OrTermination<(TimeTermination, BestScoreTermination<S::Score>), S, D>),
    WithStepCount(OrTermination<(TimeTermination, StepCountTermination), S, D>),
    WithUnimprovedStep(OrTermination<(TimeTermination, UnimprovedStepCountTermination<S>), S, D>),
    WithUnimprovedTime(OrTermination<(TimeTermination, UnimprovedTimeTermination<S>), S, D>),
}

impl<S: PlanningSolution, D: ScoreDirector<S>> fmt::Debug for AnyBasicTermination<S, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default(_) => write!(f, "AnyBasicTermination::Default"),
            Self::WithBestScore(_) => write!(f, "AnyBasicTermination::WithBestScore"),
            Self::WithStepCount(_) => write!(f, "AnyBasicTermination::WithStepCount"),
            Self::WithUnimprovedStep(_) => write!(f, "AnyBasicTermination::WithUnimprovedStep"),
            Self::WithUnimprovedTime(_) => write!(f, "AnyBasicTermination::WithUnimprovedTime"),
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>, BestCb: BestSolutionCallback<S>>
    Termination<S, D, BestCb> for AnyBasicTermination<S, D>
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

/// Solves a basic variable problem using construction heuristic + late acceptance local search.
///
/// This function is called by macro-generated `solve()` methods for solutions
/// using `#[basic_variable_config]`.
///
/// # Type Parameters
///
/// * `S` - The solution type (must implement `PlanningSolution`)
/// * `C` - The constraint set type
///
/// # Arguments
///
/// * `solution` - The initial solution to solve
/// * `finalize_fn` - Function to prepare derived fields before solving
/// * `constraints_fn` - Function that creates the constraint set
/// * `get_variable` - Gets the planning variable value for an entity
/// * `set_variable` - Sets the planning variable value for an entity
/// * `value_count` - Returns the number of valid values
/// * `entity_count_fn` - Returns the number of entities
/// * `descriptor` - Solution descriptor for solver infrastructure
/// * `entity_count_by_descriptor` - Returns entity count for a given descriptor index
/// * `terminate` - Optional external termination flag
/// * `sender` - Channel for streaming best solutions as they are found
/// * `_variable_field` - Variable field name (unused, for future extensions)
/// * `_descriptor_index` - Descriptor index (unused, for future extensions)
#[allow(clippy::too_many_arguments)]
pub fn run_solver<S, C>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_descriptor: fn(&S, usize) -> usize,
    terminate: Option<&AtomicBool>,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
    _variable_field: &'static str,
    _descriptor_index: usize,
) -> S
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    finalize_fn(&mut solution);

    let config = SolverConfig::load("solver.toml").unwrap_or_default();
    let n_entities = entity_count_fn(&solution);
    let n_values = value_count(&solution);

    info!(
        event = "solve_start",
        entity_count = n_entities,
        value_count = n_values,
    );

    // Create score director with real entity counter for selector iteration
    let constraints = constraints_fn();
    let director = TypedScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor(),
        entity_count_by_descriptor,
    );

    // Handle empty case - nothing to solve, return immediately
    if n_entities == 0 || n_values == 0 {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(event = "solve_end", score = %score);
        let solution = solver_scope.take_best_or_working_solution();
        let _ = sender.send((solution.clone(), score));
        return solution;
    }

    // Build construction phase with BestFitForager
    let values: Vec<usize> = (0..n_values).collect();
    let entity_selector = FromSolutionEntitySelector::new(0);
    let value_selector = StaticTypedValueSelector::new(values);
    let placer = QueuedEntityPlacer::new(
        entity_selector,
        value_selector,
        get_variable,
        set_variable,
        0,
        "variable",
    );
    let construction = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

    // Build local search phase with Late Acceptance
    // Unified move selector: ChangeMove + SwapMove via EitherMove
    let values: Vec<usize> = (0..n_values).collect();
    let change_selector =
        EitherChangeMoveSelector::simple(get_variable, set_variable, 0, "variable", values);
    let swap_selector = EitherSwapMoveSelector::simple(get_variable, set_variable, 0, "variable");
    let move_selector = UnionMoveSelector::new(change_selector, swap_selector);
    let acceptor = SimulatedAnnealingAcceptor::default();
    let forager = AcceptedCountForager::new(1);
    let local_search = LocalSearchPhase::new(move_selector, acceptor, forager, None);

    // Build termination from config
    let term_config = config.termination.as_ref();
    let time_limit = term_config
        .and_then(|c| c.time_limit())
        .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS));
    let time = TimeTermination::new(time_limit);

    let best_score_target: Option<S::Score> = term_config
        .and_then(|c| c.best_score_limit.as_ref())
        .and_then(|s| S::Score::parse(s).ok());

    let termination: AnyBasicTermination<S, TypedScoreDirector<S, C>> =
        if let Some(target) = best_score_target {
            AnyBasicTermination::WithBestScore(OrTermination::new((
                time,
                BestScoreTermination::new(target),
            )))
        } else if let Some(step_limit) = term_config.and_then(|c| c.step_count_limit) {
            AnyBasicTermination::WithStepCount(OrTermination::new((
                time,
                StepCountTermination::new(step_limit),
            )))
        } else if let Some(unimproved_step_limit) =
            term_config.and_then(|c| c.unimproved_step_count_limit)
        {
            AnyBasicTermination::WithUnimprovedStep(OrTermination::new((
                time,
                UnimprovedStepCountTermination::<S>::new(unimproved_step_limit),
            )))
        } else if let Some(unimproved_time) = term_config.and_then(|c| c.unimproved_time_limit()) {
            AnyBasicTermination::WithUnimprovedTime(OrTermination::new((
                time,
                UnimprovedTimeTermination::<S>::new(unimproved_time),
            )))
        } else {
            AnyBasicTermination::Default(OrTermination::new((time,)))
        };

    let callback_sender = sender.clone();
    let callback = move |solution: &S| {
        let score = solution.score().unwrap_or_default();
        let _ = callback_sender.send((solution.clone(), score));
    };

    let solver = Solver::new(((), construction, local_search))
        .with_termination(termination)
        .with_time_limit(time_limit)
        .with_best_solution_callback(callback);

    let result = if let Some(flag) = terminate {
        solver.with_terminate(flag).solve(director)
    } else {
        solver.solve(director)
    };

    // Send the final solution so the receiver always gets the last state
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
