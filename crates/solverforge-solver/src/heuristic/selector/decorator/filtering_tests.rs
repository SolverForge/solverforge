use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;

fn high_value_filter(m: &ChangeMove<TaskSolution, i32>) -> bool {
    m.to_value().is_some_and(|v| *v > 50)
}

#[test]
fn filters_moves_by_predicate() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 60, 80, 30],
    );
    let filtered = FilteringMoveSelector::new(inner, high_value_filter);

    let moves: Vec<_> = filtered.iter_moves(&director).collect();
    assert_eq!(moves.len(), 2);
    assert_eq!(moves[0].to_value(), Some(&60));
    assert_eq!(moves[1].to_value(), Some(&80));
}

#[test]
fn empty_when_no_moves_pass() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
    let filtered = FilteringMoveSelector::new(inner, high_value_filter);

    let moves: Vec<_> = filtered.iter_moves(&director).collect();
    assert!(moves.is_empty());
}
