fn scan_scalar_best_fit<S, V, D, ProgressCb>(
    variable_index: usize,
    ctx: ScalarVariableSlot<S>,
    value_candidate_limit: Option<usize>,
    construction_obligation: ConstructionObligation,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());

    for entity_index in 0..entity_count {
        let slot_id = ConstructionSlotId::new(variable_index, entity_index);
        if phase_scope.solver_scope().is_scalar_slot_completed(slot_id) {
            continue;
        }

        let current = ctx.current_value(
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        if current.is_some() {
            continue;
        }

        let values = ctx.candidate_values_for_entity(
            phase_scope.score_director().working_solution(),
            entity_index,
            value_candidate_limit,
        );
        if values.is_empty() {
            if ctx.allows_unassigned {
                complete_scalar_slot(
                    slot_id,
                    ScalarSlotCompletion::NoDoableCandidate,
                    phase_scope,
                );
                return IterationProgress::CompletedOnly;
            }
            continue;
        }

        let baseline_score =
            keep_current_allowed(ctx.allows_unassigned, construction_obligation)
                .then(|| phase_scope.calculate_score());
        let mut tracker = ScoredChoiceTracker::default();
        let mut best: Option<(usize, usize, S::Score)> = None;

        for (value_index, value) in values.into_iter().enumerate() {
            let mov = ChangeMove::new(
                entity_index,
                Some(value),
                ctx.getter,
                ctx.setter,
                ctx.variable_index,
                ctx.variable_name,
                ctx.descriptor_index,
            );
            if !mov.is_doable(phase_scope.score_director()) {
                continue;
            }
            let score = candidate_score(phase_scope, &mov);
            tracker.consider(value_index, score);
            let should_replace = match best {
                None => true,
                Some((_, _, best_score)) => score > best_score,
            };
            if should_replace {
                best = Some((value_index, value, score));
            }
        }

        match (select_best_fit(tracker, baseline_score), best) {
            (_, None) => {
                if ctx.allows_unassigned {
                    complete_scalar_slot(
                        slot_id,
                        ScalarSlotCompletion::NoDoableCandidate,
                        phase_scope,
                    );
                    return IterationProgress::CompletedOnly;
                }
            }
            (
                crate::phase::construction::ConstructionChoice::Select(_),
                Some((value_index, value, score)),
            ) => {
                return IterationProgress::Committed(Candidate::Scalar {
                    getter: ctx.getter,
                    setter: ctx.setter,
                    variable_name: ctx.variable_name,
                    descriptor_index: ctx.descriptor_index,
                    variable_index: ctx.variable_index,
                    entity_index,
                    value,
                    order_key: [variable_index, entity_index, value_index, 0],
                    score,
                });
            }
            (crate::phase::construction::ConstructionChoice::KeepCurrent, Some(_)) => {
                let completion =
                    if keep_current_allowed(ctx.allows_unassigned, construction_obligation) {
                        ScalarSlotCompletion::Kept
                    } else {
                        ScalarSlotCompletion::NoDoableCandidate
                    };
                complete_scalar_slot(slot_id, completion, phase_scope);
                return IterationProgress::CompletedOnly;
            }
        }
    }

    IterationProgress::None
}

fn scan_list_best_fit<S, V, DM, IDM, D, ProgressCb>(
    list_index: usize,
    ctx: ListVariableSlot<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    V: Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Debug + 'static,
    DM: Clone + Debug + 'static,
    IDM: Clone + Debug + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_count = (ctx.entity_count)(phase_scope.score_director().working_solution());
    if entity_count == 0 {
        return IterationProgress::None;
    }

    let assigned = (ctx.assigned_elements)(phase_scope.score_director().working_solution());
    let assigned_set: std::collections::HashSet<V> = assigned.into_iter().collect();
    let element_count = (ctx.element_count)(phase_scope.score_director().working_solution());
    let mut best_candidate: Option<Candidate<S, V>> = None;

    for element_index in 0..element_count {
        let element_id = ConstructionListElementId::new(list_index, element_index);
        if phase_scope
            .solver_scope()
            .is_list_element_completed(element_id)
        {
            continue;
        }

        let element = (ctx.index_to_element)(
            phase_scope.score_director().working_solution(),
            element_index,
        );
        if assigned_set.contains(&element) {
            continue;
        }

        let mut best_for_element: Option<(usize, usize, S::Score)> = None;
        for entity_index in 0..entity_count {
            let len = (ctx.list_len)(
                phase_scope.score_director().working_solution(),
                entity_index,
            );
            for position in 0..=len {
                let score =
                    evaluate_list_insertion(phase_scope, &ctx, element, entity_index, position);
                let should_replace = match best_for_element {
                    None => true,
                    Some((_, _, best_score)) => score > best_score,
                };
                if should_replace {
                    best_for_element = Some((entity_index, position, score));
                }
            }
        }

        if let Some((entity_index, position, score)) = best_for_element {
            update_best_candidate(
                &mut best_candidate,
                Candidate::List {
                    list_insert: ctx.list_insert,
                    descriptor_index: ctx.descriptor_index,
                    element,
                    entity_index,
                    position,
                    order_key: [list_index, element_index, entity_index, position],
                    score,
                },
            );
        } else {
            phase_scope
                .solver_scope_mut()
                .mark_list_element_completed(element_id);
        }
    }

    best_candidate
        .map(IterationProgress::Committed)
        .unwrap_or(IterationProgress::None)
}
