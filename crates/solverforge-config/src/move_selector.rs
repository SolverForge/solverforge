use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct VariableTargetConfig {
    pub entity_class: Option<String>,
    pub variable_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOrder {
    #[default]
    Original,
    Random,
    Shuffled,
    Sorted,
    Probabilistic,
}

impl SelectionOrder {
    pub const fn is_random(self) -> bool {
        matches!(self, Self::Random | Self::Shuffled | Self::Probabilistic)
    }

    pub const fn requires_complete_stream(self) -> bool {
        matches!(self, Self::Sorted | Self::Probabilistic)
    }
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

    // Atomic grouped scalar move selector.
    GroupedScalarMoveSelector(GroupedScalarMoveSelectorConfig),

    // List change move selector — relocates single elements within/between routes.
    ListChangeMoveSelector(ListChangeMoveConfig),

    // Nearby list change move selector — distance-pruned element relocation.
    NearbyListChangeMoveSelector(NearbyListChangeMoveConfig),

    // List swap move selector — swaps single elements within/between routes.
    ListSwapMoveSelector(ListSwapMoveConfig),

    // List permute move selector — permutes contiguous windows within routes.
    ListPermuteMoveSelector(ListPermuteMoveConfig),

    // List precedence move selector — prioritizes critical list arcs in precedence makespan models.
    ListPrecedenceMoveSelector(ListPrecedenceMoveConfig),

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

    // Conflict-directed scalar repair selector.
    ConflictRepairMoveSelector(ConflictRepairMoveSelectorConfig),

    // Conflict-directed compound scalar repair selector with framework-enforced hard improvement.
    CompoundConflictRepairMoveSelector(CompoundConflictRepairMoveSelectorConfig),
}

impl MoveSelectorConfig {
    pub const fn selection_order(&self) -> Option<SelectionOrder> {
        match self {
            Self::ChangeMoveSelector(config) => config.selection_order,
            Self::SwapMoveSelector(config) => config.selection_order,
            Self::NearbyChangeMoveSelector(config) => config.selection_order,
            Self::NearbySwapMoveSelector(config) => config.selection_order,
            Self::PillarChangeMoveSelector(config) => config.selection_order,
            Self::PillarSwapMoveSelector(config) => config.selection_order,
            Self::RuinRecreateMoveSelector(config) => config.selection_order,
            Self::GroupedScalarMoveSelector(config) => config.selection_order,
            Self::ListChangeMoveSelector(config) => config.selection_order,
            Self::NearbyListChangeMoveSelector(config) => config.selection_order,
            Self::ListSwapMoveSelector(config) => config.selection_order,
            Self::ListPermuteMoveSelector(config) => config.selection_order,
            Self::ListPrecedenceMoveSelector(config) => config.selection_order,
            Self::NearbyListSwapMoveSelector(config) => config.selection_order,
            Self::SublistChangeMoveSelector(config) => config.selection_order,
            Self::SublistSwapMoveSelector(config) => config.selection_order,
            Self::ListReverseMoveSelector(config) => config.selection_order,
            Self::KOptMoveSelector(config) => config.selection_order,
            Self::ListRuinMoveSelector(config) => config.selection_order,
            Self::ConflictRepairMoveSelector(config) => config.selection_order,
            Self::CompoundConflictRepairMoveSelector(config) => config.selection_order,
            Self::LimitedNeighborhood(_)
            | Self::UnionMoveSelector(_)
            | Self::CartesianProductMoveSelector(_) => None,
        }
    }

    pub fn selection_metric(&self) -> Option<&str> {
        match self {
            Self::ChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::SwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::NearbyChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::NearbySwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::PillarChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::PillarSwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::RuinRecreateMoveSelector(config) => config.selection_metric.as_deref(),
            Self::GroupedScalarMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::NearbyListChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListSwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListPermuteMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListPrecedenceMoveSelector(config) => config.selection_metric.as_deref(),
            Self::NearbyListSwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::SublistChangeMoveSelector(config) => config.selection_metric.as_deref(),
            Self::SublistSwapMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListReverseMoveSelector(config) => config.selection_metric.as_deref(),
            Self::KOptMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ListRuinMoveSelector(config) => config.selection_metric.as_deref(),
            Self::ConflictRepairMoveSelector(config) => config.selection_metric.as_deref(),
            Self::CompoundConflictRepairMoveSelector(config) => config.selection_metric.as_deref(),
            Self::LimitedNeighborhood(_)
            | Self::UnionMoveSelector(_)
            | Self::CartesianProductMoveSelector(_) => None,
        }
    }
}

// Change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub value_candidate_limit: Option<usize>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Nearby change move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbyChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub max_nearby: usize,
    pub value_candidate_limit: Option<usize>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbyChangeMoveConfig {
    fn default() -> Self {
        let (max_nearby, target) = default_nearby_target_config();
        Self {
            selection_order: None,
            selection_metric: None,
            max_nearby,
            value_candidate_limit: None,
            target,
        }
    }
}

// Nearby swap move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NearbySwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub max_nearby: usize,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

impl Default for NearbySwapMoveConfig {
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

// Pillar change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PillarChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
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
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
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
    Sequential,
    RoundRobin,
    RotatingRoundRobin,
    Random,
    #[default]
    StratifiedRandom,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnionWeighting {
    #[default]
    Equal,
    Fixed,
    CandidateCount,
}

// Ruin-and-recreate move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RuinRecreateMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
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
            selection_order: None,
            selection_metric: None,
            min_ruin_count: 2,
            max_ruin_count: 5,
            moves_per_step: None,
            value_candidate_limit: None,
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
            target: VariableTargetConfig::default(),
        }
    }
}

// Configuration for `GroupedScalarMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GroupedScalarMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub group_name: String,
    pub value_candidate_limit: Option<usize>,
    pub max_moves_per_step: Option<usize>,
    #[serde(default)]
    pub require_hard_improvement: bool,
}

// Configuration for `ListChangeMoveSelector`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListChangeMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListChangeMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub struct ListSwapMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `ListPermuteMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub struct ListPrecedenceMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `NearbyListSwapMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub struct ListReverseMoveConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

// Configuration for `KOptMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct KOptMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
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
#[serde(rename_all = "snake_case")]
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
    #[serde(default)]
    pub weighting: UnionWeighting,
    #[serde(default)]
    pub weights: Vec<u64>,
    // Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

// Cartesian product move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CartesianProductConfig {
    // When true, search phases reject composed candidates unless the hard score improves.
    #[serde(default)]
    pub require_hard_improvement: bool,

    // Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ConflictRepairMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub constraints: Vec<String>,
    #[serde(default = "default_conflict_repair_max_matches")]
    pub max_matches_per_step: usize,
    #[serde(default = "default_conflict_repair_max_repairs")]
    pub max_repairs_per_match: usize,
    #[serde(default = "default_conflict_repair_max_moves")]
    pub max_moves_per_step: usize,
    #[serde(default)]
    pub require_hard_improvement: bool,
    #[serde(default)]
    pub include_soft_matches: bool,
}

impl Default for ConflictRepairMoveSelectorConfig {
    fn default() -> Self {
        Self {
            selection_order: None,
            selection_metric: None,
            constraints: Vec::new(),
            max_matches_per_step: default_conflict_repair_max_matches(),
            max_repairs_per_match: default_conflict_repair_max_repairs(),
            max_moves_per_step: default_conflict_repair_max_moves(),
            require_hard_improvement: false,
            include_soft_matches: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CompoundConflictRepairMoveSelectorConfig {
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    pub selection_metric: Option<String>,
    pub constraints: Vec<String>,
    #[serde(default = "default_conflict_repair_max_matches")]
    pub max_matches_per_step: usize,
    #[serde(default = "default_conflict_repair_max_repairs")]
    pub max_repairs_per_match: usize,
    #[serde(default = "default_conflict_repair_max_moves")]
    pub max_moves_per_step: usize,
    #[serde(default = "default_require_hard_improvement")]
    pub require_hard_improvement: bool,
    #[serde(default)]
    pub include_soft_matches: bool,
}

impl Default for CompoundConflictRepairMoveSelectorConfig {
    fn default() -> Self {
        Self {
            selection_order: None,
            selection_metric: None,
            constraints: Vec::new(),
            max_matches_per_step: default_conflict_repair_max_matches(),
            max_repairs_per_match: default_conflict_repair_max_repairs(),
            max_moves_per_step: default_conflict_repair_max_moves(),
            require_hard_improvement: default_require_hard_improvement(),
            include_soft_matches: false,
        }
    }
}

fn default_conflict_repair_max_matches() -> usize {
    16
}

fn default_conflict_repair_max_repairs() -> usize {
    32
}

fn default_conflict_repair_max_moves() -> usize {
    256
}

fn default_require_hard_improvement() -> bool {
    true
}

fn default_nearby_target_config() -> (usize, VariableTargetConfig) {
    (10, VariableTargetConfig::default())
}

fn default_sublist_target_config() -> (usize, usize, VariableTargetConfig) {
    (1, 3, VariableTargetConfig::default())
}
