//! Builder module for constructing solver components from configuration.
//!
//! # Zero-Erasure Architecture
//!
//! This module uses [`AcceptorImpl`] enum for runtime acceptor selection
//! without type erasure. See [`crate::phase::localsearch::AcceptorImpl`].

use solverforge_config::AcceptorConfig;
use solverforge_core::domain::PlanningSolution;

use crate::phase::localsearch::{
    AcceptorImpl, HillClimbingAcceptor, LateAcceptanceAcceptor, SimulatedAnnealingAcceptor,
    TabuSearchAcceptor,
};

/// Builder for constructing acceptors from configuration.
///
/// Uses [`AcceptorImpl`] enum (monomorphic) instead of `Box<dyn>` for zero-erasure.
pub struct AcceptorBuilder;

impl AcceptorBuilder {
    /// Builds an acceptor from configuration.
    ///
    /// Returns [`AcceptorImpl`] enum - concrete type, no heap allocation.
    pub fn build<S: PlanningSolution>(config: &AcceptorConfig) -> AcceptorImpl<S> {
        AcceptorImpl::from_config(Some(config))
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
        let _acceptor: AcceptorImpl<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_tabu_search() {
        let config = AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(10),
            ..Default::default()
        });
        let _acceptor: AcceptorImpl<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_simulated_annealing() {
        let config = AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            starting_temperature: Some("1.5".to_string()),
        });
        let _acceptor: AcceptorImpl<TestSolution> = AcceptorBuilder::build(&config);
    }

    #[test]
    fn test_acceptor_builder_late_acceptance() {
        let config = AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(500),
        });
        let _acceptor: AcceptorImpl<TestSolution> = AcceptorBuilder::build(&config);
    }
}
