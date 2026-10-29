use super::*;

pub(super) fn cheapest_iteration<S, V, DM, IDM, D, ProgressCb>(
    scalar_slots: &[RuntimeScalarSlot<S>],
    list_slots: &[FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>],
    slot_order: &[ScalarOrMixedSlotOrder],
    value_limit: Option<usize>,
    obligation: ConstructionObligation,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> Iteration<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    IDM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut global = None;
    let mut completed = false;
    for entry in slot_order {
        match *entry {
            ScalarOrMixedSlotOrder::Scalar {
                scalar_index,
                construction_slot_index,
            } => {
                let slot = scalar_slots
                    .get(scalar_index)
                    .expect("frozen global scalar slot");
                for entity_index in
                    0..slot.entity_count(phase_scope.score_director().working_solution())
                {
                    let slot_id = crate::phase::construction::ConstructionSlotId::new(
                        construction_slot_index,
                        entity_index,
                    );
                    if phase_scope.solver_scope().is_scalar_slot_completed(slot_id)
                        || slot
                            .current_value(
                                phase_scope.score_director().working_solution(),
                                entity_index,
                            )
                            .is_some()
                    {
                        continue;
                    }
                    let values = candidate_values(
                        slot,
                        phase_scope.score_director().working_solution(),
                        entity_index,
                        value_limit,
                    );
                    if values.is_empty() {
                        if slot.allows_unassigned() {
                            complete_scalar(slot_id, false, phase_scope);
                            completed = true;
                        }
                        continue;
                    }
                    let baseline = keep_current_allowed(slot.allows_unassigned(), obligation)
                        .then(|| phase_scope.calculate_score());
                    let mut best = None;
                    for (value_index, value) in values.into_iter().enumerate() {
                        let move_ =
                            RuntimeScalarConstructionMove::new(slot.clone(), entity_index, value);
                        let Some((score, trace)) = evaluate(
                            &move_,
                            value_index,
                            CandidateTraceConstructionTarget {
                                descriptor_index: move_.descriptor_index(),
                                entity_index,
                            },
                            phase_scope,
                        ) else {
                            continue;
                        };
                        let candidate = Candidate::Scalar {
                            move_,
                            slot_index: construction_slot_index,
                            score,
                            order: [construction_slot_index, entity_index, value_index, 0],
                            trace,
                        };
                        replace_best(&mut best, candidate, phase_scope);
                    }
                    match best {
                        Some(candidate)
                            if baseline.is_none_or(|baseline| candidate.score() >= baseline) =>
                        {
                            replace_best(&mut global, candidate, phase_scope)
                        }
                        Some(candidate) => {
                            candidate.ignore(phase_scope);
                            complete_scalar(slot_id, true, phase_scope);
                            completed = true;
                        }
                        None if slot.allows_unassigned() => {
                            complete_scalar(slot_id, false, phase_scope);
                            completed = true;
                        }
                        None => {}
                    }
                }
            }
            ScalarOrMixedSlotOrder::List {
                list_index,
                construction_slot_index,
            } => {
                let list = list_slots.get(list_index).expect("frozen global list slot");
                let sources =
                    current_list_sources(list, phase_scope.score_director().working_solution());
                for source in &sources {
                    let element_id = crate::phase::construction::ConstructionListElementId::new(
                        construction_slot_index,
                        source.source_index,
                    );
                    if phase_scope
                        .solver_scope()
                        .is_list_element_completed(element_id)
                    {
                        continue;
                    }
                    let mut best = None;
                    let mut candidate_index = 0;
                    let owner_restriction = list.slot.element_owner(
                        phase_scope.score_director().working_solution(),
                        &source.element,
                    );
                    let entity_count = list
                        .slot
                        .entity_count(phase_scope.score_director().working_solution());
                    let owner_restriction = match owner_restriction {
                        Ok(None) => Some(list_placement::OwnerRestriction::Unrestricted),
                        Ok(Some(owner)) if owner < entity_count => {
                            Some(list_placement::OwnerRestriction::Fixed(owner))
                        }
                        _ => Some(list_placement::OwnerRestriction::Invalid),
                    };
                    if let Some(owner_restriction) = owner_restriction {
                        for entity_index in owner_restriction.candidates(entity_count) {
                            let len = list.slot.list_len(
                                phase_scope.score_director().working_solution(),
                                entity_index,
                            );
                            for position in 0..=len {
                                let move_ = RuntimeListInsertionMove::new(
                                    list.slot.clone(),
                                    source.element.clone(),
                                    source.source_index,
                                    entity_index,
                                    position,
                                );
                                let Some((score, trace)) = evaluate(
                                    &move_,
                                    candidate_index,
                                    CandidateTraceConstructionTarget {
                                        descriptor_index: move_.descriptor_index(),
                                        entity_index,
                                    },
                                    phase_scope,
                                ) else {
                                    candidate_index += 1;
                                    continue;
                                };
                                candidate_index += 1;
                                replace_best(
                                    &mut best,
                                    Candidate::List {
                                        move_,
                                        score,
                                        order: [
                                            construction_slot_index,
                                            source.source_index,
                                            entity_index,
                                            position,
                                        ],
                                        trace,
                                    },
                                    phase_scope,
                                );
                            }
                        }
                    }
                    if let Some(candidate) = best {
                        replace_best(&mut global, candidate, phase_scope);
                    } else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                    }
                }
            }
        }
    }
    if let Some(candidate) = global {
        Iteration::Candidate(candidate)
    } else if completed {
        Iteration::CompletedOnly
    } else {
        Iteration::None
    }
}
