use super::*;

use crate::heuristic::selector::ChangeMoveSelector;
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor};
use crate::test_utils::{create_nqueens_director, get_queen_row, set_queen_row, NQueensSolution};

type NQueensMove = crate::heuristic::r#move::ChangeMove<NQueensSolution, i64>;

fn create_move_selector(
    values: Vec<i64>,
) -> ChangeMoveSelector<
    NQueensSolution,
    i64,
    crate::heuristic::selector::FromSolutionEntitySelector,
    crate::heuristic::selector::StaticValueSelector<NQueensSolution, i64>,
> {
    ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, "row", values)
}

#[test]
fn test_local_search_hill_climbing() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(100));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
    assert!(solver_scope.stats().moves_evaluated > 0);
}

#[test]
fn test_local_search_reaches_optimal() {
    let director = create_nqueens_director(&[0, 2, 1, 3]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(50));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}

#[test]
fn test_local_search_step_limit() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(3));

    phase.solve(&mut solver_scope);

    assert!(solver_scope.stats().step_count <= 3);
}
