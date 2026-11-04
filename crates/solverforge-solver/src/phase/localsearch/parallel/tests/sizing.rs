use super::super::EvaluatorPool;
use super::support::{forkable_director, starting_solution, QueenMove};
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_descriptor, NQueensSolution};

#[test]
fn evaluation_policy_resolution() {
    use std::num::NonZeroUsize;

    use solverforge_config::MoveThreadCount;

    use super::super::config::AUTO_MAX_EVALUATORS;
    use super::super::{resolve_evaluation_policy, EvaluationPolicy};

    let count = |n: usize| MoveThreadCount::Count(NonZeroUsize::new(n).unwrap());
    let in_pool = |threads: usize, check: &(dyn Fn() + Sync)| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(check);
    };

    in_pool(8, &|| {
        assert_eq!(
            resolve_evaluation_policy(&MoveThreadCount::None),
            EvaluationPolicy::Sequential
        );
        assert_eq!(
            resolve_evaluation_policy(&MoveThreadCount::Auto),
            EvaluationPolicy::Auto(AUTO_MAX_EVALUATORS)
        );
        assert_eq!(
            resolve_evaluation_policy(&count(1)),
            EvaluationPolicy::Sequential
        );
        assert_eq!(
            resolve_evaluation_policy(&count(6)),
            EvaluationPolicy::Fixed(6)
        );
        assert_eq!(
            resolve_evaluation_policy(&count(64)),
            EvaluationPolicy::Fixed(8)
        );
    });
    in_pool(2, &|| {
        assert_eq!(
            resolve_evaluation_policy(&MoveThreadCount::Auto),
            EvaluationPolicy::Auto(2)
        );
    });
    in_pool(1, &|| {
        assert_eq!(
            resolve_evaluation_policy(&MoveThreadCount::Auto),
            EvaluationPolicy::Sequential
        );
        assert_eq!(
            resolve_evaluation_policy(&count(4)),
            EvaluationPolicy::Sequential
        );
    });
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
