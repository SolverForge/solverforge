use super::*;

#[test]
fn test_queen_creation() {
    let q1 = Queen::assigned(1, 1, 2);
    assert_eq!(q1.id, 1);
    assert_eq!(q1.column, 1);
    assert_eq!(q1.row, Some(2));

    let q2 = Queen::unassigned(2, 2);
    assert_eq!(q2.row, None);
}

#[test]
fn test_nqueens_solution_creation() {
    let solution = NQueensSolution::new(vec![Queen::assigned(0, 0, 0)]);
    assert_eq!(solution.queens.len(), 1);
    assert!(solution.score.is_none());
}

#[test]
fn test_get_set_queen_row() {
    let mut solution = NQueensSolution::new(vec![Queen::unassigned(0, 0)]);
    assert_eq!(get_queen_row(&solution, 0, 0), None);

    set_queen_row(&mut solution, 0, 0, Some(5));
    assert_eq!(get_queen_row(&solution, 0, 0), Some(5));
}

#[test]
fn test_shadow_solution_creation() {
    let s1 = ShadowSolution::new(vec![1, 2, 3]);
    assert_eq!(s1.values, vec![1, 2, 3]);
    assert_eq!(s1.cached_sum, 0);
    assert!(s1.score.is_none());

    let s2 = ShadowSolution::with_cached_sum(vec![1, 2, 3], 6);
    assert_eq!(s2.cached_sum, 6);
}

#[test]
fn test_shadow_default() {
    let s = ShadowSolution::default();
    assert!(s.values.is_empty());
    assert_eq!(s.cached_sum, 0);
}
