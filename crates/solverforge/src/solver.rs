//! Solver entry point that hides all internal wiring.

use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::Score;
use solverforge_scoring::director::SolvableSolution;
use solverforge_scoring::{ConstraintSet, TypedScoreDirector};
use solverforge_solver::{
    BasicConstructionPhaseBuilder, BasicLocalSearchPhaseBuilder,
    PhaseSequence, SolverBuilder,
};

/// Runs the solver on a solution with basic (non-list) planning variables.
pub fn run_solver<S, C, F>(
    mut solution: S,
    finalize_fn: fn(&mut S),
    constraints_fn: F,
    getter: fn(&S, usize) -> Option<usize>,
    setter: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count: fn(&S) -> usize,
    descriptor: fn() -> SolutionDescriptor,
    entity_count_by_idx: fn(&S, usize) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
) -> S
where
    S: SolvableSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + 'static,
    F: Fn() -> C,
{
    finalize_fn(&mut solution);

    let config = SolverConfig::load("solver.toml").unwrap_or_default();

    // Create constraints once to determine the type
    let constraints = constraints_fn();

    // Get late acceptance size from config or use default
    let late_acceptance_size = 400; // Default late acceptance size

    // Create the construction phase
    let construction_builder = BasicConstructionPhaseBuilder::<S, usize>::new(
        getter,
        setter,
        value_count,
        entity_count,
        variable_name,
        descriptor_index,
    );
    let construction_phase = construction_builder.create_phase();

    // Create the local search phase
    let local_search_builder = BasicLocalSearchPhaseBuilder::<S>::new(
        getter,
        setter,
        value_count,
        variable_name,
        descriptor_index,
        late_acceptance_size,
    );
    let local_search_phase = local_search_builder.create_phase();

    // Compose the two phases
    let combined_phase = PhaseSequence((construction_phase, local_search_phase));

    // Build the solver with time limit from config
    let time_limit = config.termination
        .as_ref()
        .and_then(|t| t.seconds_spent_limit)
        .map(|s| Duration::from_secs(s));

    let director = TypedScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor(),
        entity_count_by_idx,
    );

    let mut builder = SolverBuilder::new(combined_phase);
    if let Some(limit) = time_limit {
        builder = builder.with_time_limit(limit);
    }

    let solver = builder.build_with_time();
    solver.solve(director)
}
