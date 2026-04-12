use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;

fn by_value_asc(a: &ChangeMove<TaskSolution, i32>, b: &ChangeMove<TaskSolution, i32>) -> Ordering {
    a.to_value().cmp(&b.to_value())
}

fn by_value_desc(a: &ChangeMove<TaskSolution, i32>, b: &ChangeMove<TaskSolution, i32>) -> Ordering {
    b.to_value().cmp(&a.to_value())
}

#[test]
fn sorts_ascending() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![30, 10, 50, 20, 40],
    );
    let sorted = SortingMoveSelector::new(inner, by_value_asc);

    let values: Vec<_> = sorted
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    assert_eq!(values, vec![10, 20, 30, 40, 50]);
}

#[test]
fn sorts_descending() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![30, 10, 50, 20, 40],
    );
    let sorted = SortingMoveSelector::new(inner, by_value_desc);

    let values: Vec<_> = sorted
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    assert_eq!(values, vec![50, 40, 30, 20, 10]);
}

#[test]
fn preserves_size() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![30, 10, 50]);
    let sorted = SortingMoveSelector::new(inner, by_value_asc);

    assert_eq!(sorted.size(&director), 3);
}
