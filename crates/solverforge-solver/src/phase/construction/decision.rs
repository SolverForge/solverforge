use std::cmp::Ordering;

use crate::heuristic::selector::move_selector::CandidateId;
use solverforge_core::score::Score;

use super::ConstructionChoice;
use solverforge_config::ConstructionObligation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScoredChoiceTracker<ScoreT>
where
    ScoreT: Score,
{
    best_candidate: Option<(CandidateId, ScoreT)>,
}

impl<ScoreT> Default for ScoredChoiceTracker<ScoreT>
where
    ScoreT: Score,
{
    fn default() -> Self {
        Self {
            best_candidate: None,
        }
    }
}

impl<ScoreT> ScoredChoiceTracker<ScoreT>
where
    ScoreT: Score,
{
    pub(crate) fn consider(&mut self, idx: CandidateId, score: ScoreT) {
        let should_replace = match self.best_candidate {
            None => true,
            Some((_, best_score)) => score > best_score,
        };

        if should_replace {
            self.best_candidate = Some((idx, score));
        }
    }
}

pub(crate) fn select_first_fit(first_doable_idx: Option<CandidateId>) -> ConstructionChoice {
    first_doable_idx
        .map(ConstructionChoice::Select)
        .unwrap_or(ConstructionChoice::KeepCurrent)
}

pub(crate) fn is_first_fit_improvement<ScoreT>(
    baseline_score: ScoreT,
    candidate_score: ScoreT,
) -> bool
where
    ScoreT: Score,
{
    candidate_score > baseline_score
}

pub(crate) fn select_best_fit<ScoreT>(
    tracker: ScoredChoiceTracker<ScoreT>,
    baseline_score: Option<ScoreT>,
) -> ConstructionChoice
where
    ScoreT: Score,
{
    resolve_scored_choice(tracker, baseline_score)
}

pub(crate) fn resolve_scored_choice<ScoreT>(
    tracker: ScoredChoiceTracker<ScoreT>,
    baseline_score: Option<ScoreT>,
) -> ConstructionChoice
where
    ScoreT: Score,
{
    let Some((idx, candidate_score)) = tracker.best_candidate else {
        return ConstructionChoice::KeepCurrent;
    };

    let Some(baseline_score) = baseline_score else {
        return ConstructionChoice::Select(idx);
    };

    // The current value stays only when it beats every candidate.
    match baseline_score.cmp(&candidate_score) {
        Ordering::Greater => ConstructionChoice::KeepCurrent,
        Ordering::Less | Ordering::Equal => ConstructionChoice::Select(idx),
    }
}

pub(crate) fn keep_current_allowed(
    keep_current_legal: bool,
    construction_obligation: ConstructionObligation,
) -> bool {
    keep_current_legal
        && matches!(
            construction_obligation,
            ConstructionObligation::PreserveUnassigned
        )
}
