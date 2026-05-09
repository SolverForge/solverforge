use super::*;

#[test]
fn test_solution_creation() {
    let s1 = TestSolution::new();
    assert!(s1.score.is_none());

    let s2 = TestSolution::with_score(SoftScore::of(-5));
    assert_eq!(s2.score, Some(SoftScore::of(-5)));
}

#[test]
fn test_queen_creation() {
    let q1 = Queen::new(0, 0, Some(1));
    assert_eq!(q1.id, 0);
    assert_eq!(q1.column, 0);
    assert_eq!(q1.row, Some(1));

    let q2 = Queen::assigned(1, 1, 2);
    assert_eq!(q2.row, Some(2));

    let q3 = Queen::unassigned(2, 2);
    assert_eq!(q3.row, None);
}

#[test]
fn test_nqueens_solution_creation() {
    let s1 = NQueensSolution::uninitialized(4);
    assert_eq!(s1.queens.len(), 4);
    assert!(s1.queens.iter().all(|q| q.row.is_none()));

    let s2 = NQueensSolution::with_rows(&[0, 2, 1, 3]);
    assert_eq!(s2.queens.len(), 4);
    assert_eq!(s2.queens[0].row, Some(0));
    assert_eq!(s2.queens[1].row, Some(2));
}

#[test]
fn test_conflict_calculation_no_conflicts() {
    let solution = NQueensSolution::with_rows(&[1, 3, 0, 2]);
    let score = calculate_conflicts(&solution);
    assert_eq!(score, SoftScore::of(0));
}

#[test]
fn test_conflict_calculation_row_conflict() {
    let solution = NQueensSolution::with_rows(&[0, 0, 2, 3]);
    let score = calculate_conflicts(&solution);
    assert!(score < SoftScore::of(0));
}

#[test]
fn test_conflict_calculation_diagonal_conflict() {
    let solution = NQueensSolution::with_rows(&[0, 1, 3, 2]);
    let score = calculate_conflicts(&solution);
    assert!(score < SoftScore::of(0));
}

#[test]
fn test_create_scope() {
    let scope = create_scope();
    assert_eq!(scope.total_step_count(), 0);
}

#[test]
fn test_create_scope_with_score() {
    let scope = create_scope_with_score(SoftScore::of(-10));
    assert!(scope.best_solution().is_some());
    assert_eq!(scope.best_score(), Some(&SoftScore::of(-10)));
}
