use serde::{Deserialize, Serialize};

/// Score director configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DirectorConfig {
    /// Fully qualified name of the constraint provider type. Not read by the
    /// current solver runtime.
    pub constraint_provider: Option<String>,

    /// Whether to enable constraint matching assertions. Default: `false`.
    /// Not read by the current solver runtime.
    #[serde(default)]
    pub constraint_match_enabled: bool,
}
