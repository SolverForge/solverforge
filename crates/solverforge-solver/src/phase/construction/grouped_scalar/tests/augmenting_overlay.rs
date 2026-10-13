use super::*;

fn required_except_first(_: &AssignmentPlan, entity: usize) -> bool {
    entity != 0
}

fn split_capacity(_: &AssignmentPlan, entity: usize, value: usize) -> Option<usize> {
    Some(if entity < 2 { value } else { value + 10 })
}

fn sequence(_: &AssignmentPlan, entity: usize, _: usize) -> Option<usize> {
    Some(entity)
}

fn rejects_every_edge(_: &AssignmentPlan, _: usize, _: usize, _: usize, _: usize) -> bool {
    false
}

fn mixed_binding() -> ScalarAssignmentBinding<AssignmentPlan> {
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
            "mixed_assignment",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_entity(required_except_first)
        .with_capacity_key(split_capacity)
        .with_sequence_key(sequence)
        .with_assignment_rule(rejects_every_edge)],
        &[slot],
    );
    match groups.into_iter().next().unwrap().kind {
        ScalarGroupBindingKind::Assignment(binding) => binding,
        _ => panic!("expected assignment group"),
    }
}

#[test]
fn failed_augmenting_search_does_not_reorder_committed_indexes() {
    let binding = mixed_binding();
    let plan = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), None, Some(0)],
        candidates: vec![vec![0], vec![0], vec![0]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &plan);
    let before = state.index_snapshot();

    let candidate = assignment_move_for_entity_value(
        &binding,
        &plan,
        &mut state,
        AssignmentRequest::root(1, 0, 2),
        ScalarAssignmentMoveOptions::for_construction(ScalarGroupLimits::new()),
        AssignmentMoveIntent::required(),
        &mut || false,
    );

    assert!(candidate.is_none());
    assert_eq!(state.index_snapshot(), before);
}

#[test]
fn interrupted_augmenting_search_does_not_reorder_committed_indexes() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        ..ScalarGroupLimits::new()
    };
    let binding = assignment_binding(&descriptor, limits);
    let plan = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1), None, Some(0)],
        candidates: vec![vec![0, 2, 3], vec![1, 2], vec![0, 1], vec![0]],
    };
    let mut state = ScalarAssignmentState::new(&binding, &plan);
    let before = state.index_snapshot();
    let mut polls = 0;

    let candidate = assignment_move_for_entity_value(
        &binding,
        &plan,
        &mut state,
        AssignmentRequest::root(2, 0, limits.max_augmenting_depth.unwrap()),
        ScalarAssignmentMoveOptions::for_construction(limits),
        AssignmentMoveIntent::required(),
        &mut || {
            polls += 1;
            polls >= 5
        },
    );

    assert!(candidate.is_none());
    assert!(
        polls >= 5,
        "the cutoff must interrupt after a rolled-back branch"
    );
    assert_eq!(state.index_snapshot(), before);
}

fn optional(_: &AssignmentPlan, _: usize) -> bool {
    false
}

fn alternating_required(_: &AssignmentPlan, entity: usize) -> bool {
    entity.is_multiple_of(2)
}

fn parity_edge(plan: &AssignmentPlan, left: usize, value: usize, right: usize, _: usize) -> bool {
    plan.assignments[left] != Some(value) || (left + right + value).is_multiple_of(2)
}

fn differential_binding(
    required_entity: fn(&AssignmentPlan, usize) -> bool,
    with_capacity: bool,
    with_rule: bool,
) -> ScalarAssignmentBinding<AssignmentPlan> {
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
    let mut group = ScalarGroup::assignment(
        "differential_assignment",
        ScalarTarget::from_descriptor_index(0, "worker"),
    )
    .with_required_entity(required_entity);
    if with_capacity {
        group = group.with_capacity_key(capacity_key);
    }
    if with_rule {
        group = group
            .with_sequence_key(sequence)
            .with_assignment_rule(parity_edge);
    }
    match bind_scalar_groups(vec![group], &[slot])
        .into_iter()
        .next()
        .unwrap()
        .kind
    {
        ScalarGroupBindingKind::Assignment(binding) => binding,
        _ => panic!("expected assignment group"),
    }
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

fn reference_candidate(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    entity: usize,
    value: usize,
    depth: usize,
    allow_optional_displacement: bool,
) -> Option<Vec<(usize, Option<usize>)>> {
    let mut values = plan.assignments.clone();
    if values[entity] == Some(value) {
        return None;
    }
    let mut visiting = solverforge_core::fast_hash::FastSet::default();
    let mut edits = Vec::new();
    let produced = reference_assign(
        group,
        plan,
        &mut values,
        entity,
        value,
        depth,
        allow_optional_displacement,
        &mut visiting,
        &mut edits,
    );
    (produced && reference_final_feasible(group, plan, &edits)).then_some(edits)
}

#[allow(clippy::too_many_arguments)]
fn reference_assign(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    values: &mut Vec<Option<usize>>,
    entity: usize,
    value: usize,
    depth: usize,
    allow_optional_displacement: bool,
    visiting: &mut solverforge_core::fast_hash::FastSet<usize>,
    edits: &mut Vec<(usize, Option<usize>)>,
) -> bool {
    if !group.value_is_legal(plan, entity, Some(value)) {
        return false;
    }
    if values[entity] == Some(value) {
        return true;
    }
    let (mut blockers, mut verdict) = reference_blockers(group, plan, values, entity, value);
    if allow_optional_displacement {
        for blocker in blockers.iter().copied() {
            if !group.is_required(plan, blocker) {
                values[blocker] = None;
                edits.push((blocker, None));
            }
        }
        (blockers, verdict) = reference_blockers(group, plan, values, entity, value);
    }
    if blockers.is_empty() && !verdict.unwrap_or(true) {
        return false;
    }
    if blockers.is_empty() {
        values[entity] = Some(value);
        edits.push((entity, Some(value)));
        return true;
    }
    if depth == 0 || !visiting.insert(entity) {
        return false;
    }
    let Some(blocker) = blockers
        .iter()
        .copied()
        .find(|blocker| group.is_required(plan, *blocker))
    else {
        visiting.remove(&entity);
        return false;
    };
    for alternative in group.candidate_values(plan, blocker, None) {
        if values[blocker] == Some(alternative) {
            continue;
        }
        let saved_values = values.clone();
        let edit_checkpoint = edits.len();
        if reference_assign(
            group,
            plan,
            values,
            blocker,
            alternative,
            depth - 1,
            allow_optional_displacement,
            visiting,
            edits,
        ) {
            let (remaining, allowed) = reference_blockers(group, plan, values, entity, value);
            if remaining.is_empty() && allowed.unwrap_or(true) {
                values[entity] = Some(value);
                edits.push((entity, Some(value)));
                visiting.remove(&entity);
                return true;
            }
        }
        *values = saved_values;
        edits.truncate(edit_checkpoint);
    }
    visiting.remove(&entity);
    false
}

fn reference_blockers(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    values: &[Option<usize>],
    entity: usize,
    value: usize,
) -> (Vec<usize>, Option<bool>) {
    let mut blockers = Vec::new();
    if let Some(key) = group.capacity_key(plan, entity, value) {
        blockers.extend(values.iter().enumerate().filter_map(|(occupant, current)| {
            (current.is_some()
                && occupant != entity
                && current.is_some_and(|current| {
                    group.capacity_key(plan, occupant, current) == Some(key)
                }))
            .then_some(occupant)
        }));
    }
    let verdict = if group.has_assignment_rule() {
        let allowed = reference_rule_allows(group, plan, values, entity, value);
        if !allowed {
            if let Some(target) = group.sequence_key(plan, entity, value) {
                for adjacent in [target.checked_sub(1), target.checked_add(1)]
                    .into_iter()
                    .flatten()
                {
                    blockers.extend(values.iter().enumerate().filter_map(|(blocker, current)| {
                        (*current == Some(value)
                            && blocker != entity
                            && group.sequence_key(plan, blocker, value) == Some(adjacent))
                        .then_some(blocker)
                    }));
                }
            }
        }
        Some(allowed)
    } else {
        None
    };
    blockers.sort_unstable();
    blockers.dedup();
    (blockers, verdict)
}

fn reference_rule_allows(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    values: &[Option<usize>],
    entity: usize,
    value: usize,
) -> bool {
    let Some(target) = group.sequence_key(plan, entity, value) else {
        return true;
    };
    values.iter().enumerate().all(|(neighbor, current)| {
        if neighbor == entity || *current != Some(value) {
            return true;
        }
        let sequence = group.sequence_key(plan, neighbor, value);
        if sequence == target.checked_sub(1) {
            group.assignment_edge_allowed(plan, neighbor, value, entity, value)
        } else if sequence == target.checked_add(1) {
            group.assignment_edge_allowed(plan, entity, value, neighbor, value)
        } else {
            true
        }
    })
}

fn reference_final_feasible(
    group: &ScalarAssignmentBinding<AssignmentPlan>,
    plan: &AssignmentPlan,
    edits: &[(usize, Option<usize>)],
) -> bool {
    let mut values = plan.assignments.clone();
    let mut targets = solverforge_core::fast_hash::FastSet::default();
    let mut capacity_keys = solverforge_core::fast_hash::FastSet::default();
    let mut edges = solverforge_core::fast_hash::FastSet::default();
    for &(entity, next) in edits {
        if entity >= values.len()
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
                if let Some(previous) = sequence.checked_sub(1) {
                    edges.insert((value, previous, sequence));
                }
                if let Some(next) = sequence.checked_add(1) {
                    edges.insert((value, sequence, next));
                }
            }
        }
        values[entity] = next;
    }
    for key in capacity_keys {
        if values
            .iter()
            .enumerate()
            .filter(|(entity, value)| {
                value.is_some_and(|value| group.capacity_key(plan, *entity, value) == Some(key))
            })
            .count()
            > 1
        {
            return false;
        }
    }
    for (value, left_sequence, right_sequence) in edges {
        for left in 0..values.len() {
            for right in 0..values.len() {
                if left != right
                    && values[left] == Some(value)
                    && values[right] == Some(value)
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

#[test]
fn sparse_augmenting_search_matches_small_reference_simulation() {
    let cases = [
        (differential_binding(required, true, false), true),
        (differential_binding(optional, true, false), false),
        (differential_binding(alternating_required, true, true), true),
        (differential_binding(optional, false, true), false),
    ];
    let options = ScalarAssignmentMoveOptions::for_construction(ScalarGroupLimits::new());
    for (group, allow_optional_displacement) in cases {
        for initial in 0..64 {
            let plan = AssignmentPlan {
                score: None,
                assignments: decoded(initial),
                candidates: vec![vec![0, 1, 2]; 3],
            };
            for entity in 0..3 {
                for value in 0..3 {
                    let expected = reference_candidate(
                        &group,
                        &plan,
                        entity,
                        value,
                        options.max_depth,
                        allow_optional_displacement,
                    );
                    let mut state = ScalarAssignmentState::new(&group, &plan);
                    let before = state.index_snapshot();
                    let intent = if allow_optional_displacement {
                        AssignmentMoveIntent::required()
                    } else {
                        AssignmentMoveIntent::optional()
                    };
                    let actual = assignment_move_for_entity_value(
                        &group,
                        &plan,
                        &mut state,
                        AssignmentRequest::root(entity, value, options.max_depth),
                        options,
                        intent,
                        &mut || false,
                    )
                    .map(|mov| {
                        mov.edits()
                            .iter()
                            .map(|edit| (edit.entity_index, edit.to_value))
                            .collect::<Vec<_>>()
                    });
                    assert_eq!(
                        actual, expected,
                        "initial={initial}, entity={entity}, value={value}"
                    );
                    assert_eq!(state.index_snapshot(), before);
                }
            }
        }
    }
}
