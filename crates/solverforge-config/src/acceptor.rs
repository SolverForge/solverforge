use serde::{Deserialize, Serialize};

// Acceptor configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcceptorConfig {
    // Hill climbing (only accept improving moves).
    HillClimbing,

    // Step counting hill climbing (allow limited non-improving moves).
    StepCountingHillClimbing(StepCountingHillClimbingConfig),

    // Tabu search acceptor.
    TabuSearch(TabuSearchConfig),

    // Simulated annealing acceptor.
    SimulatedAnnealing(SimulatedAnnealingConfig),

    // Late acceptance acceptor.
    LateAcceptance(LateAcceptanceConfig),

    // Diversified late acceptance acceptor.
    DiversifiedLateAcceptance(DiversifiedLateAcceptanceConfig),

    // Great deluge acceptor.
    GreatDeluge(GreatDelugeConfig),
}

// Step counting hill climbing configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StepCountingHillClimbingConfig {
    pub step_count_limit: Option<u64>,
}

// Tabu search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TabuSearchConfig {
    // Size of entity tabu list. Explicit `0` is invalid.
    pub entity_tabu_size: Option<usize>,

    // Size of value tabu list. Explicit `0` is invalid.
    pub value_tabu_size: Option<usize>,

    // Size of move tabu list. Explicit `0` is invalid.
    pub move_tabu_size: Option<usize>,

    // Size of undo move tabu list. Explicit `0` is invalid.
    pub undo_move_tabu_size: Option<usize>,

    // Whether aspiration can override tabu on strict new-best candidates.
    // When all sizes are omitted, the canonical runtime normalizes to
    // move-tabu-only with `move_tabu_size = 10` and `aspiration_enabled = true`.
    pub aspiration_enabled: Option<bool>,
}

// Simulated annealing configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SimulatedAnnealingConfig {
    // Starting temperature.
    pub starting_temperature: Option<String>,

    // Decay rate.
    pub decay_rate: Option<f64>,
}

// Late acceptance configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LateAcceptanceConfig {
    // Size of late acceptance list.
    pub late_acceptance_size: Option<usize>,
}

// Diversified late acceptance configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DiversifiedLateAcceptanceConfig {
    // Size of late acceptance list.
    pub late_acceptance_size: Option<usize>,

    // Fractional tolerance against the phase-best score.
    pub tolerance: Option<f64>,
}

// Great deluge configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GreatDelugeConfig {
    // Water level increase ratio.
    pub water_level_increase_ratio: Option<f64>,
}
