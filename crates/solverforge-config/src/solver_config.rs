use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::director::DirectorConfig;
use crate::error::ConfigError;
use crate::phase::PhaseConfig;
use crate::termination::TerminationConfig;

// Main solver configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SolverConfig {
    // Environment mode affecting reproducibility and assertions.
    #[serde(default)]
    pub environment_mode: EnvironmentMode,

    // Random seed for reproducible results.
    #[serde(default)]
    pub random_seed: Option<u64>,

    // Number of threads for parallel move evaluation.
    #[serde(default)]
    pub move_thread_count: MoveThreadCount,

    // Termination configuration.
    #[serde(default)]
    pub termination: Option<TerminationConfig>,

    // Score director configuration.
    #[serde(default)]
    pub score_director: Option<DirectorConfig>,

    // Phase configurations.
    #[serde(default)]
    pub phases: Vec<PhaseConfig>,
}

impl SolverConfig {
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

    pub fn with_termination_seconds(mut self, seconds: u64) -> Self {
        self.termination = Some(TerminationConfig {
            seconds_spent_limit: Some(seconds),
            ..self.termination.unwrap_or_default()
        });
        self
    }

    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

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

// Environment mode affecting solver behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentMode {
    // Non-reproducible mode with minimal overhead.
    #[default]
    NonReproducible,

    // Reproducible mode with deterministic behavior.
    Reproducible,

    // Fast assert mode with basic assertions.
    FastAssert,

    // Full assert mode with comprehensive assertions.
    FullAssert,
}

// Move thread count configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveThreadCount {
    // Automatically determine thread count.
    #[default]
    Auto,

    // No parallel move evaluation.
    None,

    // Specific number of threads.
    Count(usize),
}

// Runtime configuration overrides.
#[derive(Debug, Clone, Default)]
pub struct SolverConfigOverride {
    // Override termination configuration.
    pub termination: Option<TerminationConfig>,
}

impl SolverConfigOverride {
    pub fn with_termination(termination: TerminationConfig) -> Self {
        SolverConfigOverride {
            termination: Some(termination),
        }
    }
}
