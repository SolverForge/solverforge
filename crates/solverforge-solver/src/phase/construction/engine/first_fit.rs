fn solve_scalar_first_fit<S, V, D, ProgressCb>(
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

        let mut first_doable = None;
        let baseline_score =
            keep_current_allowed(ctx.allows_unassigned, construction_obligation)
                .then(|| phase_scope.calculate_score());

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

            if let Some(baseline_score) = baseline_score {
                if is_first_fit_improvement(baseline_score, score) {
                    first_doable = Some((value_index, value, score));
                    break;
                }
            } else {
                first_doable = Some((value_index, value, score));
                break;
            }
        }

        let selection = select_first_fit(
            first_doable
                .as_ref()
                .map(|(value_index, _, _)| *value_index),
        );

        match selection {
            crate::phase::construction::ConstructionChoice::Select(selected_index) => {
                let Some((value_index, value, score)) =
                    first_doable.filter(|(value_index, _, _)| *value_index == selected_index)
                else {
                    unreachable!("selected scalar construction candidate should exist");
                };
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
            crate::phase::construction::ConstructionChoice::KeepCurrent
                if ctx.allows_unassigned =>
            {
                let completion =
                    if keep_current_allowed(ctx.allows_unassigned, construction_obligation) {
                        ScalarSlotCompletion::Kept
                    } else {
                        ScalarSlotCompletion::NoDoableCandidate
                    };
                complete_scalar_slot(slot_id, completion, phase_scope);
                return IterationProgress::CompletedOnly;
            }
            crate::phase::construction::ConstructionChoice::KeepCurrent => {}
        }
    }

    IterationProgress::None
}

fn solve_list_first_fit<S, V, DM, IDM, D, ProgressCb>(
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

        let solution = phase_scope.score_director().working_solution();
        let mut candidates = crate::list_placement::candidate_entity_indices(
            ctx.element_owner_fn,
            solution,
            entity_count,
            &element,
        );
        if let Some(entity_index) = candidates.next() {
            let position = 0;
            let score =
                evaluate_list_insertion(phase_scope, &ctx, element, entity_index, position);
            return IterationProgress::Committed(Candidate::List {
                list_insert: ctx.list_insert,
                descriptor_index: ctx.descriptor_index,
                element,
                entity_index,
                position,
                order_key: [list_index, element_index, entity_index, position],
                score,
            });
        }

        phase_scope
            .solver_scope_mut()
            .mark_list_element_completed(element_id);
    }

    IterationProgress::None
}

fn solve_dynamic_scalar_first_fit<S, V, D, ProgressCb>(
    variable_index: usize,
    ctx: DynamicScalarVariableSlot<S>,
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
    let entity_count = ctx.entity_count(phase_scope.score_director().working_solution());

    for entity_index in 0..entity_count {
        let slot_id = ConstructionSlotId::new(variable_index, entity_index);
        if phase_scope.solver_scope().is_scalar_slot_completed(slot_id) {
            continue;
        }

        let current = ctx.current_value(phase_scope.score_director().working_solution(), entity_index);
        if current.is_some() {
            continue;
        }

        let values = ctx
            .candidate_values(phase_scope.score_director().working_solution(), entity_index)
            .iter()
            .copied()
            .take(value_candidate_limit.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
        if values.is_empty() {
            if ctx.allows_unassigned {
                complete_scalar_slot(slot_id, ScalarSlotCompletion::NoDoableCandidate, phase_scope);
                return IterationProgress::CompletedOnly;
            }
            continue;
        }

        let baseline_score = keep_current_allowed(ctx.allows_unassigned, construction_obligation)
            .then(|| phase_scope.calculate_score());

        for (value_index, value) in values.into_iter().enumerate() {
            if current == Some(value)
                || !ctx.value_is_legal(
                    phase_scope.score_director().working_solution(),
                    entity_index,
                    Some(value),
                )
            {
                continue;
            }

            let score = evaluate_dynamic_scalar_assignment(
                phase_scope,
                &ctx,
                entity_index,
                Some(value),
            );
            if baseline_score.is_some_and(|baseline| !is_first_fit_improvement(baseline, score)) {
                continue;
            }

            let selection = select_first_fit(Some(value_index));
            if let crate::phase::construction::ConstructionChoice::Select(selected_index) =
                selection
            {
                debug_assert_eq!(selected_index, value_index);
                return IterationProgress::Committed(Candidate::DynamicScalar {
                    slot: ctx,
                    entity_index,
                    value,
                    order_key: [variable_index, entity_index, value_index, 0],
                    score,
                });
            }
        }

        if ctx.allows_unassigned {
            let completion = if keep_current_allowed(ctx.allows_unassigned, construction_obligation)
            {
                ScalarSlotCompletion::Kept
            } else {
                ScalarSlotCompletion::NoDoableCandidate
            };
            complete_scalar_slot(slot_id, completion, phase_scope);
            return IterationProgress::CompletedOnly;
        }
    }

    IterationProgress::None
}

fn solve_dynamic_list_first_fit<S, V, D, ProgressCb>(
    list_index: usize,
    ctx: DynamicListVariableSlot<S>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> IterationProgress<S, V>
where
    S: PlanningSolution,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let entity_count = ctx.entity_count(phase_scope.score_director().working_solution());
    if entity_count == 0 {
        return IterationProgress::None;
    }

    let assigned = ctx.assigned_elements(phase_scope.score_director().working_solution());
    let assigned_set: std::collections::HashSet<usize> = assigned.into_iter().collect();
    let element_count = ctx.element_count(phase_scope.score_director().working_solution());

    for element_index in 0..element_count {
        let element_id = ConstructionListElementId::new(list_index, element_index);
        if phase_scope
            .solver_scope()
            .is_list_element_completed(element_id)
        {
            continue;
        }
        let Some(element) =
            ctx.element(phase_scope.score_director().working_solution(), element_index)
        else {
            continue;
        };
        if assigned_set.contains(&element) {
            continue;
        }

        let entity_index = 0;
        let position = 0;
        let score = evaluate_dynamic_list_insertion(phase_scope, &ctx, element, entity_index, position);
        return IterationProgress::Committed(Candidate::DynamicList {
            slot: ctx,
            element,
            entity_index,
            position,
            order_key: [list_index, element_index, entity_index, position],
            score,
        });
    }

    IterationProgress::None
}
