use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::{ScalarMoveUnion, SequentialCompositeMove};
use crate::heuristic::selector::decorator::CartesianProductSelector;
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::{ChangeMoveSelector, MoveSelector, ScalarChangeMoveSelector};

fn high_value_filter(
    candidate: MoveCandidateRef<
        '_,
        TaskSolution,
        crate::heuristic::r#move::ChangeMove<TaskSolution, i32>,
    >,
) -> bool {
    matches!(candidate, MoveCandidateRef::Borrowed(m) if m.to_value().is_some_and(|v| *v > 50))
}

fn wrap_scalar_composite(
    mov: SequentialCompositeMove<TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> ScalarMoveUnion<TaskSolution, i32> {
    ScalarMoveUnion::Composite(mov)
}

fn composite_values(
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

fn keep_first_value_10(
    candidate: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> bool {
    composite_values(candidate).is_some_and(|(first, _)| first == 10)
}

#[test]
fn filters_moves_by_predicate() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        0,
        "priority",
        vec![10, 20, 30],
    );
    let filtered = FilteringMoveSelector::new(inner, high_value_filter);

    let moves: Vec<_> = filtered.iter_moves(&director).collect();
    assert!(moves.is_empty());
}

#[test]
fn filters_cartesian_candidates_without_materializing_the_full_stream() {
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
    let filtered = FilteringMoveSelector::new(cartesian, keep_first_value_10);

    let mut cursor = filtered.open_cursor(&director);
    let indices =
        collect_cursor_indices::<TaskSolution, ScalarMoveUnion<TaskSolution, i32>, _>(&mut cursor);

    assert!(indices.len() >= 2);
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .and_then(composite_values)
        .is_some_and(|(first, _)| first == 10)));
    assert!(cursor
        .candidate(indices[0])
        .is_some_and(|candidate| candidate.is_doable(&director)));
    assert!(cursor.take_candidate(indices[0]).is_doable(&director));
}
