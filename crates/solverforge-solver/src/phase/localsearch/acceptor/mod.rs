//! Acceptors for local search move acceptance.
//!
//! Acceptors determine whether a move should be accepted based on
//! comparing the resulting score with the previous score.

mod diversified_late_acceptance;
mod entity_tabu;
mod great_deluge;
mod hill_climbing;
mod late_acceptance;
mod move_tabu;
mod simulated_annealing;
mod step_counting;
mod tabu_search;
mod value_tabu;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

pub use diversified_late_acceptance::DiversifiedLateAcceptanceAcceptor;
pub use entity_tabu::EntityTabuAcceptor;
pub use great_deluge::GreatDelugeAcceptor;
pub use hill_climbing::HillClimbingAcceptor;
pub use late_acceptance::LateAcceptanceAcceptor;
pub use move_tabu::MoveTabuAcceptor;
pub use simulated_annealing::SimulatedAnnealingAcceptor;
pub use step_counting::StepCountingHillClimbingAcceptor;
pub use tabu_search::TabuSearchAcceptor;
pub use value_tabu::ValueTabuAcceptor;

/// Trait for accepting or rejecting moves in local search.
///
/// Acceptors implement different strategies for escaping local optima,
/// such as hill climbing, simulated annealing, or tabu search.
pub trait Acceptor<S: PlanningSolution>: Send + Debug {
    /// Records context about the move being evaluated.
    ///
    /// This MUST be called before `is_accepted()` for each move.
    /// Tabu-based acceptors use this to track which entities/moves are forbidden.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities affected by the move
    /// * `move_hash` - Hash identifying the specific move (for move tabu)
    fn record_move_context(&mut self, entity_indices: &[usize], move_hash: u64);

    /// Returns true if a move resulting in `move_score` should be accepted,
    /// given the previous step's score.
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool;

    /// Called when a phase starts.
    fn phase_started(&mut self, _initial_score: &S::Score) {}

    /// Called when a phase ends.
    fn phase_ended(&mut self) {}

    /// Called when a step starts.
    fn step_started(&mut self) {}

    /// Called when a step ends with an accepted move.
    fn step_ended(&mut self, _step_score: &S::Score) {}

    /// Sets the time gradient for temperature-based acceptors (e.g., SimulatedAnnealing).
    ///
    /// # Arguments
    /// * `time_gradient` - Progress ratio from 0.0 (start) to 1.0 (end)
    ///
    /// Default implementation is a no-op for acceptors that don't use time gradient.
    fn set_time_gradient(&mut self, _time_gradient: f64) {}
}

#[cfg(test)]
mod tests;
