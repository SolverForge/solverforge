use serde::{Deserialize, Serialize};

// Forager configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ForagerConfig {
    // Maximum number of accepted moves to consider.
    pub accepted_count_limit: Option<usize>,

    // Whether to pick early if an improving move is found.
    pub pick_early_type: Option<PickEarlyType>,
}

// Pick early type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PickEarlyType {
    // Never pick early.
    #[default]
    Never,

    // Pick first improving move.
    FirstBestScoreImproving,

    // Pick first last step score improving move.
    FirstLastStepScoreImproving,
}
