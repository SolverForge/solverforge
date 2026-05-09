/* Foragers for local search move selection

Foragers collect accepted move indices during a step and select the
best one to apply. Uses index-based API for zero-clone operation.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::CandidateId;
use solverforge_core::domain::PlanningSolution;

/// Trait for collecting and selecting moves in local search.
///
/// Foragers are responsible for:
/// - Collecting accepted move indices during move evaluation
/// - Deciding when to quit evaluating early
/// - Selecting the best move index to apply
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type (for trait bounds only, moves are never stored)
pub trait LocalSearchForager<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /* Called at the start of each step to reset state.

    `best_score` is the best solution score ever seen.
    `last_step_score` is the score at the end of the previous step.
    Foragers that implement pick-early-on-improvement use these to decide
    when to stop evaluating moves.
    */
    fn step_started(&mut self, best_score: S::Score, last_step_score: S::Score);

    /* Adds an accepted move index to the forager.

    The index refers to a position in the MoveArena.
    */
    fn add_move_index(&mut self, index: CandidateId, score: S::Score);

    // Returns true if the forager has collected enough moves and
    // wants to stop evaluating more.
    fn is_quit_early(&self) -> bool;

    fn accepted_count_limit(&self) -> Option<usize> {
        None
    }

    /* Picks the best move index from those collected.

    Returns None if no moves were accepted.
    */
    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)>;
}

mod improving;

pub use improving::{FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager};

/// A forager that stops after `N` accepted moves and picks the best of them.
///
/// The limit is the step evaluation horizon, not a storage cap for a full
/// neighborhood scan. `AcceptedCountForager(1)` therefore behaves like first
/// accepted selection, while larger limits select the best candidate among the
/// first `N` accepted moves.
pub struct AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    // Number of accepted moves to collect before ending the step.
    accepted_count_limit: usize,
    // Collected move indices with their scores.
    accepted_moves: Vec<(CandidateId, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    /// Creates a new forager with the given limit.
    ///
    /// # Panics
    /// Panics if `accepted_count_limit` is 0 — a zero limit would quit before
    /// evaluating any move, silently skipping every step.
    ///
    /// # Arguments
    /// * `accepted_count_limit` - Collect this many accepted moves before quitting early
    pub fn new(accepted_count_limit: usize) -> Self {
        assert!(
            accepted_count_limit > 0,
            "AcceptedCountForager: accepted_count_limit must be > 0, got 0"
        );
        Self {
            accepted_count_limit,
            accepted_moves: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S> Clone for AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            accepted_count_limit: self.accepted_count_limit,
            accepted_moves: Vec::new(), // Fresh vec for clone
            _phantom: PhantomData,
        }
    }
}

impl<S> Debug for AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptedCountForager")
            .field("accepted_count_limit", &self.accepted_count_limit)
            .field("accepted_count", &self.accepted_moves.len())
            .finish()
    }
}

impl<S, M> LocalSearchForager<S, M> for AcceptedCountForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score) {
        self.accepted_moves.clear();
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) {
        if self.accepted_moves.len() >= self.accepted_count_limit {
            return;
        }
        self.accepted_moves.push((index, score));
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_moves.len() >= self.accepted_count_limit
    }

    fn accepted_count_limit(&self) -> Option<usize> {
        Some(self.accepted_count_limit)
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
        }

        // Find the best move (highest score)
        let mut best_idx = 0;
        let mut best_score = self.accepted_moves[0].1;

        for (i, &(_, score)) in self.accepted_moves.iter().enumerate().skip(1) {
            if score > best_score {
                best_idx = i;
                best_score = score;
            }
        }

        // Return the best move index
        Some(self.accepted_moves.swap_remove(best_idx))
    }
}

/// A forager that picks the first accepted move.
///
/// This is the simplest forager - it quits after the first accepted move.
pub struct FirstAcceptedForager<S>
where
    S: PlanningSolution,
{
    // The first accepted move index.
    accepted_move: Option<(CandidateId, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for FirstAcceptedForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstAcceptedForager")
            .field("has_move", &self.accepted_move.is_some())
            .finish()
    }
}

impl<S> FirstAcceptedForager<S>
where
    S: PlanningSolution,
{
    pub fn new() -> Self {
        Self {
            accepted_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S> Clone for FirstAcceptedForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            accepted_move: None, // Fresh state for clone
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for FirstAcceptedForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M> LocalSearchForager<S, M> for FirstAcceptedForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) {
        if self.accepted_move.is_none() {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn accepted_count_limit(&self) -> Option<usize> {
        Some(1)
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.accepted_move.take()
    }
}

/// A forager that evaluates all accepted moves and picks the best.
///
/// Unlike `AcceptedCountForager(N)`, this forager never quits early - it
/// always evaluates the full move space before selecting the best score.
pub struct BestScoreForager<S>
where
    S: PlanningSolution,
{
    // Best accepted move index and score seen in the current step.
    best_move: Option<(CandidateId, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> BestScoreForager<S>
where
    S: PlanningSolution,
{
    pub fn new() -> Self {
        Self {
            best_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Debug for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestScoreForager")
            .field("has_move", &self.best_move.is_some())
            .finish()
    }
}

impl<S> Clone for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            best_move: None,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for BestScoreForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score) {
        self.best_move = None;
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) {
        match self.best_move {
            Some((_, best_score)) if best_score >= score => {}
            _ => self.best_move = Some((index, score)),
        }
    }

    fn is_quit_early(&self) -> bool {
        false // Never quit early — always evaluate the full move space
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.best_move.take()
    }
}

#[cfg(test)]
mod tests;
