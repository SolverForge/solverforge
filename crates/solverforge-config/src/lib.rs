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

    /// Cheapest insertion (greedy with early-pick).
    CheapestInsertion,

    /// Regret insertion (picks move with maximum regret).
    RegretInsertion,

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

    /// Number of consecutive steps without accepted moves before forcing a random move.
    /// When the solver gets stuck with no accepted moves for this many steps,
    /// it will force-accept a random doable move to escape the local optimum.
    /// Default: 100 steps.
    pub stagnation_threshold: Option<u64>,
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
    /// Temperature decay rate per step (0.0 to 1.0, typical: 0.99).
    pub decay_rate: Option<f64>,
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

/// Selection order for move selectors.
///
/// Controls how entities are ordered when iterating through moves.
/// Default is `Original` for deterministic, complete coverage.
/// Use `Shuffled` when you need randomization with guaranteed coverage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOrder {
    /// Sequential order (0, 1, 2...) - deterministic, complete coverage.
    #[default]
    Original,

    /// Shuffle entity list once per step, then iterate sequentially.
    /// Complete coverage with randomization.
    Shuffled,

    /// Random picks on each iteration - incomplete coverage, probabilistic.
    /// Never-ending iterator; forager must enforce termination.
    Random,
}

/// Move selector configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveSelectorConfig {
    // ========================================================================
    // Basic Variable Selectors
    // ========================================================================
    /// Change move selector (basic variables).
    ChangeMoveSelector(ChangeMoveConfig),

    /// Swap move selector (basic variables).
    SwapMoveSelector(SwapMoveConfig),

    /// Pillar change move selector (change all entities with same value).
    PillarChangeMoveSelector(PillarChangeMoveConfig),

    /// Pillar swap move selector (swap between entity groups).
    PillarSwapMoveSelector(PillarSwapMoveConfig),

    /// Ruin move selector (unassign multiple entities for LNS).
    RuinMoveSelector(RuinMoveConfig),

    // ========================================================================
    // List Variable Selectors
    // ========================================================================
    /// List change move selector (list variables - relocate element).
    ListChangeMoveSelector(ListChangeMoveConfig),

    /// List swap move selector (list variables - swap two elements).
    ListSwapMoveSelector(ListSwapMoveConfig),

    /// List reverse move selector (list variables - reverse segment, 2-opt style).
    ListReverseMoveSelector(ListReverseMoveConfig),

    /// K-opt move selector (list variables - tour improvement).
    KOptMoveSelector(KOptMoveConfig),

    /// Sub-list change move selector (list variables - relocate segment).
    SubListChangeMoveSelector(SubListChangeMoveConfig),

    /// Sub-list swap move selector (list variables - swap segments).
    SubListSwapMoveSelector(SubListSwapMoveConfig),

    /// List ruin move selector (remove elements from lists for LNS).
    ListRuinMoveSelector(ListRuinMoveConfig),

    // ========================================================================
    // Composite Selectors
    // ========================================================================
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

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// Pillar change move configuration (change all entities with same value).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PillarChangeMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,

    /// Selection order for pillar iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// Pillar swap move configuration (swap between entity groups).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PillarSwapMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,

    /// Selection order for pillar iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// Ruin move configuration (unassign multiple entities for LNS).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RuinMoveConfig {
    /// Number of entities to unassign per move.
    #[serde(default = "default_ruin_count")]
    pub ruin_count: usize,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

fn default_ruin_count() -> usize {
    3
}

/// List change move configuration (relocate element within/between lists).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListChangeMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// List swap move configuration (swap two elements in lists).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListSwapMoveConfig {
    /// Entity class filter.
    pub entity_class: Option<String>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// List reverse move configuration (reverse segment, 2-opt style).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListReverseMoveConfig {
    /// Minimum segment length to reverse (default: 2).
    pub minimum_segment_length: Option<usize>,
    /// Maximum segment length to reverse (None = entire list).
    pub maximum_segment_length: Option<usize>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// K-opt move configuration (tour improvement for list variables).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct KOptMoveConfig {
    /// K value for k-opt (2 = 2-opt, 3 = 3-opt, etc.).
    #[serde(default = "default_k_value")]
    pub k_value: usize,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

fn default_k_value() -> usize {
    2
}

/// Sub-list change move configuration (relocate contiguous segment).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SubListChangeMoveConfig {
    /// Minimum sub-list size.
    pub minimum_sub_list_size: Option<usize>,
    /// Maximum sub-list size.
    pub maximum_sub_list_size: Option<usize>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// Sub-list swap move configuration (swap two contiguous segments).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SubListSwapMoveConfig {
    /// Minimum sub-list size.
    pub minimum_sub_list_size: Option<usize>,
    /// Maximum sub-list size.
    pub maximum_sub_list_size: Option<usize>,
    /// Whether segments can be from the same list.
    pub select_reverse_movement: Option<bool>,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
}

/// List ruin move configuration (remove elements from lists for LNS).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListRuinMoveConfig {
    /// Number of elements to remove per move.
    #[serde(default = "default_ruin_count")]
    pub ruin_count: usize,

    /// Selection order for entity iteration.
    #[serde(default)]
    pub selection_order: SelectionOrder,
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
