use serde::{Deserialize, Serialize};

mod composite;
mod list;
mod scalar;

pub use composite::{
    CartesianProductConfig, CompoundConflictRepairMoveSelectorConfig,
    ConflictRepairMoveSelectorConfig, LimitedNeighborhoodConfig, UnionMoveSelectorConfig,
    UnionSelectionOrder, UnionWeighting,
};
pub use list::{
    KOptMoveSelectorConfig, ListChangeMoveConfig, ListPermuteMoveConfig, ListPrecedenceMoveConfig,
    ListReverseMoveConfig, ListRuinMoveSelectorConfig, ListSwapMoveConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, SublistChangeMoveConfig,
    SublistSwapMoveConfig,
};
pub use scalar::{
    ChangeMoveConfig, GroupedScalarMoveSelectorConfig, NearbyChangeMoveConfig,
    NearbySwapMoveConfig, PillarChangeMoveConfig, PillarSwapMoveConfig, RecreateHeuristicType,
    RuinRecreateMoveSelectorConfig, SwapMoveConfig,
};

/// Optional restriction of a selector or phase to one planning variable.
///
/// Always flattened into its parent, so TOML writes `entity_class` and
/// `variable_name` as top-level keys of the selector or phase.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct VariableTargetConfig {
    /// Entity class name to target; omitted means every entity class.
    pub entity_class: Option<String>,
    /// Planning variable name to target; omitted means every variable.
    pub variable_name: Option<String>,
}

/// Order in which a leaf selector yields its candidates.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOrder {
    /// Source order. Default.
    #[default]
    Original,
    /// Seeded random selection with replacement.
    Random,
    /// Seeded permutation of the source order, each candidate at most once.
    Shuffled,
    /// Ascending value of `selection_metric` over the materialized
    /// neighborhood.
    Sorted,
    /// Seeded random order weighted by `selection_metric`; zero-weight
    /// candidates are skipped.
    Probabilistic,
}

impl SelectionOrder {
    /// Returns `true` for the seeded orders: `random`, `shuffled` and
    /// `probabilistic`.
    pub const fn is_random(self) -> bool {
        matches!(self, Self::Random | Self::Shuffled | Self::Probabilistic)
    }

    /// Returns `true` for orders that must materialize the whole neighborhood:
    /// `sorted` and `probabilistic`.
    pub const fn requires_complete_stream(self) -> bool {
        matches!(self, Self::Sorted | Self::Probabilistic)
    }
}

/// Move selector configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MoveSelectorConfig {
    /// Change move selector (scalar variables).
    ChangeMoveSelector(ChangeMoveConfig),

    /// Swap move selector (scalar variables).
    SwapMoveSelector(SwapMoveConfig),

    /// Nearby change move selector (scalar variables).
    NearbyChangeMoveSelector(NearbyChangeMoveConfig),

    /// Nearby swap move selector (scalar variables).
    NearbySwapMoveSelector(NearbySwapMoveConfig),

    /// Pillar change move selector (scalar variables).
    PillarChangeMoveSelector(PillarChangeMoveConfig),

    /// Pillar swap move selector (scalar variables).
    PillarSwapMoveSelector(PillarSwapMoveConfig),

    /// Ruin-and-recreate move selector (scalar variables).
    RuinRecreateMoveSelector(RuinRecreateMoveSelectorConfig),

    /// Atomic grouped scalar move selector.
    GroupedScalarMoveSelector(GroupedScalarMoveSelectorConfig),

    /// List change move selector — relocates single elements within/between routes.
    ListChangeMoveSelector(ListChangeMoveConfig),

    /// Nearby list change move selector — distance-pruned element relocation.
    NearbyListChangeMoveSelector(NearbyListChangeMoveConfig),

    /// List swap move selector — swaps single elements within/between routes.
    ListSwapMoveSelector(ListSwapMoveConfig),

    /// List permute move selector — permutes contiguous windows within routes.
    ListPermuteMoveSelector(ListPermuteMoveConfig),

    /// List precedence move selector — prioritizes critical list arcs in
    /// precedence makespan models.
    ListPrecedenceMoveSelector(ListPrecedenceMoveConfig),

    /// Nearby list swap move selector — distance-pruned element swap.
    NearbyListSwapMoveSelector(NearbyListSwapMoveConfig),

    /// Sublist change move selector (Or-opt) — relocates contiguous segments.
    SublistChangeMoveSelector(SublistChangeMoveConfig),

    /// Sublist swap move selector — swaps contiguous segments between routes.
    SublistSwapMoveSelector(SublistSwapMoveConfig),

    /// List reverse move selector (2-opt) — reverses segments within a route.
    ListReverseMoveSelector(ListReverseMoveConfig),

    /// K-opt move selector — generalised route reconnection.
    KOptMoveSelector(KOptMoveSelectorConfig),

    /// List ruin move selector (LNS) — removes elements for reinsertion.
    ListRuinMoveSelector(ListRuinMoveSelectorConfig),

    /// Neighborhood that limits the number of yielded candidates from a child selector while
    /// preserving selector order.
    LimitedNeighborhood(LimitedNeighborhoodConfig),

    /// Union of multiple selectors.
    UnionMoveSelector(UnionMoveSelectorConfig),

    /// Cartesian product of selectors. Evaluates the right child on the left preview state,
    /// composes tabu ids in selector order, and rejects left children that require full score
    /// evaluation during preview.
    CartesianProductMoveSelector(CartesianProductConfig),

    /// Conflict-directed scalar repair selector.
    ConflictRepairMoveSelector(ConflictRepairMoveSelectorConfig),

    /// Conflict-directed compound scalar repair selector with framework-enforced hard improvement.
    CompoundConflictRepairMoveSelector(CompoundConflictRepairMoveSelectorConfig),
}

impl MoveSelectorConfig {
    /// The leaf selector's `selection_order`; `None` for limited, union and
    /// cartesian composites.
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

    /// The leaf selector's `selection_metric`; `None` for limited, union and
    /// cartesian composites.
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
