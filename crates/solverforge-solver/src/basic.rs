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

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::{ConstraintSet, TypedScoreDirector};
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
use crate::scope::SolverScope;
use crate::solver::{SolveResult, Solver};
use crate::termination::{
    OrTermination, StepCountTermination, TimeTermination, UnimprovedStepCountTermination,
    UnimprovedTimeTermination,
};

/// Default time limit in seconds.
const DEFAULT_TIME_LIMIT_SECS: u64 = 30;

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
/// * `_descriptor` - Solution descriptor (unused, for future extensions)
/// * `_entity_count` - Entity count function (unused, for future extensions)
/// * `_variable_field` - Variable field name (unused, for future extensions)
/// * `_descriptor_index` - Descriptor index (unused, for future extensions)
#[allow(clippy::too_many_arguments)]
pub fn run_solver<S, C>(
    solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    _descriptor: fn() -> SolutionDescriptor,
    _entity_count: fn(&S, usize) -> usize,
    _variable_field: &'static str,
    _descriptor_index: usize,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    // Create a channel but ignore the receiver - no streaming needed
    let (sender, _receiver) = mpsc::unbounded_channel();
    run_solver_with_channel(
        solution,
        finalize_fn,
        constraints_fn,
        get_variable,
        set_variable,
        value_count,
        entity_count_fn,
        None,
        sender,
    )
}

/// Solves a basic variable problem with channel-based solution streaming.
///
/// Logs solver progress via `tracing`. Optionally accepts a termination flag.
/// Solutions are sent through the channel as they improve.
#[allow(clippy::too_many_arguments)]
pub fn run_solver_with_channel<S, C>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    terminate: Option<&AtomicBool>,
    _sender: mpsc::UnboundedSender<(S, S::Score)>,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
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

    // Create score director
    let constraints = constraints_fn();
    let director = TypedScoreDirector::new(solution, constraints);

    // Handle empty case - nothing to solve, return immediately
    if n_entities == 0 || n_values == 0 {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(event = "solve_end", score = %score);
        return solver_scope.take_best_or_working_solution();
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
    let forager = AcceptedCountForager::new(4);
    let local_search = LocalSearchPhase::new(move_selector, acceptor, forager, None);

    // Build solver with termination configuration
    let result = solve_with_termination(
        director,
        construction,
        local_search,
        terminate,
        config.termination.as_ref(),
    );

    let score = result.solution.score().unwrap_or_default();
    info!(
        event = "solve_end",
        score = %score,
        steps = result.stats.step_count,
        moves_evaluated = result.stats.moves_evaluated,
    );
    result.solution
}

fn solve_with_termination<S, D, M1, M2, P, Fo, MS, A, Fo2>(
    director: D,
    construction: ConstructionHeuristicPhase<S, M1, P, Fo>,
    local_search: LocalSearchPhase<S, M2, MS, A, Fo2>,
    terminate: Option<&AtomicBool>,
    term_config: Option<&solverforge_config::TerminationConfig>,
) -> SolveResult<S>
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    M1: crate::heuristic::r#move::Move<S>,
    M2: crate::heuristic::r#move::Move<S>,
    P: crate::phase::construction::EntityPlacer<S, M1>,
    Fo: crate::phase::construction::ConstructionForager<S, M1>,
    MS: crate::heuristic::selector::MoveSelector<S, M2>,
    A: crate::phase::localsearch::Acceptor<S>,
    Fo2: crate::phase::localsearch::LocalSearchForager<S, M2>,
{
    let time_limit = term_config
        .and_then(|c| c.time_limit())
        .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS));
    let time = TimeTermination::new(time_limit);

    // Build termination based on config
    if let Some(step_limit) = term_config.and_then(|c| c.step_count_limit) {
        let step = StepCountTermination::new(step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, step));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else if let Some(unimproved_step_limit) =
        term_config.and_then(|c| c.unimproved_step_count_limit)
    {
        let unimproved = UnimprovedStepCountTermination::<S>::new(unimproved_step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else if let Some(unimproved_time) = term_config.and_then(|c| c.unimproved_time_limit()) {
        let unimproved = UnimprovedTimeTermination::<S>::new(unimproved_time);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    } else {
        let termination: OrTermination<_, S, D> = OrTermination::new((time,));
        build_and_solve(
            construction,
            local_search,
            termination,
            terminate,
            director,
            time_limit,
        )
    }
}

fn build_and_solve<S, D, M1, M2, P, Fo, MS, A, Fo2, Term>(
    construction: ConstructionHeuristicPhase<S, M1, P, Fo>,
    local_search: LocalSearchPhase<S, M2, MS, A, Fo2>,
    termination: Term,
    terminate: Option<&AtomicBool>,
    director: D,
    time_limit: Duration,
) -> SolveResult<S>
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    M1: crate::heuristic::r#move::Move<S>,
    M2: crate::heuristic::r#move::Move<S>,
    P: crate::phase::construction::EntityPlacer<S, M1>,
    Fo: crate::phase::construction::ConstructionForager<S, M1>,
    MS: crate::heuristic::selector::MoveSelector<S, M2>,
    A: crate::phase::localsearch::Acceptor<S>,
    Fo2: crate::phase::localsearch::LocalSearchForager<S, M2>,
    Term: crate::termination::Termination<S, D>,
{
    match terminate {
        Some(flag) => Solver::new(((), construction, local_search))
            .with_termination(termination)
            .with_time_limit(time_limit)
            .with_terminate(flag)
            .solve(director),
        None => Solver::new(((), construction, local_search))
            .with_termination(termination)
            .with_time_limit(time_limit)
            .solve(director),
    }
}
