/* Acceptors for local search move acceptance.

Acceptors determine whether a move should be accepted based on
comparing the resulting score with the previous score.
*/

pub(crate) mod diversified_late_acceptance;
pub(crate) mod great_deluge;
pub(crate) mod hill_climbing;
pub(crate) mod late_acceptance;
pub(crate) mod simulated_annealing;
pub(crate) mod step_counting;
pub(crate) mod tabu_search;
pub(crate) mod traits;

pub(crate) use diversified_late_acceptance::DiversifiedLateAcceptanceAcceptor;
pub(crate) use great_deluge::GreatDelugeAcceptor;
pub(crate) use hill_climbing::HillClimbingAcceptor;
pub(crate) use late_acceptance::LateAcceptanceAcceptor;
pub(crate) use simulated_annealing::{
    HardRegressionPolicy, SimulatedAnnealingAcceptor, SimulatedAnnealingCalibration,
};
pub(crate) use step_counting::StepCountingHillClimbingAcceptor;
pub(crate) use tabu_search::TabuSearchAcceptor;
pub(crate) use tabu_search::TabuSearchPolicy;
pub(crate) use traits::Acceptor;

#[cfg(test)]
mod tests;
