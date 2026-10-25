use super::super::EvaluatorPool;
use super::support::{forkable_director, starting_solution, QueenMove};
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_descriptor, NQueensSolution};

#[test]
fn evaluator_count_resolution() {
    use solverforge_config::MoveThreadCount;

    use super::super::resolve_evaluator_count;

    assert_eq!(resolve_evaluator_count(&MoveThreadCount::None), 1);
    assert_eq!(resolve_evaluator_count(&MoveThreadCount::Auto), 1);
    assert_eq!(resolve_evaluator_count(&MoveThreadCount::Count(0)), 1);
    assert_eq!(resolve_evaluator_count(&MoveThreadCount::Count(1)), 1);
    assert_eq!(
        resolve_evaluator_count(&MoveThreadCount::Count(2)),
        2.min(rayon::current_num_threads())
    );
}

#[test]
fn windows_aim_at_the_previous_steps_consumption() {
    let descriptor = create_nqueens_descriptor();
    let mut solver_scope = SolverScope::new(forkable_director!(starting_solution(), &descriptor));
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(4);
    pool.sync(&solver_scope);

    // No history: one candidate per evaluator.
    pool.begin_step();
    assert_eq!(pool.window_capacity(None), 4);
    pool.record_window(4);
    assert_eq!(pool.window_capacity(None), 4);
    pool.finish_step(23);

    // Cover the previous consumption, then continue one per evaluator.
    pool.begin_step();
    assert_eq!(pool.window_capacity(None), 23);
    assert_eq!(pool.window_capacity(Some(5)), 5);
    pool.record_window(23);
    assert_eq!(pool.window_capacity(None), 4);
    pool.finish_step(70);

    // Long steps run in capped windows until the target, then per evaluator.
    pool.begin_step();
    assert_eq!(pool.window_capacity(None), 32);
    pool.record_window(32);
    assert_eq!(pool.window_capacity(None), 32);
    pool.record_window(32);
    assert_eq!(pool.window_capacity(None), 6);
    pool.record_window(6);
    assert_eq!(pool.window_capacity(None), 4);
}

#[test]
fn windows_above_the_ceiling_stay_within_one_full_window() {
    let descriptor = create_nqueens_descriptor();
    let mut solver_scope = SolverScope::new(forkable_director!(starting_solution(), &descriptor));
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(64);
    pool.sync(&solver_scope);

    // No history: one candidate per evaluator, which is one full window.
    pool.begin_step();
    assert_eq!(pool.window_capacity(None), 32);
    pool.record_window(32);
    pool.finish_step(2);

    // Short history still dispatches one candidate per evaluator.
    pool.begin_step();
    assert_eq!(pool.window_capacity(None), 32);
}
