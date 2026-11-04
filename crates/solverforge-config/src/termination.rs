use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Termination configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct TerminationConfig {
    /// Maximum seconds to spend solving; added to `minutes_spent_limit`.
    pub seconds_spent_limit: Option<u64>,

    /// Maximum minutes to spend solving; added to `seconds_spent_limit`.
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
    /// Sum of `seconds_spent_limit` and `minutes_spent_limit`; `None` when
    /// neither is set. Zero is a limit of zero; a sum past `Duration::MAX`
    /// saturates there.
    pub fn time_limit(&self) -> Option<Duration> {
        if self.seconds_spent_limit.is_none() && self.minutes_spent_limit.is_none() {
            return None;
        }
        let seconds = Duration::from_secs(self.seconds_spent_limit.unwrap_or(0));
        let minutes = Duration::from_secs(self.minutes_spent_limit.unwrap_or(0)).saturating_mul(60);
        Some(seconds.saturating_add(minutes))
    }

    /// `unimproved_seconds_spent_limit` as a `Duration`.
    pub fn unimproved_time_limit(&self) -> Option<Duration> {
        self.unimproved_seconds_spent_limit.map(Duration::from_secs)
    }
}
