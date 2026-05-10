enum ConstructionSelection {
    Selected(ConstructionChoice),
    Interrupted,
}

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

    if matches!(selection, ConstructionChoice::Select(_))
        || keep_current_allowed(placement.keep_current_legal(), construction_obligation)
        || (placement.keep_current_legal()
            && matches!(
                construction_obligation,
                ConstructionObligation::AssignWhenCandidateExists
            ))
    {
        if !completion_target.is_empty() {
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

fn select_move_index<S, D, BestCb, M, Fo>(
    forager: &Fo,
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
    Fo: ConstructionForager<S, M> + 'static,
{
    let erased = forager as &dyn Any;

    if erased.is::<FirstFitForager<S, M>>() {
        return select_first_fit_index(placement, construction_obligation, step_scope);
    }
    if erased.is::<BestFitForager<S, M>>() {
        return select_best_fit_index(placement, construction_obligation, step_scope);
    }
    if erased.is::<FirstFeasibleForager<S, M>>() {
        return select_first_feasible_index(placement, construction_obligation, step_scope);
    }
    if let Some(forager) = erased.downcast_ref::<WeakestFitForager<S, M>>() {
        return select_weakest_fit_index(forager, placement, construction_obligation, step_scope);
    }
    if let Some(forager) = erased.downcast_ref::<StrongestFitForager<S, M>>() {
        return select_strongest_fit_index(forager, placement, construction_obligation, step_scope);
    }

    ConstructionSelection::Selected(
        forager.pick_move_index(placement, step_scope.score_director_mut()),
    )
}

fn select_first_fit_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let mut first_doable = None;
    let baseline_score = keep_current_allowed(placement.keep_current_legal(), construction_obligation)
        .then(|| step_scope.calculate_score());

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        if let Some(baseline_score) = baseline_score {
            let score = evaluate_trial_move(step_scope.score_director_mut(), m);
            step_scope.phase_scope_mut().record_score_calculation();
            if is_first_fit_improvement(baseline_score, score) {
                first_doable = Some(idx);
                step_scope
                    .phase_scope_mut()
                    .record_evaluated_move(evaluation_started.elapsed());
                break;
            }
        } else {
            first_doable = Some(idx);
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            break;
        }
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
    }

    ConstructionSelection::Selected(select_first_fit(first_doable))
}

fn select_best_fit_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let baseline_score = keep_current_allowed(placement.keep_current_legal(), construction_obligation)
        .then(|| step_scope.calculate_score());
    let mut tracker = ScoredChoiceTracker::default();

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let score = evaluate_trial_move(step_scope.score_director_mut(), m);
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());

        tracker.consider(idx, score);
    }

    ConstructionSelection::Selected(select_best_fit(tracker, baseline_score))
}

fn select_first_feasible_index<S, D, BestCb, M>(
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let baseline_score = keep_current_allowed(placement.keep_current_legal(), construction_obligation)
        .then(|| step_scope.calculate_score());

    let mut fallback_tracker = ScoredChoiceTracker::default();
    let mut first_feasible = None;

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return ConstructionSelection::Interrupted;
        }
        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let score = evaluate_trial_move(step_scope.score_director_mut(), m);
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());

        if score.is_feasible() {
            first_feasible = Some(idx);
            break;
        }

        fallback_tracker.consider(idx, score);
    }

    ConstructionSelection::Selected(select_first_feasible(
        first_feasible,
        fallback_tracker,
        baseline_score,
    ))
}

fn select_weakest_fit_index<S, D, BestCb, M>(
    forager: &WeakestFitForager<S, M>,
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let mut best_idx = None;
    let mut min_strength = None;

    for (evaluated, (idx, m)) in placement.moves.iter().enumerate().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, evaluated) {
            return ConstructionSelection::Interrupted;
        }

        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let strength = forager.strength(m, step_scope.score_director().working_solution());
        if min_strength.is_none_or(|best| strength < best) {
            best_idx = Some(idx);
            min_strength = Some(strength);
        }

        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
    }

    let Some(best_idx) = best_idx else {
        return ConstructionSelection::Selected(ConstructionChoice::KeepCurrent);
    };

    if !keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
        return ConstructionSelection::Selected(ConstructionChoice::Select(best_idx));
    }

    let baseline_score = step_scope.calculate_score();
    let score = evaluate_trial_move(step_scope.score_director_mut(), &placement.moves[best_idx]);
    step_scope.phase_scope_mut().record_score_calculation();

    ConstructionSelection::Selected(if score > baseline_score {
        ConstructionChoice::Select(best_idx)
    } else {
        ConstructionChoice::KeepCurrent
    })
}

fn select_strongest_fit_index<S, D, BestCb, M>(
    forager: &StrongestFitForager<S, M>,
    placement: &crate::phase::construction::Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> ConstructionSelection
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S> + 'static,
{
    let mut best_idx = None;
    let mut max_strength = None;

    for (evaluated, (idx, m)) in placement.moves.iter().enumerate().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, evaluated) {
            return ConstructionSelection::Interrupted;
        }

        if !m.is_doable(step_scope.score_director()) {
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let strength = forager.strength(m, step_scope.score_director().working_solution());
        if max_strength.is_none_or(|best| strength > best) {
            best_idx = Some(idx);
            max_strength = Some(strength);
        }

        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
    }

    let Some(best_idx) = best_idx else {
        return ConstructionSelection::Selected(ConstructionChoice::KeepCurrent);
    };

    if !keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
        return ConstructionSelection::Selected(ConstructionChoice::Select(best_idx));
    }

    let baseline_score = step_scope.calculate_score();
    let score = evaluate_trial_move(step_scope.score_director_mut(), &placement.moves[best_idx]);
    step_scope.phase_scope_mut().record_score_calculation();

    ConstructionSelection::Selected(if score > baseline_score {
        ConstructionChoice::Select(best_idx)
    } else {
        ConstructionChoice::KeepCurrent
    })
}

#[cfg(test)]
mod tests;
