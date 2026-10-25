use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, ScoreDirector};

use super::super::EvaluatorPool;
use super::support::{forkable_director, starting_solution, QueenMove};
use crate::scope::SolverScope;
use crate::test_utils::{
    create_nqueens_descriptor, nqueens_clone_count, reset_nqueens_clone_count, NQueensSolution,
};

#[test]
fn pool_with_one_evaluator_never_forks() {
    let director =
        ScoreDirector::simple(starting_solution(), create_nqueens_descriptor(), |s, _| {
            s.queens.len()
        });
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(1);

    pool.sync(&solver_scope);

    assert!(!pool.is_parallel());
}

#[test]
#[should_panic(expected = "constraint-set constructor")]
fn pool_requires_a_forkable_director() {
    let director =
        ScoreDirector::simple(starting_solution(), create_nqueens_descriptor(), |s, _| {
            s.queens.len()
        });
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(2);

    pool.sync(&solver_scope);
}

#[test]
fn pool_forks_one_replica_per_extra_evaluator_and_resyncs_after_writes() {
    let descriptor = create_nqueens_descriptor();
    let mut solver_scope = SolverScope::new(forkable_director!(starting_solution(), &descriptor));
    solver_scope.start_solving();
    solver_scope.calculate_score();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(4);

    reset_nqueens_clone_count();
    pool.sync(&solver_scope);
    assert_eq!(nqueens_clone_count(), 3);
    assert_eq!(pool.replicas().count(), 3);

    pool.sync(&solver_scope);
    assert_eq!(
        nqueens_clone_count(),
        3,
        "an unchanged generation must not fork again"
    );

    solver_scope.mutate(|director| {
        director.before_variable_changed(0, 0);
        director.working_solution_mut().queens[0].row = Some(7);
        director.after_variable_changed(0, 0);
    });
    let main_score = solver_scope.calculate_score();
    pool.sync(&solver_scope);

    assert_eq!(nqueens_clone_count(), 6);
    for replica in pool.replicas() {
        assert_eq!(replica.working_solution().queens[0].row, Some(7));
        assert_eq!(replica.working_solution().score(), Some(main_score));
        assert!(
            std::ptr::eq(Director::solution_descriptor(replica), &descriptor),
            "replicas must share the solve's descriptor, not copy it"
        );
    }
}

#[test]
fn pool_caps_evaluators_at_the_window_ceiling() {
    use super::super::pool::MAX_PARALLEL_EVALUATION_WINDOW;

    let descriptor = create_nqueens_descriptor();
    let mut solver_scope = SolverScope::new(forkable_director!(starting_solution(), &descriptor));
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(64);

    reset_nqueens_clone_count();
    pool.sync(&solver_scope);

    // A full window gives every evaluator a slot; a 33rd would never run.
    assert_eq!(pool.replicas().count(), MAX_PARALLEL_EVALUATION_WINDOW - 1);
    assert_eq!(nqueens_clone_count(), MAX_PARALLEL_EVALUATION_WINDOW - 1);
}
