use serde::{Deserialize, Serialize};

/// Score director configuration: which constraint set a solve scores with and
/// whether its snapshots can be analyzed.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DirectorConfig {
    /// Name of the constraint provider the solve scores with: the Rust path
    /// of the solution's `constraints` function or of one of its
    /// `constraint_provider` functions, exactly as written in the
    /// `#[planning_solution]` attribute. Omitted: the `constraints` function.
    /// A name that matches no declared provider fails the solve before it
    /// starts.
    pub constraint_provider: Option<String>,

    /// Whether the solve's snapshots can be analyzed with
    /// `SolverManager::analyze_snapshot`. Default: `true`. When `false`,
    /// snapshot analysis of the job fails with
    /// `SolverManagerError::SnapshotAnalysisDisabled`.
    #[serde(default = "constraint_match_enabled_default")]
    pub constraint_match_enabled: bool,
}

const fn constraint_match_enabled_default() -> bool {
    true
}

impl Default for DirectorConfig {
    fn default() -> Self {
        Self {
            constraint_provider: None,
            constraint_match_enabled: constraint_match_enabled_default(),
        }
    }
}
