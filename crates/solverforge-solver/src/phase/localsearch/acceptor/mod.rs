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
mod traits;
mod value_tabu;

pub use diversified_late_acceptance::DiversifiedLateAcceptanceAcceptor;
pub use entity_tabu::EntityTabuAcceptor;
pub use great_deluge::GreatDelugeAcceptor;
pub use hill_climbing::HillClimbingAcceptor;
pub use late_acceptance::LateAcceptanceAcceptor;
pub use move_tabu::MoveTabuAcceptor;
pub use simulated_annealing::SimulatedAnnealingAcceptor;
pub use step_counting::StepCountingHillClimbingAcceptor;
pub use tabu_search::TabuSearchAcceptor;
pub use traits::Acceptor;
pub use value_tabu::ValueTabuAcceptor;

#[cfg(test)]
mod tests;
