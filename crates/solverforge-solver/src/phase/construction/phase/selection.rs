fn filter_completed_scalar_placements<S, D, BestCb, M>(
    placements: Vec<Placement<S, M>>,
    solver_scope: &SolverScope<'_, S, D, BestCb>,
) -> Vec<Placement<S, M>>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    placements
        .into_iter()
        .filter(|placement| !placement_completed(placement, solver_scope))
        .collect()
}

fn commit_selection<S, D, BestCb, M>(
    placement: &mut Placement<S, M>,
    selection: ConstructionChoice,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    S::Score: Copy,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let completion_target = match selection {
        ConstructionChoice::KeepCurrent => placement.construction_target().clone(),
        ConstructionChoice::Select(idx) => placement.construction_target_for_move(idx).clone(),
    };

    match selection {
        ConstructionChoice::KeepCurrent => {}
        ConstructionChoice::Select(idx) => {
            step_scope.phase_scope_mut().record_move_accepted();
            let m = placement.take_move(idx);
            step_scope.apply_committed_move(&m);
            step_scope.phase_scope_mut().record_move_applied();
            if !completion_target.is_empty() {
                step_scope
                    .phase_scope_mut()
                    .record_construction_slot_assigned();
            }
        }
    }

    if construction_step_needs_score(placement.keep_current_legal(), construction_obligation) {
        let step_score = step_scope.calculate_score();
        step_scope.set_step_score(step_score);
    }

    let should_mark_completion = matches!(selection, ConstructionChoice::Select(_))
        || keep_current_allowed(placement.keep_current_legal(), construction_obligation)
        || (placement.keep_current_legal()
            && matches!(
                construction_obligation,
                ConstructionObligation::AssignWhenCandidateExists
            ));
    if should_mark_completion && !completion_target.is_empty() {
        mark_completed_target(step_scope, &completion_target);
        if matches!(selection, ConstructionChoice::KeepCurrent) {
            if keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
                step_scope
                    .phase_scope_mut()
                    .record_construction_slot_kept();
            } else {
                step_scope
                    .phase_scope_mut()
                    .record_construction_slot_no_doable();
            }
        }
    }
}

fn construction_step_needs_score(
    keep_current_legal: bool,
    construction_obligation: ConstructionObligation,
) -> bool {
    keep_current_allowed(keep_current_legal, construction_obligation)
}

fn placement_completed<S, D, BestCb, M>(
    placement: &Placement<S, M>,
    solver_scope: &SolverScope<'_, S, D, BestCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    target_completed(placement.construction_target(), solver_scope)
}

fn target_completed<S, D, BestCb>(
    target: &ConstructionTarget,
    solver_scope: &SolverScope<'_, S, D, BestCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    target
        .group_slot()
        .is_some_and(|group_slot| solver_scope.is_group_slot_completed(group_slot))
        || target
            .scalar_slots()
            .iter()
            .copied()
            .any(|slot_id| solver_scope.is_scalar_slot_completed(slot_id))
}

fn mark_completed_target<S, D, BestCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    target: &ConstructionTarget,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    if let Some(group_slot) = target.group_slot() {
        step_scope
            .phase_scope_mut()
            .solver_scope_mut()
            .mark_group_slot_completed(group_slot.clone());
    }
    for slot_id in target.scalar_slots().iter().copied() {
        step_scope
            .phase_scope_mut()
            .solver_scope_mut()
            .mark_scalar_slot_completed(slot_id);
    }
}

#[cfg(test)]
mod tests;
