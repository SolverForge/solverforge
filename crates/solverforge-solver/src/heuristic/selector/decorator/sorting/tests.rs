use super::super::test_utils::{create_director, get_priority, set_priority, Task, TaskSolution};
use super::*;
use crate::heuristic::r#move::{ChangeMove, ScalarMoveUnion};
use crate::heuristic::selector::decorator::CartesianProductSelector;
use crate::heuristic::selector::move_selector::{collect_cursor_indices, MoveCandidateRef};
use crate::heuristic::selector::{ChangeMoveSelector, ScalarChangeMoveSelector};

fn by_value_asc(
    a: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>,
    b: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>,
) -> Ordering {
    match (a, b) {
        (MoveCandidateRef::Borrowed(left), MoveCandidateRef::Borrowed(right)) => {
            left.to_value().cmp(&right.to_value())
        }
        _ => Ordering::Equal,
    }
}

fn by_value_desc(
    a: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>,
    b: MoveCandidateRef<'_, TaskSolution, ChangeMove<TaskSolution, i32>>,
) -> Ordering {
    by_value_asc(b, a)
}

fn composite_second_value_desc(
    a: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
    b: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> Ordering {
    composite_second_value(b).cmp(&composite_second_value(a))
}

fn composite_second_value(
    candidate: MoveCandidateRef<'_, TaskSolution, ScalarMoveUnion<TaskSolution, i32>>,
) -> Option<i32> {
    let MoveCandidateRef::Sequential(sequence) = candidate else {
        return None;
    };
    let ScalarMoveUnion::Change(second) = sequence.second() else {
        return None;
    };
    second.to_value().copied()
}

#[test]
fn sorts_ascending() {
    let director = create_director(vec![Task { priority: Some(1) }]);
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
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
    let inner = ChangeMoveSelector::simple(
        get_priority,
        set_priority,
        0,
        0,
        "priority",
        vec![30, 10, 50],
    );
    let sorted = SortingMoveSelector::new(inner, by_value_asc);

    assert_eq!(sorted.size(&director), 3);
}

#[test]
fn sorts_cartesian_candidates_by_borrowed_preview_data() {
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
    let cartesian = CartesianProductSelector::new(left, right);
    let sorted = SortingMoveSelector::new(cartesian, composite_second_value_desc);

    let mut cursor = sorted.open_cursor(&director);
    let indices =
        collect_cursor_indices::<TaskSolution, ScalarMoveUnion<TaskSolution, i32>, _>(&mut cursor);

    assert!(indices.len() >= 2);
    let values: Vec<_> = indices
        .iter()
        .map(|&index| {
            cursor
                .candidate(index)
                .and_then(composite_second_value)
                .expect("sorted cartesian candidate must remain valid")
        })
        .collect();
    assert_eq!(values, vec![40, 40, 30, 30]);
    assert!(cursor
        .candidate(indices[0])
        .is_some_and(|candidate| candidate.is_doable(&director)));
    assert!(cursor
        .candidate(indices[0])
        .is_some_and(|candidate| candidate.is_doable(&director)));
}
