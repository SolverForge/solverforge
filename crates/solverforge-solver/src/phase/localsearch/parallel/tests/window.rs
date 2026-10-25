use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_descriptor, thread_allocations, NQueensSolution};

use super::super::{evaluate_window, EvaluatorPool};
use super::support::{forkable_director, starting_solution, QueenMove};

fn queen_move(entity: usize, row: i64) -> QueenMove {
    QueenMove::new(
        entity,
        Some(row),
        crate::test_utils::get_queen_row,
        crate::test_utils::set_queen_row,
        0,
        "row",
        0,
    )
}

#[test]
fn window_dispatch_makes_no_heap_allocations() {
    // One pool thread runs the coordinator and every replica task, so the
    // thread-local counter sees every allocation the window makes.
    let pool_thread = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("test pool must build");
    pool_thread.install(|| {
        let descriptor = create_nqueens_descriptor();
        let mut solver_scope =
            SolverScope::new(forkable_director!(starting_solution(), &descriptor));
        solver_scope.start_solving();
        solver_scope.calculate_score();
        let mut evaluators = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(4);
        evaluators.sync(&solver_scope);

        let moves: Vec<QueenMove> = (0..8).map(|entity| queen_move(entity, 7)).collect();
        let candidates: Vec<_> = moves.iter().map(MoveCandidateRef::Borrowed).collect();
        let mut journal_pool = Vec::new();
        let mut results = Vec::new();

        // The first window sizes the reused buffers.
        evaluate_window(
            &candidates,
            solver_scope.score_director_mut(),
            &mut journal_pool,
            &mut evaluators,
            &mut results,
        );
        let before = thread_allocations();
        evaluate_window(
            &candidates,
            solver_scope.score_director_mut(),
            &mut journal_pool,
            &mut evaluators,
            &mut results,
        );

        assert_eq!(thread_allocations() - before, 0);
        assert!(results.iter().all(Option::is_some));
    });
}
