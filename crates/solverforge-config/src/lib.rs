//! Configuration system for SolverForge.
//!
//! Load solver configuration from TOML files to control termination,
//! phases, and acceptors without code changes.
//!
//! # Examples
//!
//! Load configuration from TOML string:
//!
//! ```
//! use solverforge_config::SolverConfig;
//! use std::time::Duration;
//!
//! let config = SolverConfig::from_toml_str(r#"
//!     [termination]
//!     seconds_spent_limit = 30
//!     unimproved_seconds_spent_limit = 5
//!
//!     [[phases]]
//!     type = "construction_heuristic"
//!     construction_heuristic_type = "first_fit"
//!
//!     [[phases]]
//!     type = "local_search"
//!     [phases.acceptor]
//!     type = "late_acceptance"
//!     late_acceptance_size = 400
//! "#).unwrap();
//!
//! assert_eq!(config.time_limit(), Some(Duration::from_secs(30)));
//! assert_eq!(config.phases.len(), 2);
//! ```
//!
//! Use default config when file is missing:
//!
//! ```
//! use solverforge_config::SolverConfig;
//!
//! let config = SolverConfig::load("solver.toml").unwrap_or_default();
//! // Proceeds with defaults if file doesn't exist
//! ```

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration error
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Main solver configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SolverConfig {
    /// Environment mode affecting reproducibility and assertions.
    #[serde(default)]
    pub environment_mode: EnvironmentMode,

    /// Random seed for reproducible results.
    #[serde(default)]
    pub random_seed: Option<u64>,

    /// Number of threads for parallel move evaluation.
    #[serde(default)]
    pub move_thread_count: MoveThreadCount,

    /// Termination configuration.
    #[serde(default)]
    pub termination: Option<TerminationConfig>,

    /// Score director configuration.
    #[serde(default)]
    pub score_director: Option<ScoreDirectorConfig>,

    /// Phase configurations.
    #[serde(default)]
    pub phases: Vec<PhaseConfig>,
}

impl SolverConfig {
    /// Creates a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration from a TOML file.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or contains invalid TOML.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        Self::from_toml_file(path)
    }

    /// Loads configuration from a TOML file.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_toml_str(&contents)
    }

    /// Parses configuration from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(s)?)
    }

    /// Loads configuration from a YAML file.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&contents)
    }

    /// Parses configuration from a YAML string.
    pub fn from_yaml_str(s: &str) -> Result<Self, ConfigError> {
        Ok(serde_yaml::from_str(s)?)
    }

    /// Sets the termination time limit.
    pub fn with_termination_seconds(mut self, seconds: u64) -> Self {
        self.termination = Some(TerminationConfig {
            seconds_spent_limit: Some(seconds),
            ..self.termination.unwrap_or_default()
        });
        self
    }

    /// Sets the random seed.
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// Adds a phase configuration.
    pub fn with_phase(mut self, phase: PhaseConfig) -> Self {
        self.phases.push(phase);
        self
    }

    /// Returns the termination time limit, if configured.
    ///
    /// Convenience method that delegates to `termination.time_limit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_config::SolverConfig;
    /// use std::time::Duration;
    ///
    /// let config = SolverConfig::from_toml_str(r#"
    ///     [termination]
    ///     seconds_spent_limit = 30
    /// "#).unwrap();
    ///
    /// assert_eq!(config.time_limit(), Some(Duration::from_secs(30)));
    /// ```
    pub fn time_limit(&self) -> Option<Duration> {
        self.termination.as_ref().and_then(|t| t.time_limit())
    }
}

/// Environment mode affecting solver behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentMode {
    /// Non-reproducible mode with minimal overhead.
    #[default]
    NonReproducible,

    /// Reproducible mode with deterministic behavior.
    Reproducible,

    /// Fast assert mode with basic assertions.
    FastAssert,

    /// Full assert mode with comprehensive assertions.
    FullAssert,
}

/// Move thread count configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveThreadCount {
    /// Automatically determine thread count.
    #[default]
    Auto,

    /// No parallel move evaluation.
    None,

    /// Specific number of threads.
    Count(usize),
}

/// Termination configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TerminationConfig {
    /// Maximum seconds to spend solving.
    pub seconds_spent_limit: Option<u64>,

    /// Maximum minutes to spend solving.
    pub minutes_spent_limit: Option<u64>,

    /// Target best score to achieve (as string, e.g., "0hard/0soft").
    pub best_score_limit: Option<String>,

    /// Maximum number of steps.
    pub step_count_limit: Option<u64>,

    /// Maximum unimproved steps before terminating.
    pub unimproved_step_count_limit: Option<u64>,

    /// Maximum seconds without improvement.
    pub unimproved_seconds_spent_limit: Option<u64>,
}

impl TerminationConfig {
    /// Returns the time limit as a Duration, if any.
    pub fn time_limit(&self) -> Option<Duration> {
        let seconds =
            self.seconds_spent_limit.unwrap_or(0) + self.minutes_spent_limit.unwrap_or(0) * 60;
        if seconds > 0 {
            Some(Duration::from_secs(seconds))
        } else {
            None
        }
    }

    /// Returns the unimproved time limit as a Duration, if any.
    pub fn unimproved_time_limit(&self) -> Option<Duration> {
        self.unimproved_seconds_spent_limit.map(Duration::from_secs)
    }
}

/// Score director configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ScoreDirectorConfig {
    /// Fully qualified name of the constraint provider type.
    pub constraint_provider: Option<String>,

    /// Whether to enable constraint matching assertions.
    #[serde(default)]
    pub constraint_match_enabled: bool,
}

/// Phase configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhaseConfig {
    /// Construction heuristic phase.
    ConstructionHeuristic(ConstructionHeuristicConfig),

    /// Local search phase.
    LocalSearch(LocalSearchConfig),

    /// Exhaustive search phase.
    ExhaustiveSearch(ExhaustiveSearchConfig),

    /// Partitioned search phase.
    PartitionedSearch(PartitionedSearchConfig),

    /// Custom phase.
    Custom(CustomPhaseConfig),
}

/// Construction heuristic configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ConstructionHeuristicConfig {
    /// Type of construction heuristic.
    #[serde(default)]
    pub construction_heuristic_type: ConstructionHeuristicType,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Construction heuristic types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionHeuristicType {
    /// First fit heuristic.
    #[default]
    FirstFit,

    /// First fit decreasing (by entity difficulty).
    FirstFitDecreasing,

    /// Weakest fit heuristic.
    WeakestFit,

    /// Weakest fit decreasing.
    WeakestFitDecreasing,

    /// Strongest fit heuristic.
    StrongestFit,

    /// Strongest fit decreasing.
    StrongestFitDecreasing,

    /// Cheapest insertion (greedy).
    CheapestInsertion,

    /// Allocate entity from queue.
    AllocateEntityFromQueue,

    /// Allocate to value from queue.
    AllocateToValueFromQueue,
}

/// Local search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalSearchConfig {
    /// Acceptor configuration.
    pub acceptor: Option<AcceptorConfig>,

    /// Forager configuration.
    pub forager: Option<ForagerConfig>,

    /// Move selector configuration.
    pub move_selector: Option<MoveSelectorConfig>,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Acceptor configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcceptorConfig {
    /// Hill climbing (only accept improving moves).
    HillClimbing,

    /// Tabu search acceptor.
    TabuSearch(TabuSearchConfig),

    /// Simulated annealing acceptor.
    SimulatedAnnealing(SimulatedAnnealingConfig),

    /// Late acceptance acceptor.
    LateAcceptance(LateAcceptanceConfig),

    /// Great deluge acceptor.
    GreatDeluge(GreatDelugeConfig),
}

/// Tabu search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TabuSearchConfig {
    /// Size of entity tabu list.
    pub entity_tabu_size: Option<usize>,

    /// Size of value tabu list.
    pub value_tabu_size: Option<usize>,

    /// Size of move tabu list.
    pub move_tabu_size: Option<usize>,

    /// Size of undo move tabu list.
    pub undo_move_tabu_size: Option<usize>,
}

/// Simulated annealing configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SimulatedAnnealingConfig {
    /// Starting temperature.
    pub starting_temperature: Option<String>,
}

/// Late acceptance configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LateAcceptanceConfig {
    /// Size of late acceptance list.
    pub late_acceptance_size: Option<usize>,
}

/// Great deluge configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GreatDelugeConfig {
    /// Water level increase ratio.
    pub water_level_increase_ratio: Option<f64>,
}

/// Forager configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ForagerConfig {
    /// Maximum number of accepted moves to consider.
    pub accepted_count_limit: Option<usize>,

    /// Whether to pick early if an improving move is found.
    pub pick_early_type: Option<PickEarlyType>,
}

/// Pick early type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PickEarlyType {
    /// Never pick early.
    #[default]
    Never,

    /// Pick first improving move.
    FirstBestScoreImproving,

    /// Pick first last step score improving move.
    FirstLastStepScoreImproving,
}

/// Move selector configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveSelectorConfig {
    /// Change move selector.
    ChangeMoveSelector(ChangeMoveConfig),

    /// Swap move selector.
    SwapMoveSelector(SwapMoveConfig),

    /// Union of multiple selectors.
    UnionMoveSelector(UnionMoveSelectorConfig),

    /// Cartesian product of selectors.
    CartesianProductMoveSelector(CartesianProductConfig),
}

/// Change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangeMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,
}

/// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,
}

/// Union move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UnionMoveSelectorConfig {
    /// Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

/// Cartesian product move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CartesianProductConfig {
    /// Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

/// Exhaustive search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ExhaustiveSearchConfig {
    /// Exhaustive search type.
    #[serde(default)]
    pub exhaustive_search_type: ExhaustiveSearchType,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Exhaustive search types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExhaustiveSearchType {
    /// Branch and bound.
    #[default]
    BranchAndBound,

    /// Brute force.
    BruteForce,
}

/// Partitioned search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PartitionedSearchConfig {
    /// Number of partitions.
    pub partition_count: Option<usize>,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Custom phase configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CustomPhaseConfig {
    /// Custom phase class name.
    pub custom_phase_class: Option<String>,
}

/// Runtime configuration overrides.
#[derive(Debug, Clone, Default)]
pub struct SolverConfigOverride {
    /// Override termination configuration.
    pub termination: Option<TerminationConfig>,
}

impl SolverConfigOverride {
    /// Creates a new override with termination configuration.
    pub fn with_termination(termination: TerminationConfig) -> Self {
        SolverConfigOverride {
            termination: Some(termination),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_parsing() {
        let toml = r#"
            environment_mode = "reproducible"
            random_seed = 42

            [termination]
            seconds_spent_limit = 30

            [[phases]]
            type = "construction_heuristic"
            construction_heuristic_type = "first_fit_decreasing"

            [[phases]]
            type = "local_search"
            [phases.acceptor]
            type = "late_acceptance"
            late_acceptance_size = 400
        "#;

        let config = SolverConfig::from_toml_str(toml).unwrap();
        assert_eq!(config.environment_mode, EnvironmentMode::Reproducible);
        assert_eq!(config.random_seed, Some(42));
        assert_eq!(config.termination.unwrap().seconds_spent_limit, Some(30));
        assert_eq!(config.phases.len(), 2);
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
            environment_mode: reproducible
            random_seed: 42
            termination:
              seconds_spent_limit: 30
            phases:
              - type: construction_heuristic
                construction_heuristic_type: first_fit_decreasing
              - type: local_search
                acceptor:
                  type: late_acceptance
                  late_acceptance_size: 400
        "#;

        let config = SolverConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.environment_mode, EnvironmentMode::Reproducible);
        assert_eq!(config.random_seed, Some(42));
    }

    #[test]
    fn test_builder() {
        let config = SolverConfig::new()
            .with_random_seed(123)
            .with_termination_seconds(60)
            .with_phase(PhaseConfig::ConstructionHeuristic(
                ConstructionHeuristicConfig::default(),
            ))
            .with_phase(PhaseConfig::LocalSearch(LocalSearchConfig::default()));

        assert_eq!(config.random_seed, Some(123));
        assert_eq!(config.phases.len(), 2);
    }
}
