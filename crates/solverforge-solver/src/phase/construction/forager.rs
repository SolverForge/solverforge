//! Foragers for construction heuristic move selection
//!
//! Foragers determine which move to select from the candidates
//! generated for each entity placement.
//!
//! # Zero-Erasure Design
//!
//! Foragers return indices into the placement's move Vec, not cloned moves.
//! The caller takes ownership via the index.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::heuristic::r#move::Move;

use super::Placement;

/// Trait for selecting a move during construction.
///
/// Foragers evaluate candidate moves and pick one based on their strategy.
/// Returns the index of the selected move, not a cloned move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait ConstructionForager<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Picks a move index from the placement's candidates.
    ///
    /// Returns None if no suitable move is found.
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize>;
}

/// First Fit forager - picks the first feasible move.
///
/// This is the fastest forager but may not produce optimal results.
/// It simply takes the first move that can be executed.
pub struct FirstFitForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for FirstFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for FirstFitForager<S, M> {}

impl<S, M> Default for FirstFitForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for FirstFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFitForager").finish()
    }
}

impl<S, M> FirstFitForager<S, M> {
    /// Creates a new First Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        for (idx, m) in placement.moves.iter().enumerate() {
            if m.is_doable(score_director) {
                return Some(idx);
            }
        }
        None
    }
}

/// Best Fit forager - evaluates all moves and picks the best.
///
/// This forager evaluates each candidate move by executing it,
/// calculating the score, and undoing it. The move with the best
/// score is selected.
pub struct BestFitForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for BestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for BestFitForager<S, M> {}

impl<S, M> Default for BestFitForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for BestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestFitForager").finish()
    }
}

impl<S, M> BestFitForager<S, M> {
    /// Creates a new Best Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for BestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        let mut best_idx: Option<usize> = None;
        let mut best_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Use RecordingScoreDirector for automatic undo
            let score = {
                let mut recording = RecordingScoreDirector::new(score_director);

                // Execute move
                m.do_move(&mut recording);

                // Evaluate
                let score = recording.calculate_score();

                // Undo move
                recording.undo_changes();

                score
            };

            // Check if this is the best so far
            let is_better = match &best_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                best_idx = Some(idx);
                best_score = Some(score);
            }
        }

        best_idx
    }
}

/// First Feasible forager - picks the first move that results in a feasible score.
///
/// This forager evaluates moves until it finds one that produces a feasible
/// (non-negative hard score) solution.
pub struct FirstFeasibleForager<S, M> {
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M> Clone for FirstFeasibleForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for FirstFeasibleForager<S, M> {}

impl<S, M> Default for FirstFeasibleForager<S, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> Debug for FirstFeasibleForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFeasibleForager").finish()
    }
}

impl<S, M> FirstFeasibleForager<S, M> {
    /// Creates a new First Feasible forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFeasibleForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        let mut fallback_idx: Option<usize> = None;
        let mut fallback_score: Option<S::Score> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            // Use RecordingScoreDirector for automatic undo
            let score = {
                let mut recording = RecordingScoreDirector::new(score_director);

                // Execute move
                m.do_move(&mut recording);

                // Evaluate
                let score = recording.calculate_score();

                // If feasible, return this move index immediately
                if score.is_feasible() {
                    recording.undo_changes();
                    return Some(idx);
                }

                // Undo move
                recording.undo_changes();

                score
            };

            // Track best infeasible as fallback
            let is_better = match &fallback_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                fallback_idx = Some(idx);
                fallback_score = Some(score);
            }
        }

        // No feasible move found, return best infeasible
        fallback_idx
    }
}

/// Weakest Fit forager - picks the move with the lowest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the minimum strength. This is useful for
/// assigning the "weakest" or least constraining values first.
pub struct WeakestFitForager<S, M> {
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Clone for WeakestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for WeakestFitForager<S, M> {}

impl<S, M> Debug for WeakestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakestFitForager").finish()
    }
}

impl<S, M> WeakestFitForager<S, M> {
    /// Creates a new Weakest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the minimum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        let mut best_idx: Option<usize> = None;
        let mut min_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_weaker = match min_strength {
                None => true,
                Some(best) => strength < best,
            };

            if is_weaker {
                best_idx = Some(idx);
                min_strength = Some(strength);
            }
        }

        best_idx
    }
}

/// Strongest Fit forager - picks the move with the highest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the maximum strength. This is useful for
/// assigning the "strongest" or most constraining values first.
pub struct StrongestFitForager<S, M> {
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Clone for StrongestFitForager<S, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, M> Copy for StrongestFitForager<S, M> {}

impl<S, M> Debug for StrongestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrongestFitForager").finish()
    }
}

impl<S, M> StrongestFitForager<S, M> {
    /// Creates a new Strongest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the maximum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move_index<D: ScoreDirector<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> Option<usize> {
        let mut best_idx: Option<usize> = None;
        let mut max_strength: Option<i64> = None;

        for (idx, m) in placement.moves.iter().enumerate() {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_stronger = match max_strength {
                None => true,
                Some(best) => strength > best,
            };

            if is_stronger {
                best_idx = Some(idx);
                max_strength = Some(strength);
            }
        }

        best_idx
    }
}

#[cfg(test)]
#[path = "forager_tests.rs"]
mod tests;
