use super::*;
use solverforge_core::fast_hash::FastSet;

fn sequence(_: &AssignmentPlan, entity: usize, _: usize) -> Option<usize> {
    Some(entity)
}

fn edge(plan: &AssignmentPlan, left: usize, value: usize, right: usize, _: usize) -> bool {
    // The solution argument is the baseline, not a speculative assignment snapshot.
    plan.assignments[left] != Some(value) || (left + right + value).is_multiple_of(2)
}

pub(super) fn rule_binding() -> ScalarAssignmentBinding<AssignmentPlan> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        current_value,
        set_value,
        ValueSource::EntitySlice {
            values_for_entity: candidates,
        },
        true,
    )
    .with_candidate_values(candidates);
    let groups = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "rule_assignment",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_entity(required)
        .with_sequence_key(sequence)
        .with_assignment_rule(edge)],
        &[slot],
    );
    match groups.into_iter().next().unwrap().kind {
        ScalarGroupBindingKind::Assignment(binding) => binding.clone(),
        _ => panic!("expected assignment group"),
    }
}

fn reference(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    edits: &[(usize, Option<usize>)],
) -> bool {
    let mut final_values = plan.assignments.clone();
    let mut targets = FastSet::default();
    let mut capacity_keys = FastSet::default();
    let mut edges = FastSet::default();
    for &(entity, next) in edits {
        if entity >= final_values.len()
            || !targets.insert(entity)
            || (next.is_none() && group.is_required(plan, entity))
            || !group.value_is_legal(plan, entity, next)
        {
            return false;
        }
        for value in [plan.assignments[entity], next].into_iter().flatten() {
            if let Some(key) = group.capacity_key(plan, entity, value) {
                capacity_keys.insert(key);
            }
            if let Some(sequence) = group.sequence_key(plan, entity, value) {
                if let Some(left) = sequence.checked_sub(1) {
                    edges.insert((value, left, sequence));
                }
                if let Some(right) = sequence.checked_add(1) {
                    edges.insert((value, sequence, right));
                }
            }
        }
        final_values[entity] = next;
    }
    for key in capacity_keys {
        let count = final_values
            .iter()
            .enumerate()
            .filter(|(entity, value)| {
                value.is_some_and(|value| group.capacity_key(plan, *entity, value) == Some(key))
            })
            .count();
        if count > 1 {
            return false;
        }
    }
    for (value, left_sequence, right_sequence) in edges {
        for left in 0..final_values.len() {
            for right in 0..final_values.len() {
                if left != right
                    && final_values[left] == Some(value)
                    && final_values[right] == Some(value)
                    && group.sequence_key(plan, left, value) == Some(left_sequence)
                    && group.sequence_key(plan, right, value) == Some(right_sequence)
                    && !group.assignment_edge_allowed(plan, left, value, right, value)
                {
                    return false;
                }
            }
        }
    }
    true
}

fn decoded(mut code: usize) -> Vec<Option<usize>> {
    (0..3)
        .map(|_| {
            let digit = code % 4;
            code /= 4;
            (digit < 3).then_some(digit)
        })
        .collect()
}

#[test]
fn overlay_matches_full_final_assignment_reference_and_preserves_state() {
    let bindings = [
        assignment_binding(&descriptor(), ScalarGroupLimits::new()),
        rule_binding(),
    ];
    for binding in bindings {
        for initial in 0..64 {
            let plan = AssignmentPlan {
                score: None,
                assignments: decoded(initial),
                candidates: vec![vec![0, 1, 2]; 3],
            };
            let mut state = ScalarAssignmentState::new(&binding, &plan);
            for mask in 1..8 {
                for destination in 0..64 {
                    let final_values = decoded(destination);
                    let edits = final_values
                        .iter()
                        .enumerate()
                        .filter(|(entity, _)| mask & (1 << entity) != 0)
                        .map(|(entity, value)| (entity, *value))
                        .collect::<Vec<_>>();
                    assert_eq!(
                        state.assignment_feasible_after_edits(&binding, &plan, &edits),
                        reference(&binding, &plan, &edits),
                        "initial={initial}, mask={mask}, destination={destination}"
                    );
                    for (entity, value) in plan.assignments.iter().enumerate() {
                        assert_eq!(state.current_value(entity), *value);
                    }
                    for value in 0..3 {
                        assert_eq!(
                            state.assigned_count(value),
                            plan.assignments
                                .iter()
                                .filter(|v| **v == Some(value))
                                .count()
                        );
                    }
                }
            }
        }
    }
}
