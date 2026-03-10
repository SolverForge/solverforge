use serde::{Deserialize, Serialize};

/// Score director configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DirectorConfig {
    /// Fully qualified name of the constraint provider type.
    pub constraint_provider: Option<String>,

    /// Whether to enable constraint matching assertions.
    #[serde(default)]
    pub constraint_match_enabled: bool,
}
