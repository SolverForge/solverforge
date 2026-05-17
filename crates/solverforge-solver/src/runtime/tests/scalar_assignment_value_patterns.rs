#[test]
fn scalar_assignment_value_window_swap_emits_multi_day_value_pattern() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(0), &[0, 1]),
            coverage_slot(true, 2, Some(1), &[0, 1]),
            coverage_slot(true, 3, Some(1), &[0, 1]),
            coverage_slot(true, 4, Some(1), &[0, 1]),
            coverage_slot(true, 5, Some(1), &[0, 1]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        64,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_window_swap"
                && mov.edits().len() == 3
                && mov.edits().iter().any(|edit| edit.to_value == Some(0))
                && mov.edits().iter().any(|edit| edit.to_value == Some(1))
        }),
        "{actual:?}"
    );
}

#[test]
fn scalar_assignment_value_window_swap_emits_adjacent_two_day_exchange() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(1), &[0, 1]),
        ],
    );
    let moves = value_window_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        64,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_window_swap"
                && mov.edits().len() == 2
                && mov
                    .edits()
                    .iter()
                    .any(|edit| edit.entity_index == 0 && edit.to_value == Some(1))
                && mov
                    .edits()
                    .iter()
                    .any(|edit| edit.entity_index == 1 && edit.to_value == Some(0))
        }),
        "{actual:?}"
    );
}

#[test]
fn scalar_assignment_value_long_window_swap_emits_rematch_size_pattern() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(0), &[0, 1]),
            coverage_slot(true, 2, Some(0), &[0, 1]),
            coverage_slot(true, 3, Some(0), &[0, 1]),
            coverage_slot(true, 4, Some(1), &[0, 1]),
            coverage_slot(true, 5, Some(1), &[0, 1]),
            coverage_slot(true, 6, Some(1), &[0, 1]),
            coverage_slot(true, 7, Some(1), &[0, 1]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(5),
            ..ScalarGroupLimits::new()
        },
        512,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_long_window_swap" && mov.edits().len() >= 5
        }),
        "{actual:?}"
    );
}

#[test]
fn scalar_assignment_value_block_reassignment_emits_multi_day_value_pattern() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(0), &[0, 1]),
            coverage_slot(true, 2, Some(0), &[0, 1]),
            coverage_slot(true, 5, Some(1), &[0, 1]),
        ],
    );
    let moves = value_block_reassignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(4),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        64,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_block_reassignment"
                && mov.edits().len() == 3
                && mov
                    .edits()
                    .iter()
                    .all(|edit| edit.entity_index < 3 && edit.to_value == Some(1))
        }),
        "{actual:?}"
    );

    let stream_moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(4),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        64,
    );
    let stream_actual = stream_moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        stream_moves.iter().any(|mov| {
            matches!(
                mov.reason(),
                "scalar_assignment_value_window_swap"
                    | "scalar_assignment_value_block_reassignment"
            ) && mov.edits().len() == 3
                && mov
                    .edits()
                    .iter()
                    .all(|edit| edit.entity_index < 3 && edit.to_value == Some(1))
        }),
        "{stream_actual:?}"
    );
}

#[test]
fn scalar_assignment_value_window_cycle_emits_multi_day_value_rotation() {
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1, 2]),
            coverage_slot(true, 0, Some(1), &[0, 1, 2]),
            coverage_slot(true, 0, Some(2), &[0, 1, 2]),
            coverage_slot(true, 1, Some(0), &[0, 1, 2]),
            coverage_slot(true, 1, Some(1), &[0, 1, 2]),
            coverage_slot(true, 1, Some(2), &[0, 1, 2]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        256,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_window_cycle"
                && mov.edits().len() == 6
                && mov.edits().iter().any(|edit| edit.to_value == Some(0))
                && mov.edits().iter().any(|edit| edit.to_value == Some(1))
                && mov.edits().iter().any(|edit| edit.to_value == Some(2))
        }),
        "{actual:?}"
    );
}

#[test]
fn scalar_assignment_value_run_gap_swap_emits_gap_filling_exchange() {
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1]),
            coverage_slot(true, 1, Some(1), &[0, 1]),
            coverage_slot(true, 2, Some(0), &[0, 1]),
            coverage_slot(true, 5, Some(0), &[0, 1]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        512,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_run_gap_swap"
                && mov.edits().len() == 2
                && mov
                    .edits()
                    .iter()
                    .any(|edit| edit.entity_index == 1 && edit.to_value == Some(0))
                && mov
                    .edits()
                    .iter()
                    .any(|edit| edit.entity_index == 3 && edit.to_value == Some(1))
        }),
        "{actual:?}"
    );
}

#[test]
fn scalar_assignment_value_run_release_emits_multi_day_optional_release() {
    let plan = coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, Some(0), &[0]),
            coverage_slot(false, 1, Some(0), &[0]),
            coverage_slot(false, 2, Some(0), &[0]),
            coverage_slot(true, 3, Some(0), &[0]),
        ],
    );
    let moves = selector_assignment_moves_for_plan(
        &plan,
        ScalarGroupLimits {
            max_augmenting_depth: Some(3),
            max_rematch_size: Some(4),
            ..ScalarGroupLimits::new()
        },
        128,
    );

    let actual = moves
        .iter()
        .map(|mov| {
            (
                mov.reason(),
                mov.edits()
                    .iter()
                    .map(|edit| (edit.entity_index, edit.to_value))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        moves.iter().any(|mov| {
            mov.reason() == "scalar_assignment_value_run_release"
                && mov.edits().len() == 2
                && mov
                    .edits()
                    .iter()
                    .all(|edit| edit.entity_index < 2 && edit.to_value.is_none())
        }),
        "{actual:?}"
    );
}
