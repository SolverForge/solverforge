//! Hill climbing acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Hill climbing acceptor - accepts improving or equal-score moves.
///
/// Accepts moves that result in a score equal to or better than the current score.
/// This enables plateau exploration, allowing the search to move sideways through
/// regions of equal score to eventually find paths to better solutions.
pub struct HillClimbingAcceptor<S> {
    _phantom: PhantomData<fn() -> S>,
}

impl<S> HillClimbingAcceptor<S> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for HillClimbingAcceptor<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Clone for HillClimbingAcceptor<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for HillClimbingAcceptor<S> {}

impl<S> Debug for HillClimbingAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HillClimbingAcceptor").finish()
    }
}

impl<S: PlanningSolution> Acceptor<S> for HillClimbingAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        move_score >= last_step_score
    }
}
