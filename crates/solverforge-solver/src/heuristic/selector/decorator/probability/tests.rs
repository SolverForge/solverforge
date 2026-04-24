use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::{ChangeMove, ScalarMoveUnion, SequentialCompositeMove};
use crate::heuristic::selector::decorator::CartesianProductSelector;
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::{ChangeMoveSelector, ScalarChangeMoveSelector};

fn uniform_weight(_: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>) -> f64 {
    1.0
}

fn zero_weight(_: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>) -> f64 {
    0.0
}

fn wrap_scalar_composite(
    mov: SequentialCompositeMove<TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> ScalarMoveUnion<TaskSolution, i32> {
    ScalarMoveUnion::Composite(mov)
}

fn biased_cartesian_weight(
    candidate: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> f64 {
    let MoveCandidateRef::Sequential(sequence) = candidate else {
        return 0.0;
    };
    let ScalarMoveUnion::Change(first) = sequence.first() else {
        return 0.0;
    };
    let ScalarMoveUnion::Change(second) = sequence.second() else {
        return 0.0;
    };
    match (first.to_value().copied(), second.to_value().copied()) {
        (Some(10), Some(30)) | (Some(20), Some(40)) => 2.0,
        _ => 1.0,
    }
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
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        0,
        "priority",
        vec![10, 20, 30],
    );
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
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let prob1 = ProbabilityMoveSelector::with_seed(inner1, uniform_weight, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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

#[test]
fn probabilistic_filter_keeps_cartesian_candidates_borrowable() {
    let director = create_director(vec![Task { priority: Some(0) }]);
    let left = ScalarChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        0,
        "priority",
        vec![10, 20],
    );
    let right = ScalarChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        0,
        "priority",
        vec![30, 40],
    );
    let cartesian = CartesianProductSelector::new(left, right, wrap_scalar_composite);
    let probabilistic = ProbabilityMoveSelector::with_seed(cartesian, biased_cartesian_weight, 9);

    let mut cursor = probabilistic.open_cursor(&director);
    let indices =
        collect_cursor_indices::<TaskSolution, ScalarMoveUnion<TaskSolution, i32>, _>(&mut cursor);

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|candidate| candidate.is_doable(&director))));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}
