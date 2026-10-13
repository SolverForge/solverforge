use super::super::assignment_cycle::CycleWindowCursor;
use super::super::assignment_entity::assigned_entities_by_position;
use super::*;

fn simulation(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    window: &[usize],
    attempt: usize,
    options: ScalarAssignmentMoveOptions,
) -> Option<Vec<(usize, Option<usize>)>> {
    let mut state = ScalarAssignmentState::new(group, plan);
    for &entity in window {
        state.set_value(group, plan, entity, None);
    }
    for offset in 0..window.len() {
        let entity = window[(attempt % window.len() + offset) % window.len()];
        let values = group.candidate_values(plan, entity, options.value_candidate_limit);
        let mut found = None;
        for offset in 0..values.len() {
            let value = values[(attempt % values.len() + offset) % values.len()];
            let mut blockers = Vec::new();
            let verdict = state.blockers(group, plan, entity, value, &mut blockers);
            if blockers.is_empty()
                && verdict.unwrap_or_else(|| state.assignment_allowed(group, plan, entity, value))
            {
                found = Some(value);
                break;
            }
        }
        if let Some(value) = found {
            state.set_value(group, plan, entity, Some(value));
        } else if state.is_required(entity) {
            return None;
        }
    }
    Some(
        window
            .iter()
            .filter_map(|&entity| {
                let next = state.current_value(entity);
                (next != plan.assignments[entity]).then_some((entity, next))
            })
            .collect(),
    )
}

#[test]
fn sparse_ejection_matches_simulated_discovery_order_and_coverage() {
    let bindings = [
        assignment_binding(&descriptor(), ScalarGroupLimits::new()),
        super::overlay_differential::rule_binding(),
    ];
    for binding in bindings {
        for initial in 0..64usize {
            let mut code = initial;
            let assignments: Vec<_> = (0..3)
                .map(|_| {
                    let value = code % 4;
                    code /= 4;
                    (value < 3).then_some(value)
                })
                .collect();
            for offset in [0, 1, 5, usize::MAX] {
                let plan = AssignmentPlan {
                    score: None,
                    assignments: assignments.clone(),
                    candidates: vec![vec![0, 1, 2]; 3],
                };
                let options = ScalarAssignmentMoveOptions::for_selector(
                    ScalarGroupLimits::new(),
                    None,
                    256,
                    offset,
                );
                let mut state = ScalarAssignmentState::new(&binding, &plan);
                let mut entities = assigned_entities_by_position(&binding, &plan, &state);
                order_candidates(&mut entities, options);
                let mut expected = Vec::new();
                for start in 0..entities.len() {
                    for len in 2..=options.max_rematch_size.min(entities.len()).max(2) {
                        if start + len > entities.len() {
                            break;
                        }
                        let window = &entities[start..start + len];
                        for attempt in 0..options.max_depth.max(1) {
                            if let Some(edits) =
                                simulation(&binding, &plan, window, attempt, options)
                            {
                                if edits.len() >= 2
                                    && state
                                        .assignment_feasible_after_edits(&binding, &plan, &edits)
                                {
                                    expected.push(edits);
                                }
                            }
                        }
                    }
                }
                let mut cursor = CycleWindowCursor::ejection(&binding, &plan, &state, options);
                let mut actual = Vec::new();
                while let Some(candidate) = cursor.next(&binding, &plan, &mut state) {
                    actual.push(
                        candidate
                            .edits()
                            .iter()
                            .map(|edit| (edit.entity_index, edit.to_value))
                            .collect::<Vec<_>>(),
                    );
                }
                assert_eq!(actual, expected, "initial={initial}, offset={offset}");
                for (entity, value) in plan.assignments.iter().enumerate() {
                    assert_eq!(state.current_value(entity), *value);
                }
            }
        }
    }
}
