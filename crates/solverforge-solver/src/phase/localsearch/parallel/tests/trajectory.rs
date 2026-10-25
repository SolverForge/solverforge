use super::support::{forkable_director, run, selector, starting_solution};
use crate::phase::localsearch::{
    solve_local_search_with_resources, AcceptedCountForager, BestScoreForager, HillClimbingAcceptor,
};
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_descriptor, NQueensSolution};

#[test]
fn sequential_and_parallel_reach_identical_trajectories() {
    let sequential = run(1, AcceptedCountForager::new(4, false), 25, None);
    for evaluator_count in [2, 4] {
        let parallel = run(
            evaluator_count,
            AcceptedCountForager::new(4, false),
            25,
            None,
        );
        assert_eq!(parallel.rows, sequential.rows);
        assert_eq!(parallel.score, sequential.score);
        assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
        assert_eq!(parallel.moves_applied, sequential.moves_applied);
        assert_eq!(parallel.steps, sequential.steps);
    }
    assert!(sequential.moves_applied > 0);
}

#[test]
fn speculative_evaluations_are_reported_and_not_counted() {
    let sequential = run(1, AcceptedCountForager::new(1, false), 10, None);
    let parallel = run(2, AcceptedCountForager::new(1, false), 10, None);

    assert_eq!(parallel.rows, sequential.rows);
    assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
    assert_eq!(sequential.moves_speculative, 0);
    assert!(parallel.moves_speculative > 0);
}

#[test]
fn in_phase_move_limit_reserves_the_window() {
    let parallel = run(2, BestScoreForager::new(false), 10, Some(5));

    assert_eq!(parallel.moves_evaluated, 5);
    assert_eq!(parallel.moves_speculative, 0);
}

#[test]
fn replicas_follow_solution_replacement() {
    let descriptor = create_nqueens_descriptor();
    let run_with_replacement = |evaluator_count: usize| {
        let mut solver_scope =
            SolverScope::new(forkable_director!(starting_solution(), &descriptor)).with_seed(11);
        solver_scope.start_solving();
        let mut source = selector();
        let mut acceptor = HillClimbingAcceptor::new();
        let mut forager = AcceptedCountForager::new(3, false);
        solve_local_search_with_resources(
            &mut source,
            &mut (),
            &mut acceptor,
            &mut forager,
            Some(5),
            evaluator_count,
            &mut solver_scope,
        );
        solver_scope.replace_working_solution_and_reinitialize(NQueensSolution::with_rows(&[
            9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        ]));
        solve_local_search_with_resources(
            &mut source,
            &mut (),
            &mut acceptor,
            &mut forager,
            Some(5),
            evaluator_count,
            &mut solver_scope,
        );
        solver_scope
            .working_solution()
            .queens
            .iter()
            .map(|queen| queen.row)
            .collect::<Vec<_>>()
    };

    assert_eq!(run_with_replacement(2), run_with_replacement(1));
}

#[test]
fn evaluator_counts_above_the_ceiling_keep_the_sequential_trajectory() {
    let sequential = run(1, AcceptedCountForager::new(4, false), 25, None);
    let parallel = run(64, AcceptedCountForager::new(4, false), 25, None);

    assert_eq!(parallel.rows, sequential.rows);
    assert_eq!(parallel.score, sequential.score);
    assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
    assert_eq!(parallel.moves_applied, sequential.moves_applied);
}

#[test]
fn exhausted_move_budget_pulls_like_the_sequential_loop() {
    // A full-scan forager never discards speculative work, so generation
    // counts compare exactly, including the pull that meets the spent budget.
    let sequential = run(1, BestScoreForager::new(false), 10, Some(5));
    let parallel = run(2, BestScoreForager::new(false), 10, Some(5));

    assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
    assert_eq!(parallel.moves_generated, sequential.moves_generated);
    assert_eq!(parallel.moves_speculative, 0);
}
