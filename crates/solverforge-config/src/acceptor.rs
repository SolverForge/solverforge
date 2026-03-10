use serde::{Deserialize, Serialize};

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
