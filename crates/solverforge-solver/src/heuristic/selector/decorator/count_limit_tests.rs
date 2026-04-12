use super::super::test_utils::{create_director, get_priority, set_priority, Task};
use super::*;
use crate::heuristic::selector::ChangeMoveSelector;

#[test]
fn limits_move_count() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let limited = SelectedCountLimitMoveSelector::new(inner, 3);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert_eq!(moves.len(), 3);
    assert_eq!(limited.size(&director), 3);
}

#[test]
fn returns_all_when_under_limit() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);
    let limited = SelectedCountLimitMoveSelector::new(inner, 10);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert_eq!(moves.len(), 2);
    assert_eq!(limited.size(&director), 2);
}

#[test]
fn zero_limit_yields_nothing() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
    let limited = SelectedCountLimitMoveSelector::new(inner, 0);

    let moves: Vec<_> = limited.iter_moves(&director).collect();
    assert!(moves.is_empty());
    assert_eq!(limited.size(&director), 0);
}
