fn assignment_model_with_rule(
    rule: crate::planning::ScalarAssignmentRule<CoveragePlan>,
) -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    let scalar_slot = ScalarVariableSlot::new(
        0,
        0,
        "CoverageSlot",
        |solution: &CoveragePlan| solution.slots.len(),
        "worker",
        |solution, entity_index, _variable_index| solution.slots[entity_index].assigned,
        |solution, entity_index, _variable_index, value| {
            solution.slots[entity_index].assigned = value;
        },
        ValueSource::EntitySlice {
            values_for_entity: coverage_values,
        },
        true,
    );
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot)]).with_scalar_groups(
        bind_scalar_groups(
            vec![ScalarGroup::assignment(
                "slot_assignment",
                ScalarTarget::from_descriptor_index(0, "worker"),
            )
            .with_required_entity(coverage_required)
            .with_capacity_key(coverage_capacity_key)
            .with_assignment_rule(rule)
            .with_position_key(coverage_position_key)
            .with_sequence_key(coverage_sequence_key)
            .with_entity_order(coverage_entity_order)
            .with_value_order(coverage_value_order)
            .with_limits(ScalarGroupLimits {
                max_augmenting_depth: Some(3),
                ..ScalarGroupLimits::new()
            })],
            &[scalar_slot],
        ),
    )
}

fn no_adjacent_same_worker(
    solution: &CoveragePlan,
    left_entity: usize,
    left_worker: usize,
    right_entity: usize,
    right_worker: usize,
) -> bool {
    left_worker != right_worker
        || solution.slots[left_entity]
            .day
            .abs_diff(solution.slots[right_entity].day)
            > 1
}

#[test]
fn scalar_assignment_dense_construction_uses_full_required_augmenting_chain() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(
            5,
            vec![
                coverage_slot(true, 0, None, &[0]),
                coverage_slot(true, 0, Some(0), &[0, 1]),
                coverage_slot(true, 0, Some(1), &[1, 2]),
                coverage_slot(true, 0, Some(2), &[2, 3]),
                coverage_slot(true, 0, Some(3), &[3, 4]),
            ],
        ),
        assignment_config(),
        assignment_model_with_limits(ScalarGroupLimits {
            max_augmenting_depth: Some(8),
            ..ScalarGroupLimits::new()
        }),
    );

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, Some(0));
    assert_eq!(slots[1].assigned, Some(1));
    assert_eq!(slots[2].assigned, Some(2));
    assert_eq!(slots[3].assigned, Some(3));
    assert_eq!(slots[4].assigned, Some(4));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn scalar_assignment_rule_filters_required_construction_candidates() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(
            2,
            vec![
                coverage_slot(true, 0, Some(0), &[0]),
                coverage_slot(true, 1, None, &[0, 1]),
            ],
        ),
        assignment_config(),
        assignment_model_with_rule(no_adjacent_same_worker),
    );

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, Some(0));
    assert_eq!(slots[1].assigned, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn scalar_assignment_selector_rechecks_temporal_neighbors_for_compound_moves() {
    let plan = coverage_plan(
        3,
        vec![
            coverage_slot(true, 0, Some(0), &[0, 1, 2]),
            coverage_slot(true, 1, Some(1), &[0, 1, 2]),
            coverage_slot(true, 2, Some(0), &[0, 1, 2]),
            coverage_slot(true, 3, Some(1), &[0, 1, 2]),
        ],
    );
    let model = assignment_model_with_rule(no_adjacent_same_worker);
    let group = &model.scalar_groups()[0];
    let crate::builder::ScalarGroupBindingKind::Assignment(assignment) = group.kind else {
        panic!("test model should contain an assignment-backed scalar group");
    };
    let options =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
            group.limits,
            None,
            64,
            0,
        );
    let moves = crate::phase::construction::grouped_scalar::selector_assignment_moves(
        &assignment,
        &plan,
        options,
    );

    assert!(!moves.is_empty());
    for mov in moves {
        let mut assigned = plan
            .slots
            .iter()
            .map(|slot| slot.assigned)
            .collect::<Vec<_>>();
        for edit in mov.edits() {
            assigned[edit.entity_index] = edit.to_value;
        }
        for (entity_index, worker) in assigned.iter().copied().enumerate() {
            let Some(worker) = worker else {
                continue;
            };
            let day = plan.slots[entity_index].day;
            assert!(
                plan.slots.iter().enumerate().all(|(other_index, slot)| {
                    other_index == entity_index
                        || assigned[other_index] != Some(worker)
                        || slot.day.abs_diff(day) > 1
                }),
                "assignment move {:?} violates temporal neighbor rule",
                mov
            );
        }
    }
}
