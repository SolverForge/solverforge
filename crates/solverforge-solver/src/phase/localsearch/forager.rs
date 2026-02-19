//! Foragers for local search move selection
//!
//! Foragers collect accepted move indices during a step and select the
//! best one to apply. Uses index-based API for zero-clone operation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

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
    ///
    /// `best_score` is the best solution score ever seen.
    /// `last_step_score` is the score at the end of the previous step.
    /// Foragers that implement pick-early-on-improvement use these to decide
    /// when to stop evaluating moves.
    fn step_started(&mut self, best_score: S::Score, last_step_score: S::Score);

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
    /// # Panics
    /// Panics if `accepted_count_limit` is 0 — a zero limit would quit before
    /// evaluating any move, silently skipping every step.
    ///
    /// # Arguments
    /// * `accepted_count_limit` - Stop after collecting this many accepted moves
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
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score) {
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

/// A forager that evaluates all accepted moves and picks the best.
///
/// Unlike `AcceptedCountForager(N)`, this forager never quits early — it
/// always evaluates the full move space before selecting the best score.
pub struct BestScoreForager<S>
where
    S: PlanningSolution,
{
    /// Collected move indices with their scores.
    accepted_moves: Vec<(usize, S::Score)>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> BestScoreForager<S>
where
    S: PlanningSolution,
{
    /// Creates a new best-score forager.
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

    fn add_move_index(&mut self, index: usize, score: S::Score) {
        self.accepted_moves.push((index, score));
    }

    fn is_quit_early(&self) -> bool {
        false // Never quit early — always evaluate the full move space
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
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

/// A forager that picks the first accepted move that improves on the best score ever seen.
///
/// Once a move with a score strictly better than the all-time best is found, the
/// forager quits immediately and selects that move. If no such move exists, it falls
/// back to the best among all accepted moves.
pub struct FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    /// All-time best score — set at the start of each step.
    best_score: S::Score,
    /// Collected move indices with their scores.
    accepted_moves: Vec<(usize, S::Score)>,
    /// Whether we found a move that beats the best score.
    found_best_improving: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstBestScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    /// Creates a new first-best-score-improving forager.
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
            // Found a best-improving move — keep only this one and quit early
            self.accepted_moves.clear();
            self.accepted_moves.push((index, score));
            self.found_best_improving = true;
        } else {
            // Track as fallback unless already found best-improving
            if !self.found_best_improving {
                self.accepted_moves.push((index, score));
            }
        }
    }

    fn is_quit_early(&self) -> bool {
        self.found_best_improving
    }

    fn pick_move_index(&mut self) -> Option<(usize, S::Score)> {
        if self.accepted_moves.is_empty() {
            return None;
        }
        // If we found a best-improving move it's always the single entry (already best)
        if self.found_best_improving {
            return self.accepted_moves.pop();
        }
        // Otherwise pick the best among all collected fallbacks
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
    /// Score at the end of the previous step — set at the start of each step.
    last_step_score: S::Score,
    /// Collected move indices with their scores.
    accepted_moves: Vec<(usize, S::Score)>,
    /// Whether we found a move that beats the last step score.
    found_last_step_improving: bool,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> FirstLastStepScoreImprovingForager<S>
where
    S: PlanningSolution,
{
    /// Creates a new first-last-step-score-improving forager.
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
            // Found a last-step-improving move — keep only this one and quit early
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

    fn zero() -> SimpleScore {
        SimpleScore::of(0)
    }

    #[test]
    fn test_accepted_count_forager_collects_indices() {
        let mut forager = AcceptedCountForager::<DummySolution>::new(3);
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());

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
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());

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
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());

        assert!(<AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .is_none());
    }

    #[test]
    fn test_first_accepted_forager() {
        let mut forager = FirstAcceptedForager::<DummySolution>::new();
        <FirstAcceptedForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());

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

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());
        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-10));

        <AcceptedCountForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());
        // After reset, should be empty
        assert!(<AcceptedCountForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .is_none());
    }

    #[test]
    #[should_panic(expected = "accepted_count_limit must be > 0")]
    fn test_accepted_count_forager_zero_panics() {
        let _ = AcceptedCountForager::<DummySolution>::new(0);
    }

    #[test]
    fn test_best_score_forager_never_quits_early() {
        let mut forager = BestScoreForager::<DummySolution>::new();
        <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::step_started(&mut forager, zero(), zero());

        <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 0, SimpleScore::of(-5));
        assert!(!<BestScoreForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::is_quit_early(&forager));

        <BestScoreForager<DummySolution> as LocalSearchForager<DummySolution, TestMove>>::add_move_index(&mut forager, 1, SimpleScore::of(-1));
        let (index, score) = <BestScoreForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::pick_move_index(&mut forager)
        .unwrap();
        assert_eq!(index, 1);
        assert_eq!(score, SimpleScore::of(-1));
    }

    #[test]
    fn test_first_best_score_improving_quits_on_improvement() {
        let best = SimpleScore::of(-10);
        let mut forager = FirstBestScoreImprovingForager::<DummySolution>::new();
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::step_started(&mut forager, best, zero());

        // Score -15 is worse than best (-10), not improving
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 0, SimpleScore::of(-15));
        assert!(
            !<FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );

        // Score -5 is better than best (-10), triggers early quit
        <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 1, SimpleScore::of(-5));
        assert!(
            <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );

        let (index, score) =
            <FirstBestScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::pick_move_index(&mut forager)
            .unwrap();
        assert_eq!(index, 1);
        assert_eq!(score, SimpleScore::of(-5));
    }

    #[test]
    fn test_first_last_step_improving_quits_on_improvement() {
        let last_step = SimpleScore::of(-10);
        let mut forager = FirstLastStepScoreImprovingForager::<DummySolution>::new();
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::step_started(&mut forager, zero(), last_step);

        // Score -15 is worse than last step (-10)
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 0, SimpleScore::of(-15));
        assert!(
            !<FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );

        // Score -5 is better than last step (-10), triggers early quit
        <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
            DummySolution,
            TestMove,
        >>::add_move_index(&mut forager, 1, SimpleScore::of(-5));
        assert!(
            <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::is_quit_early(&forager)
        );

        let (index, score) =
            <FirstLastStepScoreImprovingForager<DummySolution> as LocalSearchForager<
                DummySolution,
                TestMove,
            >>::pick_move_index(&mut forager)
            .unwrap();
        assert_eq!(index, 1);
        assert_eq!(score, SimpleScore::of(-5));
    }
}
