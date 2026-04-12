use super::super::test_utils::{create_director, get_priority, set_priority, Task};
use super::*;
use crate::heuristic::selector::ChangeMoveSelector;

#[test]
fn preserves_all_moves() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let shuffled = ShufflingMoveSelector::with_seed(inner, 42);

    let moves: Vec<_> = shuffled.iter_moves(&director).collect();
    assert_eq!(moves.len(), 5);
    assert_eq!(shuffled.size(&director), 5);

    let values: Vec<_> = moves.iter().filter_map(|m| m.to_value().copied()).collect();
    assert!(values.contains(&10));
    assert!(values.contains(&20));
    assert!(values.contains(&30));
    assert!(values.contains(&40));
    assert!(values.contains(&50));
}

#[test]
fn same_seed_produces_same_order() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let inner1 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let shuffled2 = ShufflingMoveSelector::with_seed(inner2, 42);

    let moves1: Vec<_> = shuffled1
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    let moves2: Vec<_> = shuffled2
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();

    assert_eq!(moves1, moves2);
}

#[test]
fn different_seeds_produce_different_order() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let inner1 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    );
    let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    );
    let shuffled2 = ShufflingMoveSelector::with_seed(inner2, 123);

    let moves1: Vec<_> = shuffled1
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    let moves2: Vec<_> = shuffled2
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();

    assert_ne!(moves1, moves2);
}
