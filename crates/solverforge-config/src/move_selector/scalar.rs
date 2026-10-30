use super::*;

/// Change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Caps the scalar value candidates generated per entity; omitted means
    /// unbounded.
    pub value_candidate_limit: Option<usize>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SwapMoveConfig {
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

/// Nearby change move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbyChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Nearby values considered per entity. Required in TOML; `Default::default()` uses `10`.
    pub max_nearby: usize,
    /// Caps the scalar value candidates generated per entity; omitted means
    /// unbounded.
    pub value_candidate_limit: Option<usize>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
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

/// Nearby swap move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NearbySwapMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Nearby swap partners considered per entity. Required in TOML;
    /// `Default::default()` uses `10`.
    pub max_nearby: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
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

/// Pillar change move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PillarChangeMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Smallest sub-pillar size; `0` together with
    /// `maximum_sub_pillar_size = 0` means full pillars only. Required in TOML;
    /// `Default::default()` uses `0`.
    pub minimum_sub_pillar_size: usize,
    /// Largest sub-pillar size; see `minimum_sub_pillar_size`. Required in
    /// TOML; `Default::default()` uses `0`.
    pub maximum_sub_pillar_size: usize,
    /// Caps the scalar value candidates generated per entity; omitted means
    /// unbounded.
    pub value_candidate_limit: Option<usize>,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// Pillar swap move configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PillarSwapMoveConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Smallest sub-pillar size; `0` together with
    /// `maximum_sub_pillar_size = 0` means full pillars only. Required in TOML;
    /// `Default::default()` uses `0`.
    pub minimum_sub_pillar_size: usize,
    /// Largest sub-pillar size; see `minimum_sub_pillar_size`. Required in
    /// TOML; `Default::default()` uses `0`.
    pub maximum_sub_pillar_size: usize,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
    #[serde(flatten)]
    pub target: VariableTargetConfig,
}

/// How a ruin-and-recreate move reassigns the entities it unassigned.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecreateHeuristicType {
    /// Assign each entity its first doable value. Default.
    #[default]
    FirstFit,
    /// Assign each entity the value with the best resulting score.
    CheapestInsertion,
}

/// Ruin-and-recreate move configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuinRecreateMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Fewest entities unassigned per move. Required in TOML; `Default::default()` uses `2`.
    pub min_ruin_count: usize,
    /// Most entities unassigned per move. Required in TOML; `Default::default()` uses `5`.
    pub max_ruin_count: usize,
    /// Ruin moves generated per step. Omitted: `10`.
    pub moves_per_step: Option<usize>,
    /// Caps the scalar value candidates generated per entity; omitted means
    /// unbounded.
    pub value_candidate_limit: Option<usize>,
    /// Recreate strategy. Required in TOML; `Default::default()` uses `first_fit`.
    pub recreate_heuristic_type: RecreateHeuristicType,
    /// Restricts the selector to one planning variable through the top-level
    /// `entity_class` and `variable_name` keys; both omitted means every matching
    /// variable.
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

/// Configuration for `GroupedScalarMoveSelector`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GroupedScalarMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Name of the model's `ScalarGroup` to move. Required.
    pub group_name: String,
    /// Caps value candidates per assignment or value source; overrides the
    /// group's own limit. Omitted: the group's limit.
    pub value_candidate_limit: Option<usize>,
    /// Caps the grouped moves generated per step; overrides the group's own
    /// limit. Omitted: the group's limit.
    pub max_moves_per_step: Option<usize>,
    /// Rejects moves that do not improve the hard score. Default: `false`.
    #[serde(default)]
    pub require_hard_improvement: bool,
}
