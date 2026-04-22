/* Acceptors for local search move acceptance.

Acceptors determine whether a move should be accepted based on
comparing the resulting score with the previous score.
*/

mod diversified_late_acceptance;
mod great_deluge;
mod hill_climbing;
mod late_acceptance;
mod simulated_annealing;
mod step_counting;
mod tabu_search;
mod traits;

pub use diversified_late_acceptance::DiversifiedLateAcceptanceAcceptor;
pub use great_deluge::GreatDelugeAcceptor;
pub use hill_climbing::HillClimbingAcceptor;
pub use late_acceptance::LateAcceptanceAcceptor;
pub use simulated_annealing::SimulatedAnnealingAcceptor;
pub use step_counting::StepCountingHillClimbingAcceptor;
pub use tabu_search::TabuSearchAcceptor;
pub(crate) use tabu_search::TabuSearchPolicy;
pub use traits::Acceptor;

#[cfg(test)]
mod tests;
