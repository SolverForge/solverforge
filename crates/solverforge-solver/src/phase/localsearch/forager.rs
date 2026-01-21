//! Foragers for local search move selection
//!
//! Foragers collect accepted move indices during a step and select the
//! best one to apply. Uses index-based API for zero-clone operation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;

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
    /// Called at the start of each step to reset state.
    fn step_started(&mut self);

    /// Adds an accepted move index to the forager.
    ///
    /// The index refers to a position in the MoveArena.
    fn add_move_index(&mut self, index: usize, score: S::Score);

    /// Returns true if the forager has collected enough moves and
    /// wants to stop evaluating more.
    fn is_quit_early(&self) -> bool;

    /// Picks the best move index from those collected.
    ///
    /// Returns None if no moves were accepted.
    fn pick_move_index(&mut self) -> Option<(usize, S::Score)>;
}

/// A forager that collects a limited number of accepted moves.
///
/// Once the limit is reached, it quits early. It picks the best
/// move among those collected.
pub struct AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    /// Maximum number of accepted moves to collect.
    accepted_count_limit: usize,
    /// Collected move indices with their scores.
    accepted_moves: Vec<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> AcceptedCountForager<S>
where
    S: PlanningSolution,
{
    /// Creates a new forager with the given limit.
    ///
    /// # Arguments
    /// * `accepted_count_limit` - Stop after collecting this many accepted moves
    pub fn new(accepted_count_limit: usize) -> Self {
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
    fn step_started(&mut self) {
        self.accepted_moves.clear();
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        self.accepted_moves.push((index, score));
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_moves.len() >= self.accepted_count_limit
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
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
    /// The first accepted move index.
    accepted_move: Option<(usize, S::Score)>,
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
    /// Creates a new first-accepted forager.
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
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        if self.accepted_move.is_none() {
            self.accepted_move = Some((index, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        self.accepted_move.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct DummySolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for DummySolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    type TestMove = ChangeMove<DummySolution, i32>;

    #[test]
    fn test_accepted_count_forager_collects_indices() {
        let mut forager = AcceptedCountForager::<DummySolution>::new(3);
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-10));
        assert!(!<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(&forager));

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 1, SimpleScore::of(-5));
        assert!(!<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(&forager));

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 2, SimpleScore::of(-8));
        assert!(<AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(&forager));
    }

    #[test]
    fn test_accepted_count_forager_picks_best_index() {
        let mut forager = AcceptedCountForager::<DummySolution>::new(10);
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-10));
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 1, SimpleScore::of(-5)); // Best
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 2, SimpleScore::of(-8));

        let (index, score) = <AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .unwrap();
        assert_eq!(index, 1);
        assert_eq!(score, SimpleScore::of(-5));
    }

    #[test]
    fn test_accepted_count_forager_empty() {
        let mut forager = AcceptedCountForager::<DummySolution>::new(3);
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);

        assert!(<AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .is_none());
    }

    #[test]
    fn test_first_accepted_forager() {
        let mut forager = FirstAcceptedForager::<DummySolution>::new();
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);

        assert!(!<FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(&forager));

        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-10));
        assert!(<FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::is_quit_early(&forager));

        // Second move should be ignored
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 1, SimpleScore::of(-5));

        let (index, score) = <FirstAcceptedForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .unwrap();
        // Should get the first one
        assert_eq!(index, 0);
        assert_eq!(score, SimpleScore::of(-10));
    }

    #[test]
    fn test_forager_resets_on_step() {
        let mut forager = AcceptedCountForager::<DummySolution>::new(3);

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-10));

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager);
        // After reset, should be empty
        assert!(<AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .is_none());
    }
}
