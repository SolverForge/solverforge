use serde::{Deserialize, Serialize};

// Move selector configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveSelectorConfig {
    // Change move selector (standard variables).
    ChangeMoveSelector(ChangeMoveConfig),

    // Swap move selector (standard variables).
    SwapMoveSelector(SwapMoveConfig),

    // List change move selector — relocates single elements within/between routes.
    ListChangeMoveSelector(ListChangeMoveConfig),

    // Nearby list change move selector — distance-pruned element relocation.
    NearbyListChangeMoveSelector(NearbyListChangeMoveConfig),

    // List swap move selector — swaps single elements within/between routes.
    ListSwapMoveSelector(ListSwapMoveConfig),

    // Nearby list swap move selector — distance-pruned element swap.
    NearbyListSwapMoveSelector(NearbyListSwapMoveConfig),

    // Sublist change move selector (Or-opt) — relocates contiguous segments.
    SubListChangeMoveSelector(SubListChangeMoveConfig),

    // Sublist swap move selector — swaps contiguous segments between routes.
    SubListSwapMoveSelector(SubListSwapMoveConfig),

    // List reverse move selector (2-opt) — reverses segments within a route.
    ListReverseMoveSelector(ListReverseMoveConfig),

    // K-opt move selector — generalised route reconnection.
    KOptMoveSelector(KOptMoveSelectorConfig),

    // List ruin move selector (LNS) — removes elements for reinsertion.
    ListRuinMoveSelector(ListRuinMoveSelectorConfig),

    // Union of multiple selectors.
    UnionMoveSelector(UnionMoveSelectorConfig),

    // Cartesian product of selectors.
    CartesianProductMoveSelector(CartesianProductConfig),
}

// Change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangeMoveConfig {
    // Entity class filter.
    pub entity_class: Option<String>,
}

// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMoveConfig {
    // Entity class filter.
    pub entity_class: Option<String>,
}

// Configuration for `ListChangeMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListChangeMoveConfig {
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

// Configuration for `NearbyListChangeMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyListChangeMoveConfig {
    // Maximum nearby destination positions to consider per source element.
    pub max_nearby: usize,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for NearbyListChangeMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            variable_name: None,
        }
    }
}

// Configuration for `ListSwapMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListSwapMoveConfig {
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

// Configuration for `NearbyListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyListSwapMoveConfig {
    // Maximum nearby swap partners to consider per source element.
    pub max_nearby: usize,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for NearbyListSwapMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            variable_name: None,
        }
    }
}

// Configuration for `SubListChangeMoveSelector` (Or-opt).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SubListChangeMoveConfig {
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for SubListChangeMoveConfig {
    fn default() -> Self {
        Self {
            min_sublist_size: 1,
            max_sublist_size: 3,
            variable_name: None,
        }
    }
}

// Configuration for `SubListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SubListSwapMoveConfig {
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for SubListSwapMoveConfig {
    fn default() -> Self {
        Self {
            min_sublist_size: 1,
            max_sublist_size: 3,
            variable_name: None,
        }
    }
}

// Configuration for `ListReverseMoveSelector` (2-opt).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListReverseMoveConfig {
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

// Configuration for `KOptMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct KOptMoveSelectorConfig {
    // K value (number of cuts). Default: 3.
    pub k: usize,
    // Minimum segment length between cuts. Default: 1.
    pub min_segment_len: usize,
    // Maximum nearby positions to consider per cut. Default: 0 (full enumeration).
    // When > 0, uses distance-pruned NearbyKOptMoveSelector instead of full KOptMoveSelector.
    pub max_nearby: usize,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for KOptMoveSelectorConfig {
    fn default() -> Self {
        Self {
            k: 3,
            min_segment_len: 1,
            max_nearby: 0,
            variable_name: None,
        }
    }
}

// Configuration for `ListRuinMoveSelector` (LNS).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListRuinMoveSelectorConfig {
    // Minimum number of elements to ruin per move. Default: 2.
    pub min_ruin_count: usize,
    // Maximum number of elements to ruin per move. Default: 5.
    pub max_ruin_count: usize,
    // Number of ruin moves to generate per step. Default: 10.
    pub moves_per_step: Option<usize>,
    // Variable name filter. If None, applies to all list variables.
    pub variable_name: Option<String>,
}

impl Default for ListRuinMoveSelectorConfig {
    fn default() -> Self {
        Self {
            min_ruin_count: 2,
            max_ruin_count: 5,
            moves_per_step: None,
            variable_name: None,
        }
    }
}

// Union move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UnionMoveSelectorConfig {
    // Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

// Cartesian product move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CartesianProductConfig {
    // Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}
