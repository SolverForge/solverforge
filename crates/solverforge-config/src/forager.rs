use serde::{Deserialize, Serialize};

// Forager configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ForagerConfig {
    // Stop after N accepted candidates and pick the best among them.
    AcceptedCount(AcceptedCountForagerConfig),

    // Evaluate the full neighborhood and pick the best accepted move.
    BestScore,

    // Pick the first accepted move.
    FirstAccepted,

    // Stop on the first move improving the phase-best score.
    FirstBestScoreImproving,

    // Stop on the first move improving the previous step score.
    FirstLastStepScoreImproving,
}

// Accepted-count forager configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AcceptedCountForagerConfig {
    pub limit: Option<usize>,
}
