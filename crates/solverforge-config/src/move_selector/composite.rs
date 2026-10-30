use super::*;

/// How a union interleaves its child selectors.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnionSelectionOrder {
    /// Drain each child in order before moving to the next.
    Sequential,
    /// Take one candidate from each live child in turn, starting with the
    /// first.
    RoundRobin,
    /// Round-robin starting at a seeded child.
    RotatingRoundRobin,
    /// Pick a live child at random for each candidate, in proportion to its
    /// weight.
    Random,
    /// Smooth weighted interleaving over a seeded child order. Default.
    #[default]
    StratifiedRandom,
}

/// How union children are weighted by `random` and `stratified_random`
/// order.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnionWeighting {
    /// Every child has weight `1`. Default.
    #[default]
    Equal,
    /// Weights come from `weights`.
    Fixed,
    /// Each child is weighted by its declared candidate count.
    CandidateCount,
}

/// Configuration for `LimitedNeighborhood`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LimitedNeighborhoodConfig {
    /// Maximum number of moves yielded from the child selector.
    pub selected_count_limit: usize,
    /// Child selector to wrap.
    pub selector: Box<MoveSelectorConfig>,
}

/// Union move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UnionMoveSelectorConfig {
    /// Child interleaving order. Default: `stratified_random`.
    #[serde(default)]
    pub selection_order: UnionSelectionOrder,
    /// Child weighting. Default: `equal`.
    #[serde(default)]
    pub weighting: UnionWeighting,
    /// One weight per child for `weighting = "fixed"`; a zero weight disables
    /// that child. Weights other than `1` need `random` or `stratified_random`
    /// order. Default: empty.
    #[serde(default)]
    pub weights: Vec<u64>,
    /// Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

/// Cartesian product move selector configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CartesianProductConfig {
    /// When true, search phases reject composed candidates unless the hard score improves.
    #[serde(default)]
    pub require_hard_improvement: bool,

    /// Child selectors.
    pub selectors: Vec<MoveSelectorConfig>,
}

/// Configuration for `ConflictRepairMoveSelector`, which asks registered
/// repair providers for scalar moves that fix matches of named constraints.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ConflictRepairMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Names of the constraints whose matches are repaired. Required; each name
    /// must match a scoring constraint.
    pub constraints: Vec<String>,
    /// Constraint matches visited per step. Default: `16`.
    #[serde(default = "default_conflict_repair_max_matches")]
    pub max_matches_per_step: usize,
    /// Repair moves taken from the provider per match. Default: `32`.
    #[serde(default = "default_conflict_repair_max_repairs")]
    pub max_repairs_per_match: usize,
    /// Total moves generated per step. Default: `256`.
    #[serde(default = "default_conflict_repair_max_moves")]
    pub max_moves_per_step: usize,
    /// Rejects repair moves that do not improve the hard score. Default:
    /// `false`.
    #[serde(default)]
    pub require_hard_improvement: bool,
    /// Allows non-hard constraints in `constraints`; with `false` they are
    /// rejected. Default: `false`.
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

/// Configuration for `CompoundConflictRepairMoveSelector`, which combines
/// the repairs for one match into a single compound move.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CompoundConflictRepairMoveSelectorConfig {
    /// Order in which this selector's candidates are tried. Omitted, the runtime
    /// supplies the surrounding default (`random` in local search).
    #[serde(default)]
    pub selection_order: Option<SelectionOrder>,
    /// Name of a registered candidate metric. Required with `sorted` and
    /// `probabilistic` order; rejected with any other order.
    pub selection_metric: Option<String>,
    /// Names of the constraints whose matches are repaired. Required; each name
    /// must match a scoring constraint.
    pub constraints: Vec<String>,
    /// Constraint matches visited per step. Default: `16`.
    #[serde(default = "default_conflict_repair_max_matches")]
    pub max_matches_per_step: usize,
    /// Repair moves taken from the provider per match. Default: `32`.
    #[serde(default = "default_conflict_repair_max_repairs")]
    pub max_repairs_per_match: usize,
    /// Total moves generated per step. Default: `256`.
    #[serde(default = "default_conflict_repair_max_moves")]
    pub max_moves_per_step: usize,
    /// Rejects compound moves that do not improve the hard score. Default:
    /// `true`.
    #[serde(default = "default_require_hard_improvement")]
    pub require_hard_improvement: bool,
    /// Allows non-hard constraints in `constraints`; with `false` they are
    /// rejected. Default: `false`.
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
