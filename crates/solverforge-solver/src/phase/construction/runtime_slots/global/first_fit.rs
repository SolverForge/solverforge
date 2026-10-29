use super::*;

pub(super) fn first_fit_iteration<S, V, DM, IDM, D, ProgressCb>(
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
    let mut completed = false;
    for entry in slot_order {
        match *entry {
            ScalarOrMixedSlotOrder::Scalar {
                scalar_index,
                construction_slot_index,
            } => {
                let slot = scalar_slots
                    .get(scalar_index)
                    .expect("frozen global order must reference a scalar slot");
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
                            return Iteration::CompletedOnly;
                        }
                        continue;
                    }
                    let baseline = keep_current_allowed(slot.allows_unassigned(), obligation)
                        .then(|| phase_scope.calculate_score());
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
                        if baseline
                            .is_some_and(|baseline| !is_first_fit_improvement(baseline, score))
                        {
                            ignore(trace, phase_scope);
                            continue;
                        }
                        return Iteration::Candidate(Candidate::Scalar {
                            move_,
                            slot_index: construction_slot_index,
                            score,
                            order: [construction_slot_index, entity_index, value_index, 0],
                            trace,
                        });
                    }
                    if slot.allows_unassigned() {
                        complete_scalar(slot_id, baseline.is_some(), phase_scope);
                        return Iteration::CompletedOnly;
                    }
                }
            }
            ScalarOrMixedSlotOrder::List {
                list_index,
                construction_slot_index,
            } => {
                let list = list_slots
                    .get(list_index)
                    .expect("frozen global order must reference a prepared list slot");
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
                    let Some(entity_index) = first_owner(
                        &list.slot,
                        &source.element,
                        phase_scope.score_director().working_solution(),
                    ) else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                        continue;
                    };
                    let move_ = RuntimeListInsertionMove::new(
                        list.slot.clone(),
                        source.element.clone(),
                        source.source_index,
                        entity_index,
                        0,
                    );
                    let Some((score, trace)) = evaluate(
                        &move_,
                        0,
                        CandidateTraceConstructionTarget {
                            descriptor_index: move_.descriptor_index(),
                            entity_index,
                        },
                        phase_scope,
                    ) else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                        continue;
                    };
                    return Iteration::Candidate(Candidate::List {
                        move_,
                        score,
                        order: [
                            construction_slot_index,
                            source.source_index,
                            entity_index,
                            0,
                        ],
                        trace,
                    });
                }
            }
        }
    }
    if completed {
        Iteration::CompletedOnly
    } else {
        Iteration::None
    }
}
