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
    /// from the operating system under `non_reproducible`, and uses
    /// [`REPRODUCIBLE_RANDOM_SEED`] under every other environment mode. Read
    /// it through [`SolverConfig::resolved_random_seed`].
    #[serde(default)]
    pub random_seed: Option<u64>,

    /// Number of evaluators for parallel move evaluation in acceptor-forager
    /// local search. Default: `auto`.
    #[serde(default)]
    pub move_thread_count: MoveThreadCount,

    /// Solver-wide termination. Omitted: no solver-wide limit.
    #[serde(default)]
    pub termination: Option<TerminationConfig>,

    /// Score director settings: the constraint provider and whether snapshot
    /// analysis is enabled. Omitted: the solution's `constraints` function,
    /// with snapshot analysis enabled.
    #[serde(default)]
    pub score_director: Option<DirectorConfig>,

    /// Phases, run in order. Omitted or empty: the model-aware default
    /// profile, which is construction followed by the default local search
    /// when `termination` has an effective limit, and construction only
    /// otherwise, because that local search would never end.
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

    /// The configured `score_director.constraint_provider` name, or `None`
    /// for the solution's `constraints` function.
    pub fn constraint_provider(&self) -> Option<&str> {
        self.score_director
            .as_ref()
            .and_then(|director| director.constraint_provider.as_deref())
    }

    /// The configured `score_director.constraint_match_enabled`; `true` when
    /// the section is omitted.
    pub fn constraint_match_enabled(&self) -> bool {
        self.score_director
            .as_ref()
            .is_none_or(|director| director.constraint_match_enabled)
    }

    /// This configuration with `overrides` applied: each field the override
    /// sets replaces the configured one.
    pub fn with_override(mut self, overrides: &SolverConfigOverride) -> Self {
        if let Some(termination) = &overrides.termination {
            self.termination = Some(termination.clone());
        }
        self
    }

    /// The seed the solve uses: `random_seed`, or [`REPRODUCIBLE_RANDOM_SEED`]
    /// when it is omitted under any mode but `non_reproducible`. `None` means
    /// a seed from the operating system.
    pub fn resolved_random_seed(&self) -> Option<u64> {
        match (self.random_seed, self.environment_mode) {
            (Some(seed), _) => Some(seed),
            (None, EnvironmentMode::NonReproducible) => None,
            (None, _) => Some(REPRODUCIBLE_RANDOM_SEED),
        }
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
    /// No runtime checks. Without `random_seed`, every run seeds itself from
    /// the operating system, so runs differ. Default.
    #[default]
    NonReproducible,

    /// No runtime checks. Without `random_seed`, the solve uses
    /// [`REPRODUCIBLE_RANDOM_SEED`], so runs with the same input,
    /// configuration and step or move-count limits produce the same result.
    /// Time limits still end runs at different points.
    Reproducible,

    /// Like `reproducible`, and at every committed step and best-solution
    /// update asserts that the incremental score equals a fresh full
    /// calculation, panicking on drift.
    FastAssert,

    /// Like `fast_assert`, and every score calculation of the score director,
    /// including every evaluated move on the main director and on evaluator
    /// replicas, is checked against a fresh full calculation.
    FullAssert,
}

/// The seed a solve without `random_seed` uses in every mode except
/// `non_reproducible`.
pub const REPRODUCIBLE_RANDOM_SEED: u64 = 0;

impl EnvironmentMode {
    /// Whether committed steps and best-solution updates are checked against a
    /// fresh score: `fast_assert` and `full_assert`.
    pub const fn asserts_steps(self) -> bool {
        matches!(self, Self::FastAssert | Self::FullAssert)
    }

    /// Whether every score calculation is checked against a fresh score:
    /// `full_assert`.
    pub const fn asserts_every_score_calculation(self) -> bool {
        matches!(self, Self::FullAssert)
    }
}

/// Move thread count configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveThreadCount {
    /// Per step, sequential evaluation or parallel windows, whichever the
    /// measured trial, window and replica costs say is faster; up to four
    /// evaluators, bounded by the solve's Rayon threads. Results are the same
    /// either way: parallel evaluation reaches the sequential trajectory.
    /// Default.
    #[default]
    Auto,

    /// No parallel move evaluation.
    None,

    /// Evaluator count, main director included; bounded by the solve's
    /// Rayon threads and by one 32-candidate evaluation window. Zero is
    /// rejected when the configuration is read.
    Count(NonZeroUsize),
}

/// Runtime configuration overrides, applied with
/// [`SolverConfig::with_override`].
#[derive(Debug, Clone, Default)]
pub struct SolverConfigOverride {
    /// When set, replaces the solver-level `termination`. Phase-level
    /// terminations are unchanged.
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
