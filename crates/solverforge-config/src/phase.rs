use serde::{Deserialize, Serialize};

use crate::acceptor::AcceptorConfig;
use crate::forager::ForagerConfig;
use crate::move_selector::{MoveSelectorConfig, VariableTargetConfig};
use crate::termination::TerminationConfig;

// Phase configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhaseConfig {
    // Construction heuristic phase.
    ConstructionHeuristic(ConstructionHeuristicConfig),

    // Local search phase.
    LocalSearch(LocalSearchConfig),

    // Exhaustive search phase.
    ExhaustiveSearch(ExhaustiveSearchConfig),

    // Partitioned search phase.
    PartitionedSearch(PartitionedSearchConfig),

    // Custom phase.
    Custom(CustomPhaseConfig),
}

fn default_k() -> usize {
    2
}

// Construction heuristic configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ConstructionHeuristicConfig {
    // Type of construction heuristic.
    #[serde(default)]
    pub construction_heuristic_type: ConstructionHeuristicType,

    // Whether nullable scalar variables may keep their current unassigned value during
    // construction, or must receive a candidate when one exists.
    #[serde(default)]
    pub construction_obligation: ConstructionObligation,

    // Optional variable target.
    #[serde(flatten)]
    pub target: VariableTargetConfig,

    // k for ListKOpt (default 2).
    #[serde(default = "default_k")]
    pub k: usize,

    // Optional cap for scalar value candidates generated per entity.
    pub value_candidate_limit: Option<usize>,

    // Optional named scalar group for atomic grouped scalar construction.
    pub group_name: Option<String>,

    // Optional cap for grouped scalar candidates generated per provider call.
    pub group_candidate_limit: Option<usize>,

    // Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

impl Default for ConstructionHeuristicConfig {
    fn default() -> Self {
        Self {
            construction_heuristic_type: ConstructionHeuristicType::default(),
            construction_obligation: ConstructionObligation::default(),
            target: VariableTargetConfig::default(),
            k: default_k(),
            value_candidate_limit: None,
            group_name: None,
            group_candidate_limit: None,
            termination: None,
        }
    }
}

// Construction obligation for nullable scalar variables.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionObligation {
    // Preserve current behavior: unassigned nullable variables may remain unassigned.
    #[default]
    PreserveUnassigned,

    // If a nullable scalar variable is unassigned and has a doable candidate, construction
    // must assign one instead of comparing against the unassigned baseline.
    AssignWhenCandidateExists,
}

// Construction heuristic types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionHeuristicType {
    // First fit heuristic (scalar variables).
    #[default]
    FirstFit,

    // First fit decreasing. Scalar-only; requires `construction_entity_order_key`.
    FirstFitDecreasing,

    // Weakest fit heuristic. Scalar-only; requires `construction_value_order_key`.
    WeakestFit,

    // Weakest fit decreasing. Scalar-only; requires both scalar construction order keys.
    WeakestFitDecreasing,

    // Strongest fit heuristic. Scalar-only; requires `construction_value_order_key`.
    StrongestFit,

    // Strongest fit decreasing. Scalar-only; requires both scalar construction order keys.
    StrongestFitDecreasing,

    // Cheapest insertion (greedy, scalar variables).
    CheapestInsertion,

    // Allocate entity from queue. Scalar-only; requires `construction_entity_order_key`.
    AllocateEntityFromQueue,

    // Allocate to value from queue. Scalar-only; requires `construction_value_order_key`.
    AllocateToValueFromQueue,

    // List round-robin construction: distributes elements evenly across entities and validates the
    // list construction hook surface before phase build.
    ListRoundRobin,

    // List cheapest insertion: inserts each element at the score-minimizing position and validates
    // the list construction hook surface before phase build.
    ListCheapestInsertion,

    // List regret insertion: inserts elements in order of highest placement regret and validates
    // the list construction hook surface before phase build.
    ListRegretInsertion,

    // List Clarke-Wright savings: greedy route merging by savings value; requires the declared
    // `cw_*` list hooks and validates them before phase build.
    ListClarkeWright,

    // List k-opt: per-route k-opt polishing (k=2 is exact 2-opt); requires the declared
    // `k_opt_*` list hooks and validates them before phase build.
    ListKOpt,
}

// Local search type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalSearchType {
    // Standard acceptor/forager local search.
    #[default]
    AcceptorForager,

    // Variable Neighborhood Descent over ordered neighborhoods.
    VariableNeighborhoodDescent,
}

// Local search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalSearchConfig {
    // Local search type.
    #[serde(default)]
    pub local_search_type: LocalSearchType,

    // Acceptor configuration.
    pub acceptor: Option<AcceptorConfig>,

    // Forager configuration.
    pub forager: Option<ForagerConfig>,

    // Move selector configuration.
    pub move_selector: Option<MoveSelectorConfig>,

    // Ordered neighborhood selectors for Variable Neighborhood Descent.
    #[serde(default)]
    pub neighborhoods: Vec<MoveSelectorConfig>,

    // Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

// Exhaustive search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ExhaustiveSearchConfig {
    // Exhaustive search type.
    #[serde(default)]
    pub exhaustive_search_type: ExhaustiveSearchType,

    // Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

// Exhaustive search types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExhaustiveSearchType {
    // Branch and bound.
    #[default]
    BranchAndBound,

    // Brute force.
    BruteForce,
}

// Partitioned search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PartitionedSearchConfig {
    // Number of partitions.
    pub partition_count: Option<usize>,

    // Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

// Custom phase configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CustomPhaseConfig {
    // Custom phase class name.
    pub custom_phase_class: Option<String>,
}
