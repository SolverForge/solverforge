//! Public fluent API for configuring and running solvers.
//!
//! Provides `SolverBuilder` for fluent solver configuration.

use std::time::Duration;

use solverforge_config::{
    AcceptorConfig, ConfigError, ConstructionHeuristicConfig, ConstructionHeuristicType,
    GreatDelugeConfig, LateAcceptanceConfig, LocalSearchConfig, PhaseConfig,
    SimulatedAnnealingConfig, SolverConfig, TabuSearchConfig,
};

/// Fluent builder for configuring solvers.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use solverforge_solver::public_api::SolverBuilder;
/// use solverforge_config::ConstructionHeuristicType;
///
/// let config = SolverBuilder::new()
///     .with_time_limit(Duration::from_secs(60))
///     .with_construction(ConstructionHeuristicType::FirstFit)
///     .with_tabu_search(7)
///     .build();
/// ```
pub struct SolverBuilder {
    config: SolverConfig,
}

impl Default for SolverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverBuilder {
    /// Creates a new solver builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: SolverConfig::default(),
        }
    }

    /// Loads configuration from a TOML file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let config = SolverConfig::load(path)?;
        Ok(Self { config })
    }

    /// Sets the time limit.
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.config = self.config.with_termination_seconds(duration.as_secs());
        self
    }

    /// Sets the step count limit.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        let mut term = self.config.termination.unwrap_or_default();
        term.step_count_limit = Some(limit);
        self.config.termination = Some(term);
        self
    }

    /// Sets the unimproved step count limit.
    pub fn with_unimproved_step_limit(mut self, limit: u64) -> Self {
        let mut term = self.config.termination.unwrap_or_default();
        term.unimproved_step_count_limit = Some(limit);
        self.config.termination = Some(term);
        self
    }

    /// Adds a construction heuristic phase.
    pub fn with_construction(mut self, heuristic_type: ConstructionHeuristicType) -> Self {
        self.config.phases.push(PhaseConfig::ConstructionHeuristic(
            ConstructionHeuristicConfig {
                construction_heuristic_type: heuristic_type,
                termination: None,
            },
        ));
        self
    }

    /// Adds a local search phase with the given acceptor.
    pub fn with_local_search(mut self, acceptor: AcceptorConfig) -> Self {
        self.config
            .phases
            .push(PhaseConfig::LocalSearch(LocalSearchConfig {
                acceptor: Some(acceptor),
                forager: None,
                move_selector: None,
                termination: None,
            }));
        self
    }

    /// Adds a local search phase with hill climbing.
    pub fn with_hill_climbing(self) -> Self {
        self.with_local_search(AcceptorConfig::HillClimbing)
    }

    /// Adds a local search phase with tabu search.
    pub fn with_tabu_search(self, entity_tabu_size: usize) -> Self {
        self.with_local_search(AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(entity_tabu_size),
            value_tabu_size: None,
            move_tabu_size: None,
            undo_move_tabu_size: None,
        }))
    }

    /// Adds a local search phase with simulated annealing.
    pub fn with_simulated_annealing(self, starting_temperature: f64, decay_rate: f64) -> Self {
        self.with_local_search(AcceptorConfig::SimulatedAnnealing(
            SimulatedAnnealingConfig {
                starting_temperature: Some(format!("{}", starting_temperature)),
                decay_rate: Some(decay_rate),
            },
        ))
    }

    /// Adds a local search phase with late acceptance.
    pub fn with_late_acceptance(self, size: usize) -> Self {
        self.with_local_search(AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(size),
        }))
    }

    /// Adds a local search phase with great deluge.
    pub fn with_great_deluge(self, water_level_increase_ratio: f64) -> Self {
        self.with_local_search(AcceptorConfig::GreatDeluge(GreatDelugeConfig {
            water_level_increase_ratio: Some(water_level_increase_ratio),
        }))
    }

    /// Adds a local search phase with default late acceptance.
    pub fn with_default_local_search(self) -> Self {
        self.with_late_acceptance(400)
    }

    /// Sets the random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.config = self.config.with_random_seed(seed);
        self
    }

    /// Builds the solver configuration.
    pub fn build(self) -> SolverConfig {
        self.config
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &SolverConfig {
        &self.config
    }
}

/// Helper functions to create acceptor configs.
pub mod acceptors {
    use super::*;

    /// Creates a hill climbing acceptor config.
    pub fn hill_climbing() -> AcceptorConfig {
        AcceptorConfig::HillClimbing
    }

    /// Creates a tabu search acceptor config with given size.
    pub fn tabu_search(entity_tabu_size: usize) -> AcceptorConfig {
        AcceptorConfig::TabuSearch(TabuSearchConfig {
            entity_tabu_size: Some(entity_tabu_size),
            value_tabu_size: None,
            move_tabu_size: None,
            undo_move_tabu_size: None,
        })
    }

    /// Creates a simulated annealing acceptor config.
    pub fn simulated_annealing(starting_temperature: f64, decay_rate: f64) -> AcceptorConfig {
        AcceptorConfig::SimulatedAnnealing(SimulatedAnnealingConfig {
            starting_temperature: Some(format!("{}", starting_temperature)),
            decay_rate: Some(decay_rate),
        })
    }

    /// Creates a late acceptance acceptor config.
    pub fn late_acceptance(size: usize) -> AcceptorConfig {
        AcceptorConfig::LateAcceptance(LateAcceptanceConfig {
            late_acceptance_size: Some(size),
        })
    }

    /// Creates a great deluge acceptor config.
    pub fn great_deluge(water_level_increase_ratio: f64) -> AcceptorConfig {
        AcceptorConfig::GreatDeluge(GreatDelugeConfig {
            water_level_increase_ratio: Some(water_level_increase_ratio),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_builder_time_limit() {
        let config = SolverBuilder::new()
            .with_time_limit(Duration::from_secs(120))
            .build();
        assert!(config.termination.is_some());
    }

    #[test]
    fn test_solver_builder_phases() {
        let config = SolverBuilder::new()
            .with_construction(ConstructionHeuristicType::FirstFit)
            .with_tabu_search(7)
            .build();
        assert_eq!(config.phases.len(), 2);
    }

    #[test]
    fn test_acceptor_helpers() {
        let _ = acceptors::hill_climbing();
        let _ = acceptors::tabu_search(7);
        let _ = acceptors::simulated_annealing(1.0, 0.99);
        let _ = acceptors::late_acceptance(400);
        let _ = acceptors::great_deluge(0.0000001);
    }

    #[test]
    fn test_solver_builder_fluent() {
        let config = SolverBuilder::new()
            .with_time_limit(Duration::from_secs(60))
            .with_seed(42)
            .with_construction(ConstructionHeuristicType::FirstFit)
            .with_hill_climbing()
            .build();

        assert_eq!(config.random_seed, Some(42));
        assert_eq!(config.phases.len(), 2);
    }
}
