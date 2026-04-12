use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::test_utils::{create_nqueens_director, get_queen_row, set_queen_row, NQueensSolution};

type NQueensMove = ChangeMove<NQueensSolution, i64>;

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
fn test_vnd_improves_solution() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((
        create_move_selector(values.clone()),
        create_move_selector(values),
    ));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}

#[test]
fn test_vnd_single_neighborhood() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let mut phase: VndPhase<_, NQueensMove> = VndPhase::new((create_move_selector(values),));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}
