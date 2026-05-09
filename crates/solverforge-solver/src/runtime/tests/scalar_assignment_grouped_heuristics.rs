fn assignment_config_with_heuristic(
    heuristic: ConstructionHeuristicType,
) -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: heuristic,
        ..assignment_config()
    }
}

fn assignment_model_without_value_order(
) -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    assignment_model_with_order_hooks(true, false)
}

fn assignment_model_without_entity_order(
) -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    assignment_model_with_order_hooks(false, true)
}

fn assignment_model_with_order_hooks(
    include_entity_order: bool,
    include_value_order: bool,
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
    let mut group = ScalarGroup::assignment(
        "slot_assignment",
        ScalarTarget::from_descriptor_index(0, "worker"),
    )
    .with_required_entity(coverage_required)
    .with_capacity_key(coverage_capacity_key)
    .with_position_key(coverage_position_key)
    .with_sequence_key(coverage_sequence_key);
    if include_entity_order {
        group = group.with_entity_order(coverage_entity_order);
    }
    if include_value_order {
        group = group.with_value_order(coverage_value_order);
    }
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot)]).with_scalar_groups(
        bind_scalar_groups(vec![group], &[scalar_slot]),
    )
}

#[test]
fn scalar_assignment_cheapest_insertion_scores_required_assignment_values() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(
            2,
            vec![
                coverage_slot(true, 0, None, &[0, 1]),
                coverage_slot(false, 0, Some(0), &[0]),
            ],
        ),
        assignment_config_with_heuristic(ConstructionHeuristicType::CheapestInsertion),
        assignment_model(),
    );

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, Some(1));
    assert_eq!(slots[1].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn scalar_assignment_weakest_fit_uses_assignment_value_order() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(2, vec![coverage_slot(true, 0, None, &[0, 1])]),
        assignment_config_with_heuristic(ConstructionHeuristicType::WeakestFit),
        assignment_model(),
    );

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(0));
}

#[test]
fn scalar_assignment_strongest_fit_uses_assignment_value_order() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(2, vec![coverage_slot(true, 0, None, &[0, 1])]),
        assignment_config_with_heuristic(ConstructionHeuristicType::StrongestFit),
        assignment_model(),
    );

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(1));
}

#[test]
#[should_panic(expected = "requires ScalarGroup::with_value_order")]
fn scalar_assignment_strength_heuristics_validate_value_order_hook() {
    let _ = solve_assignment_with_config_and_model(
        coverage_plan(2, vec![coverage_slot(true, 0, None, &[0, 1])]),
        assignment_config_with_heuristic(ConstructionHeuristicType::WeakestFit),
        assignment_model_without_value_order(),
    );
}

#[test]
#[should_panic(expected = "requires ScalarGroup::with_entity_order")]
fn scalar_assignment_decreasing_heuristics_validate_entity_order_hook() {
    let _ = solve_assignment_with_config_and_model(
        coverage_plan(2, vec![coverage_slot(true, 0, None, &[0, 1])]),
        assignment_config_with_heuristic(ConstructionHeuristicType::FirstFitDecreasing),
        assignment_model_without_entity_order(),
    );
}

#[test]
fn scalar_assignment_optional_construction_remains_score_improving_only() {
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(
            1,
            vec![coverage_slot_with_penalty(false, 0, None, &[0], 5)],
        ),
        assignment_config(),
        assignment_model(),
    );

    assert_eq!(solver_scope.working_solution().slots[0].assigned, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -1))
    );
}
