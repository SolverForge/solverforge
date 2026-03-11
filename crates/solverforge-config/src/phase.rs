use serde::{Deserialize, Serialize};

use crate::acceptor::AcceptorConfig;
use crate::forager::ForagerConfig;
use crate::move_selector::MoveSelectorConfig;
use crate::termination::TerminationConfig;

/// Phase configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhaseConfig {
    /// Construction heuristic phase.
    ConstructionHeuristic(ConstructionHeuristicConfig),

    /// Local search phase.
    LocalSearch(LocalSearchConfig),

    /// Exhaustive search phase.
    ExhaustiveSearch(ExhaustiveSearchConfig),

    /// Partitioned search phase.
    PartitionedSearch(PartitionedSearchConfig),

    /// Custom phase.
    Custom(CustomPhaseConfig),
}

/// Construction heuristic configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ConstructionHeuristicConfig {
    /// Type of construction heuristic.
    #[serde(default)]
    pub construction_heuristic_type: ConstructionHeuristicType,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Construction heuristic types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionHeuristicType {
    /// First fit heuristic (basic variables).
    #[default]
    FirstFit,

    /// First fit decreasing (by entity difficulty).
    FirstFitDecreasing,

    /// Weakest fit heuristic.
    WeakestFit,

    /// Weakest fit decreasing.
    WeakestFitDecreasing,

    /// Strongest fit heuristic.
    StrongestFit,

    /// Strongest fit decreasing.
    StrongestFitDecreasing,

    /// Cheapest insertion (greedy, basic variables).
    CheapestInsertion,

    /// Allocate entity from queue.
    AllocateEntityFromQueue,

    /// Allocate to value from queue.
    AllocateToValueFromQueue,

    /// List round-robin construction: distributes elements evenly across entities.
    ListRoundRobin,

    /// List cheapest insertion: inserts each element at the score-minimizing position.
    ListCheapestInsertion,

    /// List regret insertion: inserts elements in order of highest placement regret.
    ListRegretInsertion,

    /// List Clarke-Wright savings: greedy route merging by savings value.
    ListClarkeWright,
}

/// Local search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalSearchConfig {
    /// Acceptor configuration.
    pub acceptor: Option<AcceptorConfig>,

    /// Forager configuration.
    pub forager: Option<ForagerConfig>,

    /// Move selector configuration.
    pub move_selector: Option<MoveSelectorConfig>,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Exhaustive search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ExhaustiveSearchConfig {
    /// Exhaustive search type.
    #[serde(default)]
    pub exhaustive_search_type: ExhaustiveSearchType,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Exhaustive search types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExhaustiveSearchType {
    /// Branch and bound.
    #[default]
    BranchAndBound,

    /// Brute force.
    BruteForce,
}

/// Partitioned search configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PartitionedSearchConfig {
    /// Number of partitions.
    pub partition_count: Option<usize>,

    /// Phase termination configuration.
    pub termination: Option<TerminationConfig>,
}

/// Custom phase configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CustomPhaseConfig {
    /// Custom phase class name.
    pub custom_phase_class: Option<String>,
}
