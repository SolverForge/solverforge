use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct VariableTargetConfig {
    pub entity_class: Option<String>,
    pub variable_name: Option<String>,
}

// Move selector configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveSelectorConfig {
    // Change move selector (scalar variables).
    ChangeMoveSelector(ChangeMoveConfig),

    // Swap move selector (scalar variables).
    SwapMoveSelector(SwapMoveConfig),

    // Nearby change move selector (scalar variables).
    NearbyChangeMoveSelector(NearbyChangeMoveConfig),

    // Nearby swap move selector (scalar variables).
    NearbySwapMoveSelector(NearbySwapMoveConfig),

    // Pillar change move selector (scalar variables).
    PillarChangeMoveSelector(PillarChangeMoveConfig),

    // Pillar swap move selector (scalar variables).
    PillarSwapMoveSelector(PillarSwapMoveConfig),

    // Ruin-and-recreate move selector (scalar variables).
    RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig),

    // List change move selector — relocates single elements within/between routes.
    ListChangeMoveSelector(ListChangeMoveConfig),

    // Nearby list change move selector — distance-pruned element relocation.
    NearbyListChangeMoveSelector(NearbyListChangeMoveConfig),

    // List swap move selector — swaps single elements within/between routes.
    ListSwapMoveSelector(ListSwapMoveConfig),

    // Nearby list swap move selector — distance-pruned element swap.
    NearbyListSwapMoveSelector(NearbyListSwapMoveConfig),

    // Sublist change move selector (Or-opt) — relocates contiguous segments.
    SublistChangeMoveSelector(SublistChangeMoveConfig),

    // Sublist swap move selector — swaps contiguous segments between routes.
    SublistSwapMoveSelector(SublistSwapMoveConfig),

    // List reverse move selector (2-opt) — reverses segments within a route.
    ListReverseMoveSelector(ListReverseMoveConfig),

    // K-opt move selector — generalised route reconnection.
    KOptMoveSelector(KOptMoveSelectorConfig),

    // List ruin move selector (LNS) — removes elements for reinsertion.
    ListRuinMoveSelector(ListRuinMoveSelectorConfig),

    // Neighborhood that limits the number of yielded candidates from a child selector while
    // preserving selector order.
    LimitedNeighborhood(LimitedNeighborhoodConfig),

    // Union of multiple selectors.
    UnionMoveSelector(UnionMoveSelectorConfig),

    // Cartesian product of selectors. Evaluates the right child on the left preview state,
    // composes tabu ids in selector order, and rejects left children that require full score
    // evaluation during preview.
    CartesianProductMoveSelector(CartesianProductConfig),
}

// Change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangeMoveConfig {
    pub value_candidate_limit: Option<usize>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMoveConfig {
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Nearby change move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyChangeMoveConfig {
    pub max_nearby: usize,
    pub value_candidate_limit: Option<usize>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyChangeMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            value_candidate_limit: None,
            target: VariableTargetConfig::default(),
        }
    }
}

// Nearby swap move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbySwapMoveConfig {
    pub max_nearby: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbySwapMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            target: VariableTargetConfig::default(),
        }
    }
}

// Pillar change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PillarChangeMoveConfig {
    pub minimum_sub_pillar_size: usize,
    pub maximum_sub_pillar_size: usize,
    pub value_candidate_limit: Option<usize>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Pillar swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PillarSwapMoveConfig {
    pub minimum_sub_pillar_size: usize,
    pub maximum_sub_pillar_size: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecreateHeuristicType {
    #[default]
    FirstFit,
    CheapestInsertion,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnionSelectionOrder {
    #[default]
    Sequential,
    RoundRobin,
}

// Ruin-and-recreate move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RuinRecreateMoveSelectorConfig {
    pub min_ruin_count: usize,
    pub max_ruin_count: usize,
    pub moves_per_step: Option<usize>,
    pub value_candidate_limit: Option<usize>,
    pub recreate_heuristic_type: RecreateHeuristicType,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for RuinRecreateMoveSelectorConfig {
    fn default() -> Self {
        Self {
            min_ruin_count: 2,
            max_ruin_count: 5,
            moves_per_step: None,
            value_candidate_limit: None,
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `ListChangeMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListChangeMoveConfig {
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListChangeMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyListChangeMoveConfig {
    // Maximum nearby destination positions to consider per source element.
    pub max_nearby: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyListChangeMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `ListSwapMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListSwapMoveConfig {
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyListSwapMoveConfig {
    // Maximum nearby swap partners to consider per source element.
    pub max_nearby: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyListSwapMoveConfig {
    fn default() -> Self {
        Self {
            max_nearby: 10,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `SublistChangeMoveSelector` (Or-opt).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SublistChangeMoveConfig {
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for SublistChangeMoveConfig {
    fn default() -> Self {
        Self {
            min_sublist_size: 1,
            max_sublist_size: 3,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `SublistSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SublistSwapMoveConfig {
    // Minimum segment size (inclusive). Default: 1.
    pub min_sublist_size: usize,
    // Maximum segment size (inclusive). Default: 3.
    pub max_sublist_size: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for SublistSwapMoveConfig {
    fn default() -> Self {
        Self {
            min_sublist_size: 1,
            max_sublist_size: 3,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `ListReverseMoveSelector` (2-opt).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListReverseMoveConfig {
    #[serde(flatten)]
    pub target: VariableTargetConfig,
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
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for KOptMoveSelectorConfig {
    fn default() -> Self {
        Self {
            k: 3,
            min_segment_len: 1,
            max_nearby: 0,
            target: VariableTargetConfig::default(),
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
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for ListRuinMoveSelectorConfig {
    fn default() -> Self {
        Self {
            min_ruin_count: 2,
            max_ruin_count: 5,
            moves_per_step: None,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `LimitedNeighborhood`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LimitedNeighborhoodConfig {
    // Maximum number of moves yielded from the child selector.
    pub selected_count_limit: usize,
    // Child selector to wrap.
    pub selector: Box<MoveSelectorConfig>,
}

// Union move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UnionMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: UnionSelectionOrder,
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
