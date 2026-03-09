//! Builder module for constructing solver components from configuration
//!
//! This module provides the wiring between configuration types and
//! the actual solver implementation.

use std::fmt::Debug;

use solverforge_config::AcceptorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::phase::localsearch::{
    Acceptor, GreatDelugeAcceptor, HillClimbingAcceptor, LateAcceptanceAcceptor,
    SimulatedAnnealingAcceptor, TabuSearchAcceptor,
};

/// A concrete enum over all built-in acceptor types.
///
/// Returned by [`AcceptorBuilder::build`] to avoid `Box<dyn Acceptor<S>>`.
/// Dispatches to the inner acceptor via `match` — fully monomorphized.
#[allow(clippy::large_enum_variant)]
pub enum AnyAcceptor<S: PlanningSolution> {
    /// Hill climbing acceptor.
    HillClimbing(HillClimbingAcceptor),
    /// Tabu search acceptor.
    TabuSearch(TabuSearchAcceptor<S>),
    /// Simulated annealing acceptor.
    SimulatedAnnealing(SimulatedAnnealingAcceptor),
    /// Late acceptance acceptor.
    LateAcceptance(LateAcceptanceAcceptor<S>),
    /// Great deluge acceptor.
    GreatDeluge(GreatDelugeAcceptor<S>),
}

impl<S: PlanningSolution> Debug for AnyAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HillClimbing(a) => write!(f, "AnyAcceptor::HillClimbing({a:?})"),
            Self::TabuSearch(a) => write!(f, "AnyAcceptor::TabuSearch({a:?})"),
            Self::SimulatedAnnealing(a) => write!(f, "AnyAcceptor::SimulatedAnnealing({a:?})"),
            Self::LateAcceptance(a) => write!(f, "AnyAcceptor::LateAcceptance({a:?})"),
            Self::GreatDeluge(a) => write!(f, "AnyAcceptor::GreatDeluge({a:?})"),
        }
    }
}

impl<S: PlanningSolution> Clone for AnyAcceptor<S>
where
    S::Score: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::HillClimbing(a) => Self::HillClimbing(a.clone()),
            Self::TabuSearch(a) => Self::TabuSearch(a.clone()),
            Self::SimulatedAnnealing(a) => Self::SimulatedAnnealing(a.clone()),
            Self::LateAcceptance(a) => Self::LateAcceptance(a.clone()),
            Self::GreatDeluge(a) => Self::GreatDeluge(a.clone()),
        }
    }
}

impl<S: PlanningSolution> Acceptor<S> for AnyAcceptor<S>
where
    S::Score: Score,
{
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::is_accepted(a, last_step_score, move_score),
            Self::TabuSearch(a) => Acceptor::<S>::is_accepted(a, last_step_score, move_score),
            Self::SimulatedAnnealing(a) => {
                Acceptor::<S>::is_accepted(a, last_step_score, move_score)
            }
            Self::LateAcceptance(a) => Acceptor::<S>::is_accepted(a, last_step_score, move_score),
            Self::GreatDeluge(a) => Acceptor::<S>::is_accepted(a, last_step_score, move_score),
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::TabuSearch(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::LateAcceptance(a) => Acceptor::<S>::phase_started(a, initial_score),
            Self::GreatDeluge(a) => Acceptor::<S>::phase_started(a, initial_score),
        }
    }

    fn phase_ended(&mut self) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::phase_ended(a),
            Self::TabuSearch(a) => Acceptor::<S>::phase_ended(a),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::phase_ended(a),
            Self::LateAcceptance(a) => Acceptor::<S>::phase_ended(a),
            Self::GreatDeluge(a) => Acceptor::<S>::phase_ended(a),
        }
    }

    fn step_started(&mut self) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::step_started(a),
            Self::TabuSearch(a) => Acceptor::<S>::step_started(a),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::step_started(a),
            Self::LateAcceptance(a) => Acceptor::<S>::step_started(a),
            Self::GreatDeluge(a) => Acceptor::<S>::step_started(a),
        }
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        match self {
            Self::HillClimbing(a) => Acceptor::<S>::step_ended(a, step_score),
            Self::TabuSearch(a) => Acceptor::<S>::step_ended(a, step_score),
            Self::SimulatedAnnealing(a) => Acceptor::<S>::step_ended(a, step_score),
            Self::LateAcceptance(a) => Acceptor::<S>::step_ended(a, step_score),
            Self::GreatDeluge(a) => Acceptor::<S>::step_ended(a, step_score),
        }
    }
}

/// Builder for constructing acceptors from configuration.
pub struct AcceptorBuilder;

impl AcceptorBuilder {
    /// Builds a concrete [`AnyAcceptor`] from configuration.
    pub fn build<S: PlanningSolution>(config: &AcceptorConfig) -> AnyAcceptor<S>
    where
        S::Score: Score,
    {
        match config {
            AcceptorConfig::HillClimbing => AnyAcceptor::HillClimbing(HillClimbingAcceptor::new()),

            AcceptorConfig::TabuSearch(tabu_config) => {
                let tabu_size = tabu_config
                    .entity_tabu_size
                    .or(tabu_config.move_tabu_size)
                    .unwrap_or(7);
                AnyAcceptor::TabuSearch(TabuSearchAcceptor::<S>::new(tabu_size))
            }

            AcceptorConfig::SimulatedAnnealing(sa_config) => {
                let starting_temp = sa_config
                    .starting_temperature
                    .as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::new(
                    starting_temp,
                    0.999985,
                ))
            }

            AcceptorConfig::LateAcceptance(la_config) => {
                let size = la_config.late_acceptance_size.unwrap_or(400);
                AnyAcceptor::LateAcceptance(LateAcceptanceAcceptor::<S>::new(size))
            }

            AcceptorConfig::GreatDeluge(gd_config) => {
                let rain_speed = gd_config.water_level_increase_ratio.unwrap_or(0.001);
                AnyAcceptor::GreatDeluge(GreatDelugeAcceptor::<S>::new(rain_speed))
            }
        }
    }

    /// Creates a default hill climbing acceptor.
    pub fn hill_climbing<S: PlanningSolution>() -> HillClimbingAcceptor {
        HillClimbingAcceptor::new()
    }

    /// Creates a tabu search acceptor with the given size.
    pub fn tabu_search<S: PlanningSolution>(tabu_size: usize) -> TabuSearchAcceptor<S> {
        TabuSearchAcceptor::<S>::new(tabu_size)
    }

    /// Creates a simulated annealing acceptor.
    pub fn simulated_annealing(starting_temp: f64, decay_rate: f64) -> SimulatedAnnealingAcceptor {
        SimulatedAnnealingAcceptor::new(starting_temp, decay_rate)
    }

    /// Creates a late acceptance acceptor.
    pub fn late_acceptance<S: PlanningSolution>(size: usize) -> LateAcceptanceAcceptor<S> {
        LateAcceptanceAcceptor::<S>::new(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_config::{
        AcceptorConfig, LateAcceptanceConfig, SimulatedAnnealingConfig, TabuSearchConfig,
    };
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn test_acceptor_builder_hill_climbing() {
        let config = AcceptorConfig::HillClimbing;
        let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_tabu_search() {
        let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(10),
            ..Default::default()
        });
        let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_simulated_annealing() {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            starting_temperature: Some("1.5".to_string()),
        });
        let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_late_acceptance() {
        let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(500),
        });
        let _acceptor: AnyAcceptor<TestSolution> = AcceptorBuilder::build(&config);
    }
}
