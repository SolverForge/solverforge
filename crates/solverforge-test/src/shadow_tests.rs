use super::*;

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
fn test_default() {
    let s = ShadowSolution::default();
    assert!(s.values.is_empty());
    assert_eq!(s.cached_sum, 0);
}
