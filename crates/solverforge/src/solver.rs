//! Solver entry point that hides all internal wiring.

use solverforge_config::SolverConfig;
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::Score;
use solverforge_scoring::director::SolvableSolution;
use solverforge_scoring::{ConstraintSet, TypedScoreDirector};
use solverforge_solver::manager::{LocalSearchType, SolverManager};
use solverforge_solver::{BasicConstructionPhaseBuilder, BasicLocalSearchPhaseBuilder};

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

    let construction = BasicConstructionPhaseBuilder::<S, usize>::new(
        getter,
        setter,
        value_count,
        entity_count,
        variable_name,
        descriptor_index,
    );

    let local_search = BasicLocalSearchPhaseBuilder::<S>::new(
        getter,
        setter,
        value_count,
        variable_name,
        descriptor_index,
        LocalSearchType::LateAcceptance { size: 400 },
    );

    let manager = SolverManager::<S>::builder()
        .with_phase_factory(construction)
        .with_phase_factory(local_search)
        .with_config(config)
        .build()
        .expect("Failed to build solver");

    let constraints = constraints_fn();
    let director = TypedScoreDirector::with_descriptor(
        solution,
        constraints,
        descriptor(),
        entity_count_by_idx,
    );

    let mut solver = manager.create_solver();
    solver.solve_with_director(Box::new(director))
}
