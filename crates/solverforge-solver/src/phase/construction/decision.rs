use std::cmp::Ordering;

use solverforge_core::score::Score;

use super::ConstructionChoice;
use solverforge_config::ConstructionObligation;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaselinePolicy {
    NeverPreferCurrent,
    KeepIfAlreadyFeasible,
    KeepOnlyIfStrictlyBetterThanAllMoves,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EqualScorePolicy {
    PreferMove,
    PreferCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScoredChoiceTracker<ScoreT>
where
    ScoreT: Score,
{
    best_candidate: Option<(usize, ScoreT)>,
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
    pub(crate) fn consider(&mut self, idx: usize, score: ScoreT) {
        let should_replace = match self.best_candidate {
            None => true,
            Some((_, best_score)) => score > best_score,
        };

        if should_replace {
            self.best_candidate = Some((idx, score));
        }
    }
}

pub(crate) fn select_first_fit(first_doable_idx: Option<usize>) -> ConstructionChoice {
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
    resolve_scored_choice(
        tracker,
        baseline_score,
        BaselinePolicy::KeepOnlyIfStrictlyBetterThanAllMoves,
        EqualScorePolicy::PreferMove,
    )
}

pub(crate) fn select_first_feasible<ScoreT>(
    first_feasible_idx: Option<usize>,
    fallback_tracker: ScoredChoiceTracker<ScoreT>,
    baseline_score: Option<ScoreT>,
) -> ConstructionChoice
where
    ScoreT: Score,
{
    if should_keep_current_immediately(baseline_score, BaselinePolicy::KeepIfAlreadyFeasible) {
        return ConstructionChoice::KeepCurrent;
    }

    if let Some(idx) = first_feasible_idx {
        return ConstructionChoice::Select(idx);
    }

    resolve_scored_choice(
        fallback_tracker,
        baseline_score,
        BaselinePolicy::KeepOnlyIfStrictlyBetterThanAllMoves,
        EqualScorePolicy::PreferMove,
    )
}

pub(crate) fn should_keep_current_immediately<ScoreT>(
    baseline_score: Option<ScoreT>,
    baseline_policy: BaselinePolicy,
) -> bool
where
    ScoreT: Score,
{
    matches!(baseline_policy, BaselinePolicy::KeepIfAlreadyFeasible)
        && baseline_score.is_some_and(|score| score.is_feasible())
}

pub(crate) fn resolve_scored_choice<ScoreT>(
    tracker: ScoredChoiceTracker<ScoreT>,
    baseline_score: Option<ScoreT>,
    baseline_policy: BaselinePolicy,
    equal_score_policy: EqualScorePolicy,
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

    match baseline_policy {
        BaselinePolicy::NeverPreferCurrent => ConstructionChoice::Select(idx),
        BaselinePolicy::KeepIfAlreadyFeasible if baseline_score.is_feasible() => {
            ConstructionChoice::KeepCurrent
        }
        BaselinePolicy::KeepIfAlreadyFeasible
        | BaselinePolicy::KeepOnlyIfStrictlyBetterThanAllMoves => {
            match baseline_score.cmp(&candidate_score) {
                Ordering::Greater => ConstructionChoice::KeepCurrent,
                Ordering::Less => ConstructionChoice::Select(idx),
                Ordering::Equal => match equal_score_policy {
                    EqualScorePolicy::PreferMove => ConstructionChoice::Select(idx),
                    EqualScorePolicy::PreferCurrent => ConstructionChoice::KeepCurrent,
                },
            }
        }
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
