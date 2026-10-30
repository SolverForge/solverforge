use serde::{Deserialize, Serialize};

/// Acceptor configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcceptorConfig {
    /// Hill climbing (only accept improving moves).
    HillClimbing {},

    /// Step counting hill climbing (allow limited non-improving moves).
    StepCountingHillClimbing(StepCountingHillClimbingConfig),

    /// Tabu search acceptor.
    TabuSearch(TabuSearchConfig),

    /// Simulated annealing acceptor.
    SimulatedAnnealing(SimulatedAnnealingConfig),

    /// Late acceptance acceptor.
    LateAcceptance(LateAcceptanceConfig),

    /// Diversified late acceptance acceptor.
    DiversifiedLateAcceptance(DiversifiedLateAcceptanceConfig),

    /// Great deluge acceptor.
    GreatDeluge(GreatDelugeConfig),
}

/// Step counting hill climbing configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepCountingHillClimbingConfig {
    /// Steps a non-improving move is accepted for after the last improvement.
    /// Omitted: `100`.
    pub step_count_limit: Option<u64>,
}

/// Tabu search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TabuSearchConfig {
    /// Size of entity tabu list. Explicit `0` is invalid.
    pub entity_tabu_size: Option<usize>,

    /// Size of value tabu list. Explicit `0` is invalid.
    pub value_tabu_size: Option<usize>,

    /// Size of move tabu list. Explicit `0` is invalid.
    pub move_tabu_size: Option<usize>,

    /// Size of undo move tabu list. Explicit `0` is invalid.
    pub undo_move_tabu_size: Option<usize>,

    /// Whether aspiration can override tabu on strict new-best candidates.
    /// Omitted: `true`. When every size is omitted, the runtime uses move tabu
    /// only, with `move_tabu_size = 10`.
    pub aspiration_enabled: Option<bool>,
}

/// Simulated annealing configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SimulatedAnnealingConfig {
    /// Per-score-level starting temperatures, highest priority level first.
    /// Omitted: calibrated from sampled moves, see `calibration`.
    pub level_temperatures: Option<Vec<f64>>,

    /// Factor every temperature is multiplied by after each step; must be in
    /// `(0, 1]`. Omitted: `0.999985`.
    pub decay_rate: Option<f64>,

    /// Temperature at or below which worsening moves are rejected
    /// deterministically. Omitted: `1e-9`.
    pub hill_climbing_temperature: Option<f64>,

    /// Hard-regression policy. Omitted: `temperature_controlled`.
    pub hard_regression_policy: Option<HardRegressionPolicyConfig>,

    /// Sampled auto-calibration settings used when level_temperatures is omitted.
    pub calibration: Option<SimulatedAnnealingCalibrationConfig>,
}

/// Whether simulated annealing may accept a move that worsens a hard level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HardRegressionPolicyConfig {
    /// Hard regressions are accepted with the same temperature rule as any
    /// other level. Default.
    #[default]
    TemperatureControlled,
    /// A move whose first differing level is a worse hard level is always
    /// rejected.
    NeverAcceptHardRegression,
}

/// Settings for sampling the starting temperatures when
/// `level_temperatures` is omitted.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SimulatedAnnealingCalibrationConfig {
    /// Worsening moves sampled before the temperatures are set. Omitted: `128`.
    pub sample_size: Option<usize>,
    /// Probability with which an average sampled worsening move is accepted at
    /// the calibrated temperature. Omitted: `0.8`.
    pub target_acceptance_probability: Option<f64>,
    /// Temperature for a level without samples, and the lower bound of every
    /// calibrated temperature. Omitted: `1.0`.
    pub fallback_temperature: Option<f64>,
}

/// Late acceptance configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LateAcceptanceConfig {
    /// Number of past step scores a move is compared against. Omitted: `400`.
    pub late_acceptance_size: Option<usize>,
}

/// Diversified late acceptance configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DiversifiedLateAcceptanceConfig {
    /// Number of past step scores a move is compared against. Omitted: `400`.
    pub late_acceptance_size: Option<usize>,
}

/// Great deluge configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GreatDelugeConfig {
    /// Fraction of the initial score's magnitude the water level rises per
    /// step. Omitted: `0.001`.
    pub water_level_increase_ratio: Option<f64>,
}
