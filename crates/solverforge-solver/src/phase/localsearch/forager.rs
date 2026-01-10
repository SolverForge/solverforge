//! Foragers for local search move selection
//!
//! Foragers collect accepted moves during a step and select the
//! best one to apply.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;

/// Trait for collecting and selecting moves in local search.
///
/// Foragers are responsible for:
/// - Collecting accepted moves during move evaluation
/// - Deciding when to quit evaluating early
/// - Selecting the best move to apply
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `M` - The move type
pub trait LocalSearchForager<S, D, M>: Send + Debug
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S, D>,
{
    /// Called at the start of each step to reset state.
    fn step_started(&mut self);

    /// Adds an accepted move to the forager.
    fn add_move(&mut self, m: M, score: S::Score);

    /// Returns true if the forager has collected enough moves and
    /// wants to stop evaluating more.
    fn is_quit_early(&self) -> bool;

    /// Picks the best move from those collected.
    ///
    /// Returns None if no moves were accepted.
    fn pick_move(&mut self) -> Option<(M, S::Score)>;
}

/// A forager that collects a limited number of accepted moves.
///
/// Once the limit is reached, it quits early. It picks the best
/// move among those collected.
pub struct AcceptedCountForager<S, D, M>
where
    S: PlanningSolution,
{
    /// Maximum number of accepted moves to collect.
    accepted_count_limit: usize,
    /// Collected moves with their scores.
    accepted_moves: Vec<(M, S::Score)>,
    _phantom: PhantomData<fn() -> (S, D)>,
}

impl<S, D, M> AcceptedCountForager<S, D, M>
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

impl<S, D, M> Clone for AcceptedCountForager<S, D, M>
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

impl<S, D, M> Debug for AcceptedCountForager<S, D, M>
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

impl<S, D, M> LocalSearchForager<S, D, M> for AcceptedCountForager<S, D, M>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S, D>,
{
    fn step_started(&mut self) {
        self.accepted_moves.clear();
    }

    fn add_move(&mut self, m: M, score: S::Score) {
        self.accepted_moves.push((m, score));
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_moves.len() >= self.accepted_count_limit
    }

    fn pick_move(&mut self) -> Option<(M, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
        }

        // Find the best move (highest score)
        let mut best_index = 0;
        let mut best_score = &self.accepted_moves[0].1;

        for (i, (_, score)) in self.accepted_moves.iter().enumerate().skip(1) {
            if score > best_score {
                best_index = i;
                best_score = score;
            }
        }

        // Remove and return the best move
        Some(self.accepted_moves.swap_remove(best_index))
    }
}

/// A forager that picks the first accepted move.
///
/// This is the simplest forager - it quits after the first accepted move.
pub struct FirstAcceptedForager<S, D, M>
where
    S: PlanningSolution,
{
    /// The first accepted move.
    accepted_move: Option<(M, S::Score)>,
    _phantom: PhantomData<fn() -> (S, D)>,
}

impl<S, D, M> Debug for FirstAcceptedForager<S, D, M>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstAcceptedForager")
            .field("has_move", &self.accepted_move.is_some())
            .finish()
    }
}

impl<S, D, M> FirstAcceptedForager<S, D, M>
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

impl<S, D, M> Clone for FirstAcceptedForager<S, D, M>
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

impl<S, D, M> Default for FirstAcceptedForager<S, D, M>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, D, M> LocalSearchForager<S, D, M> for FirstAcceptedForager<S, D, M>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    M: Move<S, D>,
{
    fn step_started(&mut self) {
        self.accepted_move = None;
    }

    fn add_move(&mut self, m: M, score: S::Score) {
        if self.accepted_move.is_none() {
            self.accepted_move = Some((m, score));
        }
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_move.is_some()
    }

    fn pick_move(&mut self) -> Option<(M, S::Score)> {
        self.accepted_move.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;

    #[derive(Clone, Debug)]
    struct DummySolution {
        values: Vec<Option<i32>>,
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

    // Typed getter - zero erasure
    fn get_value(s: &DummySolution, idx: usize) -> Option<i32> {
        s.values.get(idx).copied().flatten()
    }

    // Typed setter - zero erasure
    fn set_value(s: &mut DummySolution, idx: usize, v: Option<i32>) {
        if let Some(val) = s.values.get_mut(idx) {
            *val = v;
        }
    }

    type TestDirector = SimpleScoreDirector<DummySolution, fn(&DummySolution) -> SimpleScore>;
    type TestMove = ChangeMove<DummySolution, TestDirector, i32>;

    fn create_move(value: i32) -> TestMove {
        ChangeMove::new(0, Some(value), get_value, set_value, "test", 0)
    }

    #[test]
    fn test_accepted_count_forager_collects_moves() {
        let mut forager = AcceptedCountForager::<DummySolution, TestDirector, TestMove>::new(3);
        forager.step_started();

        forager.add_move(create_move(1), SimpleScore::of(-10));
        assert!(!forager.is_quit_early());

        forager.add_move(create_move(2), SimpleScore::of(-5));
        assert!(!forager.is_quit_early());

        forager.add_move(create_move(3), SimpleScore::of(-8));
        assert!(forager.is_quit_early());
    }

    #[test]
    fn test_accepted_count_forager_picks_best() {
        let mut forager = AcceptedCountForager::<DummySolution, TestDirector, TestMove>::new(10);
        forager.step_started();

        forager.add_move(create_move(1), SimpleScore::of(-10));
        forager.add_move(create_move(2), SimpleScore::of(-5)); // Best
        forager.add_move(create_move(3), SimpleScore::of(-8));

        let (_, score) = forager.pick_move().unwrap();
        assert_eq!(score, SimpleScore::of(-5));
    }

    #[test]
    fn test_accepted_count_forager_empty() {
        let mut forager = AcceptedCountForager::<DummySolution, TestDirector, TestMove>::new(3);
        forager.step_started();

        assert!(forager.pick_move().is_none());
    }

    #[test]
    fn test_first_accepted_forager() {
        let mut forager = FirstAcceptedForager::<DummySolution, TestDirector, TestMove>::new();
        forager.step_started();

        assert!(!forager.is_quit_early());

        forager.add_move(create_move(1), SimpleScore::of(-10));
        assert!(forager.is_quit_early());

        // Second move should be ignored
        forager.add_move(create_move(2), SimpleScore::of(-5));

        let (_, score) = forager.pick_move().unwrap();
        // Should get the first one, not the better second one
        assert_eq!(score, SimpleScore::of(-10));
    }

    #[test]
    fn test_forager_resets_on_step() {
        let mut forager = AcceptedCountForager::<DummySolution, TestDirector, TestMove>::new(3);

        forager.step_started();
        forager.add_move(create_move(1), SimpleScore::of(-10));

        forager.step_started();
        // After reset, should be empty
        assert!(forager.pick_move().is_none());
    }
}
