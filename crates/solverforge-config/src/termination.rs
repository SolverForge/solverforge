use std::time::Duration;

use serde::{Deserialize, Serialize};

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
