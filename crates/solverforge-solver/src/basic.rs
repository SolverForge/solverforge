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

use crate::phase::basic::{BasicConstructionPhase, BasicLocalSearchPhase};
use crate::scope::SolverScope;
use crate::solver::Solver;
use crate::termination::{
    ExternalTermination, OrTermination, StepCountTermination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
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
pub fn run_solver_with_channel<S, C>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: fn() -> C,
    get_variable: fn(&S, usize) -> Option<usize>,
    set_variable: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count_fn: fn(&S) -> usize,
    terminate: Option<&AtomicBool>,
    sender: mpsc::UnboundedSender<(S, S::Score)>,
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
        entity_count = n_entities,
        variable_count = n_entities,
        value_count = n_values,
        "Solving started"
    );

    // Create score director
    let constraints = constraints_fn();
    let director = TypedScoreDirector::new(solution, constraints);

    // Handle empty case
    if n_entities == 0 || n_values == 0 {
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        info!(
            duration_ms = 0,
            total_moves = 0,
            phase_count = 0,
            final_score = %score,
            "Solving ended"
        );
        return solver_scope.take_best_or_working_solution();
    }

    // Build phases
    let construction = BasicConstructionPhase::new(
        get_variable,
        set_variable,
        entity_count_fn,
        value_count,
    );

    let local_search = BasicLocalSearchPhase::new(
        get_variable,
        set_variable,
        entity_count_fn,
        value_count,
        sender,
    );

    // Execute with appropriate termination based on config and external flag
    let result = match (terminate, config.termination.as_ref()) {
        (Some(flag), Some(term_config)) => {
            solve_with_external_and_config(
                director,
                construction,
                local_search,
                flag,
                term_config,
            )
        }
        (Some(flag), None) => {
            solve_with_external_only(director, construction, local_search, flag)
        }
        (None, Some(term_config)) => {
            solve_with_config_only(director, construction, local_search, term_config)
        }
        (None, None) => {
            solve_with_default_termination(director, construction, local_search)
        }
    };

    let score = result.score().unwrap_or_default();
    info!(final_score = %score, "Solving ended");
    result
}

fn solve_with_external_and_config<S, D, G, T, E, V>(
    director: D,
    construction: BasicConstructionPhase<S, G, T, E, V>,
    local_search: BasicLocalSearchPhase<S, G, T, E, V>,
    flag: &AtomicBool,
    term_config: &solverforge_config::TerminationConfig,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    let external = ExternalTermination::new(flag);
    let time = TimeTermination::new(
        term_config
            .time_limit()
            .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS)),
    );

    if let Some(step_limit) = term_config.step_count_limit {
        let step = StepCountTermination::new(step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((external, time, step));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else if let Some(unimproved_step_limit) = term_config.unimproved_step_count_limit {
        let unimproved = UnimprovedStepCountTermination::<S>::new(unimproved_step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((external, time, unimproved));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else if let Some(unimproved_time) = term_config.unimproved_time_limit() {
        let unimproved = UnimprovedTimeTermination::<S>::new(unimproved_time);
        let termination: OrTermination<_, S, D> = OrTermination::new((external, time, unimproved));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else {
        let termination: OrTermination<_, S, D> = OrTermination::new((external, time));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    }
}

fn solve_with_external_only<S, D, G, T, E, V>(
    director: D,
    construction: BasicConstructionPhase<S, G, T, E, V>,
    local_search: BasicLocalSearchPhase<S, G, T, E, V>,
    flag: &AtomicBool,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    let external = ExternalTermination::new(flag);
    let time = TimeTermination::seconds(DEFAULT_TIME_LIMIT_SECS);
    let termination: OrTermination<_, S, D> = OrTermination::new((external, time));

    Solver::new((construction, local_search))
        .with_termination(termination)
        .solve(director)
}

fn solve_with_config_only<S, D, G, T, E, V>(
    director: D,
    construction: BasicConstructionPhase<S, G, T, E, V>,
    local_search: BasicLocalSearchPhase<S, G, T, E, V>,
    term_config: &solverforge_config::TerminationConfig,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    let time = TimeTermination::new(
        term_config
            .time_limit()
            .unwrap_or(Duration::from_secs(DEFAULT_TIME_LIMIT_SECS)),
    );

    if let Some(step_limit) = term_config.step_count_limit {
        let step = StepCountTermination::new(step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, step));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else if let Some(unimproved_step_limit) = term_config.unimproved_step_count_limit {
        let unimproved = UnimprovedStepCountTermination::<S>::new(unimproved_step_limit);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else if let Some(unimproved_time) = term_config.unimproved_time_limit() {
        let unimproved = UnimprovedTimeTermination::<S>::new(unimproved_time);
        let termination: OrTermination<_, S, D> = OrTermination::new((time, unimproved));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    } else {
        let termination: OrTermination<_, S, D> = OrTermination::new((time,));
        Solver::new((construction, local_search))
            .with_termination(termination)
            .solve(director)
    }
}

fn solve_with_default_termination<S, D, G, T, E, V>(
    director: D,
    construction: BasicConstructionPhase<S, G, T, E, V>,
    local_search: BasicLocalSearchPhase<S, G, T, E, V>,
) -> S
where
    S: PlanningSolution,
    S::Score: Score,
    D: solverforge_scoring::ScoreDirector<S>,
    G: Fn(&S, usize) -> Option<usize> + Send,
    T: Fn(&mut S, usize, Option<usize>) + Send,
    E: Fn(&S) -> usize + Send,
    V: Fn(&S) -> usize + Send,
{
    let time = TimeTermination::seconds(DEFAULT_TIME_LIMIT_SECS);
    let termination: OrTermination<_, S, D> = OrTermination::new((time,));

    Solver::new((construction, local_search))
        .with_termination(termination)
        .solve(director)
}
