use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use super::decision::{
    is_first_fit_improvement, select_best_fit, select_first_feasible, select_first_fit,
    ScoredChoiceTracker,
};
use super::evaluation::evaluate_trial_move;
use super::forager::{
    BestFitForager, ConstructionChoice, ConstructionForager, FirstFeasibleForager, FirstFitForager,
    StrongestFitForager, WeakestFitForager,
};
use super::forager_step::{
    select_best_fit_index, select_first_feasible_index, select_first_fit_index,
    select_strongest_fit_index, select_weakest_fit_index,
};
use super::Placement;
use crate::heuristic::r#move::Move;
use crate::scope::{ProgressCallback, StepScope};

impl<S, M> ConstructionForager<S, M> for FirstFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice {
        let mut first_doable = None;
        let baseline_score = placement
            .keep_current_legal()
            .then(|| score_director.calculate_score());

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            if let Some(baseline_score) = baseline_score {
                let candidate_score = evaluate_trial_move(score_director, m);
                if is_first_fit_improvement(baseline_score, candidate_score) {
                    first_doable = Some(idx);
                    break;
                }
            } else {
                first_doable = Some(idx);
                break;
            }
        }

        select_first_fit(first_doable)
    }

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        select_first_fit_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for BestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice {
        let baseline_score = placement
            .keep_current_legal()
            .then(|| score_director.calculate_score());
        let mut tracker = ScoredChoiceTracker::default();

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let score = evaluate_trial_move(score_director, m);

            tracker.consider(idx, score);
        }

        select_best_fit(tracker, baseline_score)
    }

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        select_best_fit_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFeasibleForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice {
        let baseline_score = placement
            .keep_current_legal()
            .then(|| score_director.calculate_score());

        let mut fallback_tracker = ScoredChoiceTracker::default();
        let mut first_feasible = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let score = evaluate_trial_move(score_director, m);

            if score.is_feasible() {
                first_feasible = Some(idx);
                break;
            }

            fallback_tracker.consider(idx, score);
        }

        select_first_feasible(first_feasible, fallback_tracker, baseline_score)
    }

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        select_first_feasible_index(placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice {
        let mut best_idx: Option<usize> = None;
        let mut min_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = self.strength(m, score_director.working_solution());

            let is_weaker = match min_strength {
                None => true,
                Some(best) => strength < best,
            };

            if is_weaker {
                best_idx = Some(idx);
                min_strength = Some(strength);
            }
        }

        let Some(best_idx) = best_idx else {
            return ConstructionChoice::KeepCurrent;
        };

        if !placement.keep_current_legal() {
            return ConstructionChoice::Select(best_idx);
        }

        let baseline_score = score_director.calculate_score();
        let candidate_score = evaluate_trial_move(score_director, &placement.moves[best_idx]);
        if candidate_score > baseline_score {
            ConstructionChoice::Select(best_idx)
        } else {
            ConstructionChoice::KeepCurrent
        }
    }

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        select_weakest_fit_index(self, placement, construction_obligation, step_scope)
    }
}

impl<S, M> ConstructionForager<S, M> for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice {
        let mut best_idx: Option<usize> = None;
        let mut max_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = self.strength(m, score_director.working_solution());

            let is_stronger = match max_strength {
                None => true,
                Some(best) => strength > best,
            };

            if is_stronger {
                best_idx = Some(idx);
                max_strength = Some(strength);
            }
        }

        let Some(best_idx) = best_idx else {
            return ConstructionChoice::KeepCurrent;
        };

        if !placement.keep_current_legal() {
            return ConstructionChoice::Select(best_idx);
        }

        let baseline_score = score_director.calculate_score();
        let candidate_score = evaluate_trial_move(score_director, &placement.moves[best_idx]);
        if candidate_score > baseline_score {
            ConstructionChoice::Select(best_idx)
        } else {
            ConstructionChoice::KeepCurrent
        }
    }

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        select_strongest_fit_index(self, placement, construction_obligation, step_scope)
    }
}
