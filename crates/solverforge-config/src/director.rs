use serde::{Deserialize, Serialize};

/// Score director configuration: which constraint set a solve scores with.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DirectorConfig {
    /// Name of the constraint provider the solve scores with: the Rust path
    /// of the solution's `constraints` function or of one of its
    /// `constraint_provider` functions, exactly as written in the
    /// `#[planning_solution]` attribute. Omitted: the `constraints` function.
    /// A name that matches no declared provider fails the solve before it
    /// starts.
    pub constraint_provider: Option<String>,

    /// Whether to enable constraint matching assertions. Default: `false`.
    /// Not read by the current solver runtime.
    #[serde(default)]
    pub constraint_match_enabled: bool,
}
