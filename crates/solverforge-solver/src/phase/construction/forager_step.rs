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
use crate::phase::control::should_interrupt_evaluation;
use crate::scope::{ProgressCallback, StepScope};

pub(super) fn select_first_fit_index<S, D, BestCb, M>(
    placement: &Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let mut first_doable = None;
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return None;
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

    Some(select_first_fit(first_doable))
}

pub(super) fn select_best_fit_index<S, D, BestCb, M>(
    placement: &Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());
    let mut tracker = ScoredChoiceTracker::default();

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return None;
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

    Some(select_best_fit(tracker, baseline_score))
}

pub(super) fn select_first_feasible_index<S, D, BestCb, M>(
    placement: &Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let baseline_score =
        keep_current_allowed(placement.keep_current_legal(), construction_obligation)
            .then(|| step_scope.calculate_score());

    let mut fallback_tracker = ScoredChoiceTracker::default();
    let mut first_feasible = None;

    for (idx, m) in placement.moves.iter().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, idx) {
            return None;
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

    Some(select_first_feasible(
        first_feasible,
        fallback_tracker,
        baseline_score,
    ))
}

pub(super) fn select_weakest_fit_index<S, D, BestCb, M>(
    forager: &WeakestFitForager<S, M>,
    placement: &Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let mut best_idx = None;
    let mut min_strength = None;

    for (evaluated, (idx, m)) in placement.moves.iter().enumerate().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, evaluated) {
            return None;
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
        return Some(ConstructionChoice::KeepCurrent);
    };

    if !keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
        return Some(ConstructionChoice::Select(best_idx));
    }

    let baseline_score = step_scope.calculate_score();
    let score = evaluate_trial_move(step_scope.score_director_mut(), &placement.moves[best_idx]);
    step_scope.phase_scope_mut().record_score_calculation();

    Some(if score > baseline_score {
        ConstructionChoice::Select(best_idx)
    } else {
        ConstructionChoice::KeepCurrent
    })
}

pub(super) fn select_strongest_fit_index<S, D, BestCb, M>(
    forager: &StrongestFitForager<S, M>,
    placement: &Placement<S, M>,
    construction_obligation: ConstructionObligation,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
) -> Option<ConstructionChoice>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
{
    let mut best_idx = None;
    let mut max_strength = None;

    for (evaluated, (idx, m)) in placement.moves.iter().enumerate().enumerate() {
        let evaluation_started = Instant::now();
        if should_interrupt_evaluation(step_scope, evaluated) {
            return None;
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
        return Some(ConstructionChoice::KeepCurrent);
    };

    if !keep_current_allowed(placement.keep_current_legal(), construction_obligation) {
        return Some(ConstructionChoice::Select(best_idx));
    }

    let baseline_score = step_scope.calculate_score();
    let score = evaluate_trial_move(step_scope.score_director_mut(), &placement.moves[best_idx]);
    step_scope.phase_scope_mut().record_score_calculation();

    Some(if score > baseline_score {
        ConstructionChoice::Select(best_idx)
    } else {
        ConstructionChoice::KeepCurrent
    })
}
