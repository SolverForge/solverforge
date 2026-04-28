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

    /* Picks the best move index from those collected.

    Returns None if no moves were accepted.
    */
    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)>;
}

mod improving;

pub use improving::{FirstBestScoreImprovingForager, FirstLastStepScoreImprovingForager};

/// A forager that retains up to `N` accepted moves and picks the best.
///
/// This forager does **not** quit early. The limit controls retained
/// accepted candidates, not neighborhood traversal. Early-exit behavior
/// belongs to the explicit `First*Improving` foragers.
pub struct AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    // Maximum number of accepted moves to retain.
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
    /// * `accepted_count_limit` - Retain up to this many accepted moves
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
        if self.accepted_moves.len() < self.accepted_count_limit {
            self.accepted_moves.push((index, score));
            return;
        }

        let mut worst_idx = 0;
        let mut worst_score = self.accepted_moves[0].1;
        for (i, &(_, retained_score)) in self.accepted_moves.iter().enumerate().skip(1) {
            if retained_score < worst_score {
                worst_idx = i;
                worst_score = retained_score;
            }
        }

        if score > worst_score {
            self.accepted_moves[worst_idx] = (index, score);
        }
    }

    fn is_quit_early(&self) -> bool {
        false
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

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.accepted_move.take()
    }
}

/// A forager that evaluates all accepted moves and picks the best.
///
/// Unlike `AcceptedCountForager(N)`, this forager never quits early — it
/// always evaluates the full move space before selecting the best score.
pub struct BestScoreForager<S>
where
    S: PlanningSolution,
{
    // Collected move indices with their scores.
    accepted_moves: Vec<(CandidateId, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> BestScoreForager<S>
where
    S: PlanningSolution,
{
    pub fn new() -> Self {
        Self {
            accepted_moves: Vec::new(),
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
            .field("accepted_count", &self.accepted_moves.len())
            .finish()
    }
}

impl<S> Clone for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            accepted_moves: Vec::new(),
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
        self.accepted_moves.clear();
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) {
        self.accepted_moves.push((index, score));
    }

    fn is_quit_early(&self) -> bool {
        false // Never quit early — always evaluate the full move space
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
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

#[cfg(test)]
mod tests;
