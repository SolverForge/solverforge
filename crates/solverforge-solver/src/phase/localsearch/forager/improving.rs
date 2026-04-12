use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::r#move::Move;

use super::LocalSearchForager;

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
    accepted_moves: Vec<(usize, S::Score)>,
    found_best_improving: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    pub fn new() -> Self {
        Self {
            best_score: S::Score::zero(),
            accepted_moves: Vec::new(),
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
        Self::new()
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
            accepted_moves: Vec::new(),
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
    fn step_started(&mut self, best_score: S::Score, _last_step_score: S::Score) {
        self.best_score = best_score;
        self.accepted_moves.clear();
        self.found_best_improving = false;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if score > self.best_score {
            self.accepted_moves.clear();
            self.accepted_moves.push((index, score));
            self.found_best_improving = true;
        } else if !self.found_best_improving {
            self.accepted_moves.push((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.found_best_improving
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
        }
        if self.found_best_improving {
            return self.accepted_moves.pop();
        }

        let mut best_idx = 0;
        let mut best_score = self.accepted_moves[0].1;
        for (i, &(_, score)) in self.accepted_moves.iter().enumerate().skip(1) {
            if score > best_score {
                best_idx = i;
                best_score = score;
            }
        }
        Some(self.accepted_moves.swap_remove(best_idx))
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
    accepted_moves: Vec<(usize, S::Score)>,
    found_last_step_improving: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    pub fn new() -> Self {
        Self {
            last_step_score: S::Score::zero(),
            accepted_moves: Vec::new(),
            found_last_step_improving: false,
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Debug for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstLastStepScoreImprovingForager")
            .field("found_last_step_improving", &self.found_last_step_improving)
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
            accepted_moves: Vec::new(),
            found_last_step_improving: false,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, last_step_score: S::Score) {
        self.last_step_score = last_step_score;
        self.accepted_moves.clear();
        self.found_last_step_improving = false;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if score > self.last_step_score {
            self.accepted_moves.clear();
            self.accepted_moves.push((index, score));
            self.found_last_step_improving = true;
        } else if !self.found_last_step_improving {
            self.accepted_moves.push((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.found_last_step_improving
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
        }
        if self.found_last_step_improving {
            return self.accepted_moves.pop();
        }

        let mut best_idx = 0;
        let mut best_score = self.accepted_moves[0].1;
        for (i, &(_, score)) in self.accepted_moves.iter().enumerate().skip(1) {
            if score > best_score {
                best_idx = i;
                best_score = score;
            }
        }
        Some(self.accepted_moves.swap_remove(best_idx))
    }
}
