/* Foragers for construction heuristic move selection

Foragers determine which move to select from the candidates
generated for each entity placement.

# Zero-Erasure Design

Foragers return indices into the placement's move Vec, not cloned moves.
The caller takes ownership via the index.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::ConstructionObligation;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::scope::{ProgressCallback, StepScope};

use super::Placement;

/// Selection result for a single construction placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionChoice {
    KeepCurrent,
    Select(usize),
}

/// Trait for selecting a move during construction.
///
/// Foragers evaluate candidate moves and pick one based on their strategy.
// Returns either a selected move index or an explicit keep-current choice.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait ConstructionForager<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /* Picks a construction choice from the placement's candidates.
     */
    fn pick_move_index<D: Director<S>>(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut D,
    ) -> ConstructionChoice;

    fn select_move_index<D, BestCb>(
        &self,
        placement: &Placement<S, M>,
        _construction_obligation: ConstructionObligation,
        step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    ) -> Option<ConstructionChoice>
    where
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        Some(self.pick_move_index(placement, step_scope.score_director_mut()))
    }
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
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
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
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
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
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Weakest Fit forager - picks the move with the lowest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the minimum strength. This is useful for
/// assigning the "weakest" or least constraining values first.
pub struct WeakestFitForager<S, M> {
    // Function to evaluate strength of a move.
    strength_fn: fn(&M, &S) -> i64,
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
    pub fn new(strength_fn: fn(&M, &S) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn strength(&self, mov: &M, solution: &S) -> i64 {
        (self.strength_fn)(mov, solution)
    }
}

/// Strongest Fit forager - picks the move with the highest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the maximum strength. This is useful for
/// assigning the "strongest" or most constraining values first.
pub struct StrongestFitForager<S, M> {
    // Function to evaluate strength of a move.
    strength_fn: fn(&M, &S) -> i64,
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
    pub fn new(strength_fn: fn(&M, &S) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn strength(&self, mov: &M, solution: &S) -> i64 {
        (self.strength_fn)(mov, solution)
    }
}

#[cfg(test)]
mod tests;
