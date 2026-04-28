use super::*;

#[test]
fn test_solution_creation() {
    let s1 = TestSolution::new();
    assert!(s1.score.is_none());

    let s2 = TestSolution::with_score(SoftScore::of(-5));
    assert_eq!(s2.score, Some(SoftScore::of(-5)));
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
