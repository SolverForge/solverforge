use super::*;

#[allow(clippy::drop_non_drop)]
fn select_strength_index<S, D, BestCb, M, C>(
    placement: &mut Placement<S, M, C>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    strength: impl Fn(&M, &S) -> i64,
    prefer: impl Fn(i64, i64) -> bool,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let mut retained: Option<(CandidateId, i64)> = None;
    loop {
        if evaluation_should_terminate(step_scope) {
            mark_retained_ignored(
                placement,
                retained.map(|(candidate_id, _)| candidate_id),
                step_scope,
            );
            return None;
        }
        let Some(candidate_id) = next_candidate(placement, step_scope) else {
            if should_interrupt_before_candidate(step_scope) {
                mark_retained_ignored(
                    placement,
                    retained.map(|(candidate_id, _)| candidate_id),
                    step_scope,
                );
                return None;
            }
            break;
        };
        let evaluation_started = Instant::now();
        let candidate = placement
            .candidates()
            .candidate(candidate_id)
            .expect("construction candidate must remain live");
        if crate::pinning::move_changes_pinned(&candidate, step_scope.score_director())
            || !candidate.is_doable(step_scope.score_director())
        {
            drop(candidate);
            mark_candidate_disposition(
                placement,
                candidate_id,
                CandidateTraceDisposition::Evaluated,
                step_scope,
            );
            release(
                placement,
                candidate_id,
                CandidateTraceDisposition::NotDoable,
                step_scope,
            );
            record_evaluated_candidate(step_scope, evaluation_started.elapsed());
            continue;
        }
        let candidate_strength = match candidate {
            crate::heuristic::selector::move_selector::MoveCandidateRef::Borrowed(mov) => {
                strength(mov, step_scope.score_director().working_solution())
            }
            crate::heuristic::selector::move_selector::MoveCandidateRef::Sequential(_) => {
                unreachable!("construction candidates are concrete atomic moves")
            }
        };
        record_evaluated_candidate(step_scope, evaluation_started.elapsed());
        mark_candidate_disposition(
            placement,
            candidate_id,
            CandidateTraceDisposition::Evaluated,
            step_scope,
        );
        if retained.is_none_or(|(_, best)| prefer(candidate_strength, best)) {
            if let Some((replaced, _)) = retained.replace((candidate_id, candidate_strength)) {
                release(
                    placement,
                    replaced,
                    CandidateTraceDisposition::ForagerIgnored,
                    step_scope,
                );
            }
        } else {
            release(
                placement,
                candidate_id,
                CandidateTraceDisposition::ForagerIgnored,
                step_scope,
            );
        }
    }

    let Some((best_id, _)) = retained else {
        return Some(ConstructionChoice::KeepCurrent);
    };
    if !keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
        return Some(ConstructionChoice::Select(best_id));
    }

    let baseline_score = step_scope.calculate_score();
    let candidate = placement
        .candidates()
        .candidate(best_id)
        .expect("retained construction candidate must remain live");
    let score = evaluate_trial_move(step_scope.score_director_mut(), &candidate);
    drop(candidate);
    placement.record_candidate_score(best_id, score);
    step_scope.phase_scope_mut().record_score_calculation();
    if score > baseline_score {
        Some(ConstructionChoice::Select(best_id))
    } else {
        release(
            placement,
            best_id,
            CandidateTraceDisposition::ForagerIgnored,
            step_scope,
        );
        Some(ConstructionChoice::KeepCurrent)
    }
}

pub(in super::super) fn select_weakest_fit_index<S, D, BestCb, M, C>(
    forager: &WeakestFitForager<S, M>,
    placement: &mut Placement<S, M, C>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    select_strength_index(
        placement,
        construction_obligation,
        step_scope,
        |mov, solution| forager.strength(mov, solution),
        |candidate, best| candidate < best,
    )
}

pub(in super::super) fn select_strongest_fit_index<S, D, BestCb, M, C>(
    forager: &StrongestFitForager<S, M>,
    placement: &mut Placement<S, M, C>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    select_strength_index(
        placement,
        construction_obligation,
        step_scope,
        |mov, solution| forager.strength(mov, solution),
        |candidate, best| candidate > best,
    )
}
