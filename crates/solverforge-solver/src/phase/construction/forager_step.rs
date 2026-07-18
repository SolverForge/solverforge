use std::time::Instant;

use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use super::decision::{
    is_first_fit_improvement, keep_current_allowed, select_best_fit, select_first_feasible,
    select_first_fit, ScoredChoiceTracker,
};
use super::evaluation::evaluate_trial_move;
use super::{ConstructionChoice, Placement, StrongestFitForager, WeakestFitForager};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, MoveCursor};
use crate::phase::control::{
    settle_construction_interrupt, should_interrupt_before_candidate,
    should_interrupt_before_evaluation, StepInterrupt,
};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

fn next_candidate<S, D, BestCb, M, C>(
    placement: &mut Placement<S, M, C>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<CandidateId>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    let candidate_id = placement
        .candidates_mut()
        .next_candidate_with_control(&mut || should_interrupt_before_candidate(step_scope))?;
    let construction_target = CandidateTraceConstructionTarget {
        descriptor_index: placement.entity_ref.descriptor_index,
        entity_index: placement.entity_ref.entity_index,
    };
    let candidate = placement
        .candidates()
        .candidate(candidate_id)
        .expect("construction candidate id must remain borrowable after pull");
    let trace_token = step_scope.phase_scope_mut().record_candidate_pull(
        CandidateTraceSource::Construction,
        None,
        candidate_id.index(),
        Some(construction_target),
        &candidate,
    );
    if let Some(token) = trace_token {
        placement.record_candidate_trace_token(candidate_id, token);
    }
    step_scope
        .phase_scope_mut()
        .record_generated_move(std::time::Duration::ZERO);
    Some(candidate_id)
}

fn mark_candidate_disposition<S, D, BestCb, M, C>(
    placement: &Placement<S, M, C>,
    candidate_id: CandidateId,
    disposition: CandidateTraceDisposition,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    if let Some(token) = placement.candidate_trace_token(candidate_id) {
        step_scope
            .phase_scope_mut()
            .record_candidate_trace_disposition(token, disposition);
    }
}

fn release<S, D, BestCb, M, C>(
    placement: &mut Placement<S, M, C>,
    candidate_id: CandidateId,
    disposition: CandidateTraceDisposition,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    mark_candidate_disposition(placement, candidate_id, disposition, step_scope);
    assert!(placement.candidates_mut().release_candidate(candidate_id));
}

fn mark_retained_ignored<S, D, BestCb, M, C>(
    placement: &Placement<S, M, C>,
    retained: Option<CandidateId>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    if let Some(candidate_id) = retained {
        mark_candidate_disposition(
            placement,
            candidate_id,
            CandidateTraceDisposition::ForagerIgnored,
            step_scope,
        );
    }
}

fn evaluation_should_terminate<S, D, BestCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    if step_scope.progress_polling_required() {
        step_scope.phase_scope_mut().report_progress_if_due();
    }
    should_interrupt_before_evaluation(step_scope)
        && matches!(
            settle_construction_interrupt(step_scope),
            StepInterrupt::TerminatePhase
        )
}

#[allow(clippy::drop_non_drop)]
pub(super) fn select_first_fit_index<S, D, BestCb, M, C>(
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
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());
    loop {
        if evaluation_should_terminate(step_scope) {
            return None;
        }
        let Some(candidate_id) = next_candidate(placement, step_scope) else {
            if should_interrupt_before_candidate(step_scope) {
                return None;
            }
            break;
        };
        let evaluation_started = Instant::now();
        let candidate = placement
            .candidates()
            .candidate(candidate_id)
            .expect("construction candidate must remain live");
        if !candidate.is_doable(step_scope.score_director()) {
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
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }

        let selected = if let Some(baseline_score) = baseline_score {
            let score = evaluate_trial_move(step_scope.score_director_mut(), &candidate);
            drop(candidate);
            placement.record_candidate_score(candidate_id, score);
            step_scope.phase_scope_mut().record_score_calculation();
            is_first_fit_improvement(baseline_score, score)
        } else {
            drop(candidate);
            true
        };
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
        mark_candidate_disposition(
            placement,
            candidate_id,
            CandidateTraceDisposition::Evaluated,
            step_scope,
        );
        if selected {
            return Some(select_first_fit(Some(candidate_id)));
        }
        release(
            placement,
            candidate_id,
            CandidateTraceDisposition::ForagerIgnored,
            step_scope,
        );
    }

    Some(ConstructionChoice::KeepCurrent)
}

#[allow(clippy::drop_non_drop)]
pub(super) fn select_best_fit_index<S, D, BestCb, M, C>(
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
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());
    let mut tracker = ScoredChoiceTracker::default();
    let mut retained: Option<(CandidateId, S::Score)> = None;
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
        if !candidate.is_doable(step_scope.score_director()) {
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
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }
        let score = evaluate_trial_move(step_scope.score_director_mut(), &candidate);
        drop(candidate);
        placement.record_candidate_score(candidate_id, score);
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
        mark_candidate_disposition(
            placement,
            candidate_id,
            CandidateTraceDisposition::Evaluated,
            step_scope,
        );

        if retained.is_none_or(|(_, best_score)| score > best_score) {
            if let Some((replaced, _)) = retained.replace((candidate_id, score)) {
                release(
                    placement,
                    replaced,
                    CandidateTraceDisposition::ForagerIgnored,
                    step_scope,
                );
            }
            tracker.consider(candidate_id, score);
        } else {
            release(
                placement,
                candidate_id,
                CandidateTraceDisposition::ForagerIgnored,
                step_scope,
            );
        }
    }

    let choice = select_best_fit(tracker, baseline_score);
    if matches!(choice, ConstructionChoice::KeepCurrent) {
        if let Some((retained, _)) = retained {
            release(
                placement,
                retained,
                CandidateTraceDisposition::ForagerIgnored,
                step_scope,
            );
        }
    }
    Some(choice)
}

#[allow(clippy::drop_non_drop)]
pub(super) fn select_first_feasible_index<S, D, BestCb, M, C>(
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
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());
    let mut tracker = ScoredChoiceTracker::default();
    let mut retained: Option<(CandidateId, S::Score)> = None;
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
        if !candidate.is_doable(step_scope.score_director()) {
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
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
            continue;
        }
        let score = evaluate_trial_move(step_scope.score_director_mut(), &candidate);
        drop(candidate);
        placement.record_candidate_score(candidate_id, score);
        step_scope.phase_scope_mut().record_score_calculation();
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
        mark_candidate_disposition(
            placement,
            candidate_id,
            CandidateTraceDisposition::Evaluated,
            step_scope,
        );
        if score.is_feasible() {
            if let Some((fallback, _)) = retained {
                release(
                    placement,
                    fallback,
                    CandidateTraceDisposition::ForagerIgnored,
                    step_scope,
                );
            }
            let choice = select_first_feasible(Some(candidate_id), tracker, baseline_score);
            match choice {
                ConstructionChoice::Select(_) => {}
                ConstructionChoice::KeepCurrent => {
                    release(
                        placement,
                        candidate_id,
                        CandidateTraceDisposition::ForagerIgnored,
                        step_scope,
                    );
                }
            }
            return Some(choice);
        }
        if retained.is_none_or(|(_, best_score)| score > best_score) {
            if let Some((replaced, _)) = retained.replace((candidate_id, score)) {
                release(
                    placement,
                    replaced,
                    CandidateTraceDisposition::ForagerIgnored,
                    step_scope,
                );
            }
            tracker.consider(candidate_id, score);
        } else {
            release(
                placement,
                candidate_id,
                CandidateTraceDisposition::ForagerIgnored,
                step_scope,
            );
        }
    }

    let choice = select_first_feasible(None, tracker, baseline_score);
    if matches!(choice, ConstructionChoice::KeepCurrent) {
        if let Some((retained, _)) = retained {
            release(
                placement,
                retained,
                CandidateTraceDisposition::ForagerIgnored,
                step_scope,
            );
        }
    }
    Some(choice)
}

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
        if !candidate.is_doable(step_scope.score_director()) {
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
            step_scope
                .phase_scope_mut()
                .record_evaluated_move(evaluation_started.elapsed());
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
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
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

pub(super) fn select_weakest_fit_index<S, D, BestCb, M, C>(
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

pub(super) fn select_strongest_fit_index<S, D, BestCb, M, C>(
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
