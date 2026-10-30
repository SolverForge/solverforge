use std::num::NonZeroUsize;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::director::DirectorConfig;
use crate::error::ConfigError;
use crate::phase::PhaseConfig;
use crate::termination::TerminationConfig;

/// Main solver configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SolverConfig {
    /// Runtime checking mode. Default: `non_reproducible`.
    #[serde(default)]
    pub environment_mode: EnvironmentMode,

    /// Seed for the solver's random choices. Omitted: every run seeds itself
    /// from the operating system, so runs differ.
    #[serde(default)]
    pub random_seed: Option<u64>,

    /// Number of evaluators for parallel move evaluation in acceptor-forager
    /// local search. Default: `auto`.
    #[serde(default)]
    pub move_thread_count: MoveThreadCount,

    /// Solver-wide termination. Omitted: no solver-wide limit.
    #[serde(default)]
    pub termination: Option<TerminationConfig>,

    /// Score director settings. Parsed and serialized, but not read by the
    /// current solver runtime.
    #[serde(default)]
    pub score_director: Option<DirectorConfig>,

    /// Phases, run in order. Omitted or empty: the model-aware default
    /// profile.
    #[serde(default)]
    pub phases: Vec<PhaseConfig>,

    /// Optional bounded candidate-pull trace for cross-runtime diagnostics.
    ///
    /// This is deliberately opt-in: recording a trace retains one owned
    /// identity per engine-consumed candidate.  A non-zero cap is required so
    /// diagnostic runs cannot accidentally retain an unbounded neighborhood.
    #[serde(default)]
    pub candidate_trace: Option<CandidateTraceConfig>,
}

impl SolverConfig {
    /// Creates an empty configuration; equivalent to `SolverConfig::default()`.
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

    /// Sets `termination.seconds_spent_limit`, keeping other termination keys.
    pub fn with_termination_seconds(mut self, seconds: u64) -> Self {
        self.termination = Some(TerminationConfig {
            seconds_spent_limit: Some(seconds),
            ..self.termination.unwrap_or_default()
        });
        self
    }

    /// Sets `random_seed`.
    pub fn with_random_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    /// Appends a phase.
    pub fn with_phase(mut self, phase: PhaseConfig) -> Self {
        self.phases.push(phase);
        self
    }

    /// Returns a deterministic, complete representation of this effective
    /// configuration for diagnostic provenance.
    ///
    /// The representation is intentionally produced by the same serde model
    /// used to load solver configuration.  Consumers compare this exact value
    /// together with the resolved phase plan in the candidate trace header.
    pub fn canonical_toml(&self) -> String {
        toml::to_string(self)
            .expect("SolverConfig serialization must succeed for a serde-derived configuration")
    }

    /// Returns the canonical solver-document representation of one phase.
    ///
    /// Runtime provenance uses this rather than debug formatting so every
    /// configured selector, acceptor, forager, target, and termination field
    /// participates in the candidate-trace plan digest.
    pub fn canonical_phase_toml(phase: &PhaseConfig) -> String {
        Self {
            phases: vec![phase.clone()],
            ..Self::default()
        }
        .canonical_toml()
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

/// Opt-in bounded candidate-pull trace configuration.
///
/// Enable from TOML with, for example:
///
/// ```toml
/// [candidate_trace]
/// max_entries = 100_000
/// ```
///
/// A zero cap is rejected during deserialization.  The trace records its
/// total pull count even after the cap, marks itself truncated, and never
/// retains identities beyond this limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CandidateTraceConfig {
    /// Most candidate identities retained; zero is rejected. Required.
    pub max_entries: NonZeroUsize,
}

impl CandidateTraceConfig {
    /// Creates a trace configuration retaining at most `max_entries`
    /// identities.
    pub const fn new(max_entries: NonZeroUsize) -> Self {
        Self { max_entries }
    }
}

/// Environment mode affecting solver behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentMode {
    /// No extra runtime checks. Default.
    #[default]
    NonReproducible,

    /// Currently runs exactly like `non_reproducible`; repeatable runs come
    /// from setting `random_seed`.
    Reproducible,

    /// Currently runs exactly like `non_reproducible`.
    FastAssert,

    /// On score calculation and best-solution updates, asserts that the
    /// incremental score equals a fresh full calculation, panicking on drift.
    FullAssert,
}

/// Move thread count configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveThreadCount {
    /// Sequential until benchmarks set an automatic policy.
    #[default]
    Auto,

    /// No parallel move evaluation.
    None,

    /// Evaluator count, main director included; bounded by the solve's
    /// Rayon threads and by one 32-candidate evaluation window.
    Count(usize),
}

/// Runtime configuration overrides.
#[derive(Debug, Clone, Default)]
pub struct SolverConfigOverride {
    /// Override termination configuration.
    pub termination: Option<TerminationConfig>,
}

impl SolverConfigOverride {
    /// Creates an override that replaces the solver's termination
    /// configuration.
    pub fn with_termination(termination: TerminationConfig) -> Self {
        SolverConfigOverride {
            termination: Some(termination),
        }
    }
}
