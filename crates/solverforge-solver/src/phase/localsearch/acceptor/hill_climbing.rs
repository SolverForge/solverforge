//! Hill climbing acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Hill climbing acceptor - accepts only improving moves.
///
/// This is the simplest acceptor. It only accepts moves that result
/// in a strictly better score. This can get stuck in local optima.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::HillClimbingAcceptor;
///
/// let acceptor = HillClimbingAcceptor::new();
/// ```
#[derive(Debug, Clone, Default)]
pub struct HillClimbingAcceptor;

impl HillClimbingAcceptor {
    /// Creates a new hill climbing acceptor.
    pub fn new() -> Self {
        Self
    }
}

impl<S: PlanningSolution> Acceptor<S> for HillClimbingAcceptor {
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Accept if the move score is better than the last step score
        move_score > last_step_score
    }
}
