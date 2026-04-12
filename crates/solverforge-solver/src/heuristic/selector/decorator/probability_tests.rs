use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;

fn uniform_weight(_: &ChangeMove<TaskSolution, i32>) -> f64 {
    1.0
}

fn zero_weight(_: &ChangeMove<TaskSolution, i32>) -> f64 {
    0.0
}

#[test]
fn selects_some_moves_with_uniform_weight() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let mut total_selected = 0;
    for seed in 0..10 {
        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let prob = ProbabilityMoveSelector::with_seed(inner, uniform_weight, seed);
        total_selected += prob.iter_moves(&director).count();
    }
    assert!(total_selected > 0);
}

#[test]
fn zero_weight_selects_nothing() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
    let prob = ProbabilityMoveSelector::with_seed(inner, zero_weight, 42);

    let moves: Vec<_> = prob.iter_moves(&director).collect();
    assert!(moves.is_empty());
}

#[test]
fn same_seed_produces_same_selection() {
    let director = create_director(vec![Task { priority: Some(1) }]);

    let inner1 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let prob1 = ProbabilityMoveSelector::with_seed(inner1, uniform_weight, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let prob2 = ProbabilityMoveSelector::with_seed(inner2, uniform_weight, 42);

    let moves1: Vec<_> = prob1
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();
    let moves2: Vec<_> = prob2
        .iter_moves(&director)
        .filter_map(|m| m.to_value().copied())
        .collect();

    assert_eq!(moves1, moves2);
}
