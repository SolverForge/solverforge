//! Builder module for constructing solver components from configuration
//!
//! This module provides the wiring between configuration types and
//! the actual solver implementation.

use solverforge_config::AcceptorConfig;
use solverforge_core::domain::PlanningSolution;

use crate::phase::localsearch::{
    Acceptor, HillClimbingAcceptor, LateAcceptanceAcceptor, SimulatedAnnealingAcceptor,
    TabuSearchAcceptor,
};

/// Builder for constructing acceptors from configuration.
pub struct AcceptorBuilder;

impl AcceptorBuilder {
    /// Builds an acceptor from configuration.
    pub fn build<S: PlanningSolution>(config: &AcceptorConfig) -> Box<dyn Acceptor<S>> {
        match config {
            AcceptorConfig::HillClimbing => Box::new(HillClimbingAcceptor::new()),

            AcceptorConfig::TabuSearch(tabu_config) => {
                // Use entity tabu size if specified, otherwise default
                let tabu_size = tabu_config
                    .entity_tabu_size
                    .or(tabu_config.move_tabu_size)
                    .unwrap_or(7);
                Box::new(TabuSearchAcceptor::<S>::new(tabu_size))
            }

            AcceptorConfig::SimulatedAnnealing(sa_config) => {
                // Parse starting temperature (default to 1.0 if not specified)
                let starting_temp = sa_config
                    .starting_temperature
                    .as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(1.0);
                Box::new(SimulatedAnnealingAcceptor::new(starting_temp, 0.99))
            }

            AcceptorConfig::LateAcceptance(la_config) => {
                let size = la_config.late_acceptance_size.unwrap_or(400);
                Box::new(LateAcceptanceAcceptor::<S>::new(size))
            }

            AcceptorConfig::GreatDeluge(_) => {
                // Great deluge not yet implemented, fall back to hill climbing
                tracing::warn!("Great deluge acceptor not yet implemented, using hill climbing");
                Box::new(HillClimbingAcceptor::new())
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
    pub fn simulated_annealing(
        starting_temp: f64,
        decay_rate: f64,
    ) -> SimulatedAnnealingAcceptor {
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
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_tabu_search() {
        let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(10),
            ..Default::default()
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_simulated_annealing() {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            starting_temperature: Some("1.5".to_string()),
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_late_acceptance() {
        let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(500),
        });
        let _acceptor: Box<dyn Acceptor<TestSolution>> = AcceptorBuilder::build(&config);
    }
}
