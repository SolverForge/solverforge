use serde::{Deserialize, Serialize};

/// How a step chooses among accepted candidates with equal scores.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScoreTieBreak {
    /// Seeded reservoir selection among equally scored candidates. Default.
    #[default]
    Random,
    /// Keep the first equally scored candidate.
    First,
}

/// Forager configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ForagerConfig {
    /// Stop after N accepted candidates and pick the best among them.
    AcceptedCount(AcceptedCountForagerConfig),

    /// Evaluate the full neighborhood and pick the best accepted move.
    BestScore {},

    /// Pick the first accepted move.
    FirstAccepted {},

    /// Stop on the first move improving the phase-best score.
    FirstBestScoreImproving {},

    /// Stop on the first move improving the previous step score.
    FirstLastStepScoreImproving {},
}

/// Accepted-count forager configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AcceptedCountForagerConfig {
    /// Accepted moves collected before the step picks the best of them.
    /// Omitted: `1`.
    pub limit: Option<usize>,
}
