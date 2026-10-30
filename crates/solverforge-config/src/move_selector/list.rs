use super::*;

/// Configuration for `ListChangeMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Configuration for `NearbyListChangeMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbyListChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Maximum nearby destination positions to consider per source element.
    /// Required in TOML;
    /// `Default::default()` uses `10`.
    pub max_nearby: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyListChangeMoveConfig {
    fn default() -> Self {
        let (max_nearby, target) = default_nearby_target_config();
        Self {
            selection_order: None,
            selection_metric: None,
            max_nearby,
            target,
        }
    }
}

/// Configuration for `ListSwapMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListSwapMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Configuration for `ListPermuteMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListPermuteMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Minimum window size (inclusive). Required in TOML;
    /// `Default::default()` uses `2`.
    pub min_window_size: usize,
    /// Maximum window size (inclusive). Required in TOML;
    /// `Default::default()` uses `5`.
    pub max_window_size: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for ListPermuteMoveConfig {
    fn default() -> Self {
        Self {
            selection_order: None,
            selection_metric: None,
            min_window_size: 2,
            max_window_size: 5,
            target: VariableTargetConfig::default(),
        }
    }
}

/// Configuration for `ListPrecedenceMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListPrecedenceMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Configuration for `NearbyListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbyListSwapMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Maximum nearby swap partners to consider per source element.
    /// Required in TOML;
    /// `Default::default()` uses `10`.
    pub max_nearby: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyListSwapMoveConfig {
    fn default() -> Self {
        let (max_nearby, target) = default_nearby_target_config();
        Self {
            selection_order: None,
            selection_metric: None,
            max_nearby,
            target,
        }
    }
}

/// Configuration for `SublistChangeMoveSelector` (Or-opt).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SublistChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Minimum segment size (inclusive). Required in TOML;
    /// `Default::default()` uses `1`.
    pub min_sublist_size: usize,
    /// Maximum segment size (inclusive). Required in TOML;
    /// `Default::default()` uses `3`.
    pub max_sublist_size: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for SublistChangeMoveConfig {
    fn default() -> Self {
        let (min_sublist_size, max_sublist_size, target) = default_sublist_target_config();
        Self {
            selection_order: None,
            selection_metric: None,
            min_sublist_size,
            max_sublist_size,
            target,
        }
    }
}

/// Configuration for `SublistSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SublistSwapMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Minimum segment size (inclusive). Required in TOML;
    /// `Default::default()` uses `1`.
    pub min_sublist_size: usize,
    /// Maximum segment size (inclusive). Required in TOML;
    /// `Default::default()` uses `3`.
    pub max_sublist_size: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for SublistSwapMoveConfig {
    fn default() -> Self {
        let (min_sublist_size, max_sublist_size, target) = default_sublist_target_config();
        Self {
            selection_order: None,
            selection_metric: None,
            min_sublist_size,
            max_sublist_size,
            target,
        }
    }
}

/// Configuration for `ListReverseMoveSelector` (2-opt).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListReverseMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Configuration for `KOptMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct KOptMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Number of cuts, from 2 to 5. Required in TOML;
    /// `Default::default()` uses `3`.
    pub k: usize,
    /// Minimum segment length between cuts. Required in TOML;
    /// `Default::default()` uses `1`.
    pub min_segment_len: usize,
    /// Maximum nearby positions to consider per cut; `0` enumerates every cut
    /// and a positive value uses distance-pruned nearby k-opt candidates.
    /// Required in TOML;
    /// `Default::default()` uses `0`.
    pub max_nearby: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for KOptMoveSelectorConfig {
    fn default() -> Self {
        Self {
            selection_order: None,
            selection_metric: None,
            k: 3,
            min_segment_len: 1,
            max_nearby: 0,
            target: VariableTargetConfig::default(),
        }
    }
}

/// Configuration for `ListRuinMoveSelector` (LNS).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListRuinMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Minimum number of elements to ruin per move. Required in TOML;
    /// `Default::default()` uses `2`.
    pub min_ruin_count: usize,
    /// Maximum number of elements to ruin per move. Required in TOML;
    /// `Default::default()` uses `5`.
    pub max_ruin_count: usize,
    /// Number of ruin moves to generate per step. Omitted: `10`.
    pub moves_per_step: Option<usize>,
    /// Optional maximum source list length eligible for this selector.
    pub max_source_list_len: Option<usize>,
    /// Whether recreate should skip currently empty destination lists.
    #[serde(default)]
    pub skip_empty_destinations: bool,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for ListRuinMoveSelectorConfig {
    fn default() -> Self {
        Self {
            selection_order: None,
            selection_metric: None,
            min_ruin_count: 2,
            max_ruin_count: 5,
            moves_per_step: None,
            max_source_list_len: None,
            skip_empty_destinations: false,
            target: VariableTargetConfig::default(),
        }
    }
}
