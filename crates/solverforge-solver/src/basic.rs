//! Basic variable solver for simple assignment problems.
//!
//! This module provides `run_solver` for problems using `#[basic_variable_config]`,
//! where each entity has a single planning variable that can be assigned from a
//! fixed value range.

use rand::Rng;
use solverforge_config::SolverConfig;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::{ConstraintSet, TypedScoreDirector};

/// Late acceptance history size.
const LATE_ACCEPTANCE_SIZE: usize = 400;

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
    mut solution: S,
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
    S::Score: Score + Default + Copy,
    C: ConstraintSet<S, S::Score>,
{
    // Finalize derived fields
    finalize_fn(&mut solution);

    // Load config
    let config = SolverConfig::load("solver.toml").unwrap_or_default();
    let time_limit = config
        .termination
        .as_ref()
        .and_then(|t| t.seconds_spent_limit)
        .unwrap_or(DEFAULT_TIME_LIMIT_SECS);

    // Create constraints and score director
    let constraints = constraints_fn();

    let n_entities = entity_count_fn(&solution);
    let n_values = value_count(&solution);

    if n_entities == 0 || n_values == 0 {
        return solution;
    }

    // Phase 1: Construction heuristic (greedy first-fit)
    let mut rng = rand::rng();
    for entity_idx in 0..n_entities {
        if get_variable(&solution, entity_idx).is_none() {
            let value = rng.random_range(0..n_values);
            set_variable(&mut solution, entity_idx, Some(value));
        }
    }

    // Create score director with working solution
    let mut director = TypedScoreDirector::new(solution, constraints);
    let mut current_score = director.calculate_score();

    // Phase 2: Late Acceptance local search
    let mut late_scores = vec![current_score; LATE_ACCEPTANCE_SIZE];
    let start = std::time::Instant::now();
    let time_limit_duration = std::time::Duration::from_secs(time_limit);
    let mut step: u64 = 0;

    while start.elapsed() < time_limit_duration {
        // Pick random entity and new value
        let entity_idx = rng.random_range(0..n_entities);
        let old_value = get_variable(director.working_solution(), entity_idx);
        let new_value = Some(rng.random_range(0..n_values));

        if old_value == new_value {
            continue;
        }

        // Apply move using incremental protocol
        director.before_variable_changed(entity_idx);
        set_variable(director.working_solution_mut(), entity_idx, new_value);
        director.after_variable_changed(entity_idx);
        let new_score = director.calculate_score();

        // Late acceptance criterion
        let late_idx = (step as usize) % LATE_ACCEPTANCE_SIZE;
        let late_score = late_scores[late_idx];

        if new_score >= current_score || new_score >= late_score {
            // Accept
            late_scores[late_idx] = new_score;
            current_score = new_score;
        } else {
            // Reject - undo using incremental protocol
            director.before_variable_changed(entity_idx);
            set_variable(director.working_solution_mut(), entity_idx, old_value);
            director.after_variable_changed(entity_idx);
            director.calculate_score(); // Update cached_score after undo
        }

        step += 1;
    }

    director.into_working_solution()
}
