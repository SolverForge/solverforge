use super::*;

// Configuration for `ListChangeMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListChangeMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbyListChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Maximum nearby destination positions to consider per source element.
    pub max_nearby: usize,
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

// Configuration for `ListSwapMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListSwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `ListPermuteMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListPermuteMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Minimum window size (inclusive). Default: 2.
    pub min_window_size: usize,
    // Maximum window size (inclusive). Default: 5.
    pub max_window_size: usize,
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

// Configuration for `ListPrecedenceMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListPrecedenceMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbyListSwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Maximum nearby swap partners to consider per source element.
    pub max_nearby: usize,
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

// Configuration for `SublistChangeMoveSelector` (Or-opt).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SublistChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
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

// Configuration for `SublistSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SublistSwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
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

// Configuration for `ListReverseMoveSelector` (2-opt).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListReverseMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `KOptMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct KOptMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // K value (number of cuts). Default: 3.
    pub k: usize,
    // Minimum segment length between cuts. Default: 1.
    pub min_segment_len: usize,
    // Maximum nearby positions to consider per cut. Default: 0 (full enumeration).
    // When > 0, uses distance-pruned nearby k-opt candidates instead of full enumeration.
    pub max_nearby: usize,
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

// Configuration for `ListRuinMoveSelector` (LNS).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListRuinMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    // Minimum number of elements to ruin per move. Default: 2.
    pub min_ruin_count: usize,
    // Maximum number of elements to ruin per move. Default: 5.
    pub max_ruin_count: usize,
    // Number of ruin moves to generate per step. Default: 10.
    pub moves_per_step: Option<usize>,
    // Optional maximum source list length eligible for this selector.
    pub max_source_list_len: Option<usize>,
    // Whether recreate should skip currently empty destination lists.
    #[serde(default)]
    pub skip_empty_destinations: bool,
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
