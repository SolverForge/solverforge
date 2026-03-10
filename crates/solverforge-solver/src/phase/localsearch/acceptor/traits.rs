//! Acceptor trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

/// Trait for accepting or rejecting moves in local search.
///
/// Acceptors implement different strategies for escaping local optima,
/// such as hill climbing, simulated annealing, or tabu search.
pub trait Acceptor<S: PlanningSolution>: Send + Debug {
    /// Returns true if a move resulting in `move_score` should be accepted,
    /// given the previous step's score.
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool;

    /// Called when a phase starts.
    fn phase_started(&mut self, _initial_score: &S::Score) {}

    /// Called when a phase ends.
    fn phase_ended(&mut self) {}

    /// Called when a step starts.
    fn step_started(&mut self) {}

    /// Called when a step ends with an accepted move.
    fn step_ended(&mut self, _step_score: &S::Score) {}
}
