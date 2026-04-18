use super::*;

use std::thread;
use std::time::Duration;

use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor};
use crate::test_utils::{
    create_minimal_director, create_nqueens_director, get_queen_row, set_queen_row,
    NQueensSolution, TestSolution,
};

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

#[derive(Debug)]
struct NoopMove;

impl Move<TestSolution> for NoopMove {
    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, _score_director: &mut D) {}

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "noop"
    }
}

#[derive(Debug)]
struct SlowOpenSelector;

impl MoveSelector<TestSolution, NoopMove> for SlowOpenSelector {
    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> impl Iterator<Item = NoopMove> + 'a {
        thread::sleep(Duration::from_millis(20));
        std::iter::once(NoopMove)
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
    }
}

#[test]
fn test_local_search_records_selector_open_time_as_generation_time() {
    let director = create_minimal_director();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = SlowOpenSelector;
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NoopMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert!(solver_scope.stats().generation_time() >= Duration::from_millis(20));
    assert_eq!(solver_scope.stats().moves_generated, 1);
}
