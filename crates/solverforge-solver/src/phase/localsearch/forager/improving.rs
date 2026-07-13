use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::CandidateId;

use super::{BestCandidate, ForagerDecision, LocalSearchForager};

/// A forager that picks the first accepted move that improves on the best score ever seen.
///
/// Once a move with a score strictly better than the all-time best is found, the
/// forager quits immediately and selects that move. If no such move exists, it falls
/// back to the best among all accepted moves.
pub struct FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    best_score: S::Score,
    best_move: BestCandidate<S>,
    found_best_improving: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    pub fn new(random_ties: bool) -> Self {
        Self {
            best_score: S::Score::zero(),
            best_move: BestCandidate::new(random_ties),
            found_best_improving: false,
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new(true)
    }
}

impl<S> Debug for FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstBestScoreImprovingForager")
            .field("found_best_improving", &self.found_best_improving)
            .finish()
    }
}

impl<S> Clone for FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            best_score: self.best_score,
            best_move: BestCandidate::new(self.best_move.random_ties()),
            found_best_improving: false,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, best_score: S::Score, _last_step_score: S::Score, step_seed: u64) {
        self.best_score = best_score;
        self.best_move.reset(step_seed);
        self.found_best_improving = false;
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        if score > self.best_score {
            self.found_best_improving = true;
            return self.best_move.replace(index, score);
        }
        if self.found_best_improving {
            return ForagerDecision::Release;
        }
        self.best_move.consider(index, score)
    }

    fn is_quit_early(&self) -> bool {
        self.found_best_improving
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.best_move.take()
    }
}

/// A forager that picks the first accepted move that improves on the last step's score.
///
/// Once a move with a score strictly better than the previous step is found, the
/// forager quits immediately and selects that move. If no such move exists, it falls
/// back to the best among all accepted moves.
pub struct FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    last_step_score: S::Score,
    accepted_count: usize,
    best_move: BestCandidate<S>,
    found_last_step_improving: bool,
    accepted_count_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    pub fn new(random_ties: bool) -> Self {
        Self {
            last_step_score: S::Score::zero(),
            accepted_count: 0,
            best_move: BestCandidate::new(random_ties),
            found_last_step_improving: false,
            accepted_count_limit: None,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_accepted_count_limit(mut self, accepted_count_limit: usize) -> Self {
        assert!(
            accepted_count_limit > 0,
            "FirstLastStepScoreImprovingForager: accepted_count_limit must be > 0, got 0"
        );
        self.accepted_count_limit = Some(accepted_count_limit);
        self
    }
}

impl<S> Default for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new(true)
    }
}

impl<S> Debug for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstLastStepScoreImprovingForager")
            .field("found_last_step_improving", &self.found_last_step_improving)
            .field("accepted_count_limit", &self.accepted_count_limit)
            .finish()
    }
}

impl<S> Clone for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            last_step_score: self.last_step_score,
            accepted_count: 0,
            best_move: BestCandidate::new(self.best_move.random_ties()),
            found_last_step_improving: false,
            accepted_count_limit: self.accepted_count_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, last_step_score: S::Score, step_seed: u64) {
        self.last_step_score = last_step_score;
        self.accepted_count = 0;
        self.best_move.reset(step_seed);
        self.found_last_step_improving = false;
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        if self.found_last_step_improving
            || self
                .accepted_count_limit
                .is_some_and(|limit| self.accepted_count >= limit)
        {
            return ForagerDecision::Release;
        }
        self.accepted_count += 1;
        if score > self.last_step_score {
            self.found_last_step_improving = true;
            return self.best_move.replace(index, score);
        }
        self.best_move.consider(index, score)
    }

    fn is_quit_early(&self) -> bool {
        self.found_last_step_improving
            || self
                .accepted_count_limit
                .is_some_and(|limit| self.accepted_count >= limit)
    }

    fn accepted_count_limit(&self) -> Option<usize> {
        self.accepted_count_limit
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.best_move.take()
    }
}
