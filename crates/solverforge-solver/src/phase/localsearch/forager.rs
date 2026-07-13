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
    fn step_started(&mut self, best_score: S::Score, last_step_score: S::Score, step_seed: u64);

    /* Adds an accepted move index to the forager.

    The ID refers to a live candidate in the current move cursor.
    */
    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision;

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

/// Tells the phase which candidate payload remains owned after online foraging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForagerDecision {
    /// Retain the newly accepted candidate.
    Keep,
    /// Release the newly accepted candidate; the current winner remains retained.
    Release,
    /// Retain the new candidate and release the replaced candidate immediately.
    Replace(CandidateId),
}

pub(super) struct BestCandidate<S>
where
    S: PlanningSolution,
{
    selected: Option<(CandidateId, S::Score)>,
    equal_count: u64,
    step_seed: u64,
    random_ties: bool,
}

impl<S> BestCandidate<S>
where
    S: PlanningSolution,
{
    fn new(random_ties: bool) -> Self {
        Self {
            selected: None,
            equal_count: 0,
            step_seed: 0,
            random_ties,
        }
    }

    fn reset(&mut self, step_seed: u64) {
        self.selected = None;
        self.equal_count = 0;
        self.step_seed = step_seed;
    }

    fn consider(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        let Some((selected, best_score)) = self.selected else {
            self.selected = Some((index, score));
            self.equal_count = 1;
            return ForagerDecision::Keep;
        };

        match score.cmp(&best_score) {
            std::cmp::Ordering::Less => ForagerDecision::Release,
            std::cmp::Ordering::Greater => self.replace(index, score),
            std::cmp::Ordering::Equal => {
                self.equal_count += 1;
                if self.random_ties && reservoir_pick(self.step_seed, self.equal_count) {
                    self.selected = Some((index, score));
                    ForagerDecision::Replace(selected)
                } else {
                    ForagerDecision::Release
                }
            }
        }
    }

    pub(super) fn replace(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        self.equal_count = 1;
        match self.selected.replace((index, score)) {
            Some((replaced, _)) => ForagerDecision::Replace(replaced),
            None => ForagerDecision::Keep,
        }
    }

    pub(super) fn take(&mut self) -> Option<(CandidateId, S::Score)> {
        self.equal_count = 0;
        self.selected.take()
    }

    pub(super) fn has_selection(&self) -> bool {
        self.selected.is_some()
    }

    fn random_ties(&self) -> bool {
        self.random_ties
    }
}

fn reservoir_pick(step_seed: u64, equal_count: u64) -> bool {
    let mixed = splitmix64(
        step_seed ^ equal_count.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xF04A_63E2_39B7_4D11,
    );
    mixed.is_multiple_of(equal_count)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
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
    accepted_count: usize,
    best_move: BestCandidate<S>,
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
    pub fn new(accepted_count_limit: usize, random_ties: bool) -> Self {
        assert!(
            accepted_count_limit > 0,
            "AcceptedCountForager: accepted_count_limit must be > 0, got 0"
        );
        Self {
            accepted_count_limit,
            accepted_count: 0,
            best_move: BestCandidate::new(random_ties),
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
            accepted_count: 0,
            best_move: BestCandidate::new(self.best_move.random_ties()),
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
            .field("accepted_count", &self.accepted_count)
            .finish()
    }
}

impl<S, M> LocalSearchForager<S, M> for AcceptedCountForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score, step_seed: u64) {
        self.accepted_count = 0;
        self.best_move.reset(step_seed);
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        if self.accepted_count >= self.accepted_count_limit {
            return ForagerDecision::Release;
        }
        self.accepted_count += 1;
        self.best_move.consider(index, score)
    }

    fn is_quit_early(&self) -> bool {
        self.accepted_count >= self.accepted_count_limit
    }

    fn accepted_count_limit(&self) -> Option<usize> {
        Some(self.accepted_count_limit)
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.best_move.take()
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
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score, _step_seed: u64) {
        self.accepted_move = None;
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        if self.accepted_move.is_none() {
            self.accepted_move = Some((index, score));
            ForagerDecision::Keep
        } else {
            ForagerDecision::Release
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
    best_move: BestCandidate<S>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> BestScoreForager<S>
where
    S: PlanningSolution,
{
    pub fn new(random_ties: bool) -> Self {
        Self {
            best_move: BestCandidate::new(random_ties),
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn default() -> Self {
        Self::new(true)
    }
}

impl<S> Debug for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestScoreForager")
            .field("has_move", &self.best_move.has_selection())
            .finish()
    }
}

impl<S> Clone for BestScoreForager<S>
where
    S: PlanningSolution,
{
    fn clone(&self) -> Self {
        Self {
            best_move: BestCandidate::new(self.best_move.random_ties()),
            _phantom: PhantomData,
        }
    }
}

impl<S, M> LocalSearchForager<S, M> for BestScoreForager<S>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn step_started(&mut self, _best_score: S::Score, _last_step_score: S::Score, step_seed: u64) {
        self.best_move.reset(step_seed);
    }

    fn add_move_index(&mut self, index: CandidateId, score: S::Score) -> ForagerDecision {
        self.best_move.consider(index, score)
    }

    fn is_quit_early(&self) -> bool {
        false // Never quit early — always evaluate the full move space
    }

    fn pick_move_index(&mut self) -> Option<(CandidateId, S::Score)> {
        self.best_move.take()
    }
}

#[cfg(test)]
mod any_tests;
#[cfg(test)]
mod tests;
