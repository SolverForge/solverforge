//! Forager builder and `AnyForager` enum.

use solverforge_config::{ForagerConfig, PickEarlyType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use std::fmt::Debug;

use crate::heuristic::r#move::Move;
use crate::phase::localsearch::{
    AcceptedCountForager, BestScoreForager, FirstAcceptedForager, FirstBestScoreImprovingForager,
    FirstLastStepScoreImprovingForager, LocalSearchForager,
};

/// A concrete enum over all built-in forager types.
///
/// Returned by [`ForagerBuilder::build`] to avoid `Box<dyn LocalSearchForager<S, M>>`.
/// Dispatches to the inner forager via `match` — fully monomorphized.
#[allow(clippy::large_enum_variant)]
pub enum AnyForager<S: PlanningSolution> {
    /// Collects up to `N` accepted moves, picks the best.
    AcceptedCount(AcceptedCountForager<S>),
    /// Picks the first accepted move.
    FirstAccepted(FirstAcceptedForager<S>),
    /// Evaluates all moves, picks the best score overall.
    BestScore(BestScoreForager<S>),
    /// Picks the first move that improves on the all-time best.
    BestScoreImproving(FirstBestScoreImprovingForager<S>),
    /// Picks the first move that improves on the last step's score.
    LastStepScoreImproving(FirstLastStepScoreImprovingForager<S>),
}

impl<S: PlanningSolution> Debug for AnyForager<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptedCount(a) => write!(f, "AnyForager::AcceptedCount({a:?})"),
            Self::FirstAccepted(a) => write!(f, "AnyForager::FirstAccepted({a:?})"),
            Self::BestScore(a) => write!(f, "AnyForager::BestScore({a:?})"),
            Self::BestScoreImproving(a) => write!(f, "AnyForager::BestScoreImproving({a:?})"),
            Self::LastStepScoreImproving(a) => {
                write!(f, "AnyForager::LastStepScoreImproving({a:?})")
            }
        }
    }
}

impl<S: PlanningSolution, M: Move<S>> LocalSearchForager<S, M> for AnyForager<S>
where
    S::Score: Score,
{
    fn step_started(&mut self, best_score: S::Score, last_step_score: S::Score) {
        match self {
            Self::AcceptedCount(f) => {
                LocalSearchForager::<S, M>::step_started(f, best_score, last_step_score)
            }
            Self::FirstAccepted(f) => {
                LocalSearchForager::<S, M>::step_started(f, best_score, last_step_score)
            }
            Self::BestScore(f) => {
                LocalSearchForager::<S, M>::step_started(f, best_score, last_step_score)
            }
            Self::BestScoreImproving(f) => {
                LocalSearchForager::<S, M>::step_started(f, best_score, last_step_score)
            }
            Self::LastStepScoreImproving(f) => {
                LocalSearchForager::<S, M>::step_started(f, best_score, last_step_score)
            }
        }
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        match self {
            Self::AcceptedCount(f) => LocalSearchForager::<S, M>::add_move_index(f, index, score),
            Self::FirstAccepted(f) => LocalSearchForager::<S, M>::add_move_index(f, index, score),
            Self::BestScore(f) => LocalSearchForager::<S, M>::add_move_index(f, index, score),
            Self::BestScoreImproving(f) => {
                LocalSearchForager::<S, M>::add_move_index(f, index, score)
            }
            Self::LastStepScoreImproving(f) => {
                LocalSearchForager::<S, M>::add_move_index(f, index, score)
            }
        }
    }

    fn is_quit_early(&self) -> bool {
        match self {
            Self::AcceptedCount(f) => LocalSearchForager::<S, M>::is_quit_early(f),
            Self::FirstAccepted(f) => LocalSearchForager::<S, M>::is_quit_early(f),
            Self::BestScore(f) => LocalSearchForager::<S, M>::is_quit_early(f),
            Self::BestScoreImproving(f) => LocalSearchForager::<S, M>::is_quit_early(f),
            Self::LastStepScoreImproving(f) => LocalSearchForager::<S, M>::is_quit_early(f),
        }
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        match self {
            Self::AcceptedCount(f) => LocalSearchForager::<S, M>::pick_move_index(f),
            Self::FirstAccepted(f) => LocalSearchForager::<S, M>::pick_move_index(f),
            Self::BestScore(f) => LocalSearchForager::<S, M>::pick_move_index(f),
            Self::BestScoreImproving(f) => LocalSearchForager::<S, M>::pick_move_index(f),
            Self::LastStepScoreImproving(f) => LocalSearchForager::<S, M>::pick_move_index(f),
        }
    }
}

/// Builder for constructing foragers from configuration.
pub struct ForagerBuilder;

impl ForagerBuilder {
    /// Builds a concrete [`AnyForager`] from configuration.
    ///
    /// - `accepted_count_limit = Some(n)` without pick_early → `AcceptedCount(n)`
    /// - `pick_early_type = FirstBestScoreImproving` → `BestScoreImproving`
    /// - `pick_early_type = FirstLastStepScoreImproving` → `LastStepScoreImproving`
    /// - No config → `AcceptedCount(1)` (default)
    pub fn build<S: PlanningSolution>(config: Option<&ForagerConfig>) -> AnyForager<S>
    where
        S::Score: Score,
    {
        let Some(cfg) = config else {
            return AnyForager::AcceptedCount(AcceptedCountForager::new(1));
        };

        match cfg.pick_early_type {
            Some(PickEarlyType::FirstBestScoreImproving) => {
                AnyForager::BestScoreImproving(FirstBestScoreImprovingForager::new())
            }
            Some(PickEarlyType::FirstLastStepScoreImproving) => {
                AnyForager::LastStepScoreImproving(FirstLastStepScoreImprovingForager::new())
            }
            _ => {
                let limit = cfg.accepted_count_limit.unwrap_or(1).max(1);
                AnyForager::AcceptedCount(AcceptedCountForager::new(limit))
            }
        }
    }
}
