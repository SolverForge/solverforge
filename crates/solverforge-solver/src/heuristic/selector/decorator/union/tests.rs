use super::super::test_utils::{create_director, get_priority, set_priority, Task};
use super::*;
use crate::heuristic::selector::ChangeMoveSelector;

#[test]
fn combines_both_selectors() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let first = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);
    let second = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![100, 200, 300],
    );
    let union = UnionMoveSelector::new(first, second);

    let values: Vec<_> = union
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    assert_eq!(values, vec![10, 20, 100, 200, 300]);
    assert_eq!(union.size(&director), 5);
}

#[test]
fn handles_empty_first() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let first = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![]);
    let second =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![100, 200]);
    let union = UnionMoveSelector::new(first, second);

    let values: Vec<_> = union
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    assert_eq!(values, vec![100, 200]);
}

#[test]
fn handles_empty_second() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let first = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);
    let second = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![]);
    let union = UnionMoveSelector::new(first, second);

    let values: Vec<_> = union
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    assert_eq!(values, vec![10, 20]);
}

#[test]
fn both_empty_yields_nothing() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let first = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![]);
    let second = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![]);
    let union = UnionMoveSelector::new(first, second);

    let moves: Vec<_> = union.iter_moves(&director).collect();
    assert!(moves.is_empty());
    assert_eq!(union.size(&director), 0);
}
