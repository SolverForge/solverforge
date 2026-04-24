use super::super::test_utils::{create_director, get_priority, set_priority, Task};
use super::*;
use crate::heuristic::r#move::{ScalarMoveUnion, SequentialCompositeMove};
use crate::heuristic::selector::decorator::{test_utils::TaskSolution, CartesianProductSelector};
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::{ChangeMoveSelector, ScalarChangeMoveSelector};

fn wrap_scalar_composite(
    mov: SequentialCompositeMove<TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> ScalarMoveUnion<TaskSolution, i32> {
    ScalarMoveUnion::Composite(mov)
}

fn composite_pair(
    candidate: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> Option<(i32, i32)> {
    let MoveCandidateRef::Sequential(sequence) = candidate else {
        return None;
    };
    let ScalarMoveUnion::Change(first) = sequence.first() else {
        return None;
    };
    let ScalarMoveUnion::Change(second) = sequence.second() else {
        return None;
    };
    Some((*first.to_value()?, *second.to_value()?))
}

#[test]
fn preserves_all_moves() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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
        0,
        "priority",
        vec![10, 20, 30, 40, 50],
    );
    let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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
        0,
        "priority",
        vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    );
    let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

    let inner2 = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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

#[test]
fn shuffles_cartesian_candidates_without_dropping_borrowable_access() {
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
    let shuffled = ShufflingMoveSelector::with_seed(cartesian, 17);

    let mut cursor = shuffled.open_cursor(&director);
    let indices =
        collect_cursor_indices::<TaskSolution, ScalarMoveUnion<TaskSolution, i32>, _>(&mut cursor);

    assert!(indices.len() >= 2);
    let pairs: Vec<_> = indices
        .iter()
        .map(|&index| {
            cursor
                .candidate(index)
                .and_then(composite_pair)
                .expect("shuffled cartesian candidate must remain valid")
        })
        .collect();
    assert_eq!(pairs.len(), 4);
    assert!(pairs.contains(&(10, 30)));
    assert!(pairs.contains(&(10, 40)));
    assert!(pairs.contains(&(20, 30)));
    assert!(pairs.contains(&(20, 40)));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}
