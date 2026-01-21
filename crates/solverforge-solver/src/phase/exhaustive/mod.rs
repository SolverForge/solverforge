//! Exhaustive search phase using branch-and-bound.
//!
//! Exhaustive search explores the entire solution space systematically,
//! using pruning to avoid exploring branches that cannot improve on the
//! best solution found so far.
//!
//! # Exploration Types
//!
//! - **Depth First**: Explores deepest nodes first (memory efficient)
//! - **Breadth First**: Explores level by level (finds shortest paths)
//! - **Score First**: Explores best-scoring nodes first (greedy)
//! - **Optimistic Bound First**: Explores most promising bounds first (A*)

mod bounder;
mod decider;
mod node;

use std::collections::BinaryHeap;
use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope};

pub use bounder::{BounderType, FixedOffsetBounder, ScoreBounder, SimpleScoreBounder};
pub use decider::{ExhaustiveSearchDecider, SimpleDecider};
pub use node::{ExhaustiveSearchNode, MoveSequence};

/// Type of exploration strategy for exhaustive search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExplorationType {
    /// Depth-first search: explores deepest nodes first.
    /// Most memory efficient, but may take longer to find optimal.
    #[default]
    DepthFirst,

    /// Breadth-first search: explores level by level.
    /// Memory intensive, but finds shortest solution paths.
    BreadthFirst,

    /// Score-first search: explores best-scoring nodes first.
    /// Greedy approach that may find good solutions quickly.
    ScoreFirst,

    /// Optimistic bound first: explores most promising nodes first.
    /// A*-like behavior, requires a good bounder.
    OptimisticBoundFirst,
}

impl std::fmt::Display for ExplorationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExplorationType::DepthFirst => write!(f, "DepthFirst"),
            ExplorationType::BreadthFirst => write!(f, "BreadthFirst"),
            ExplorationType::ScoreFirst => write!(f, "ScoreFirst"),
            ExplorationType::OptimisticBoundFirst => write!(f, "OptimisticBoundFirst"),
        }
    }
}

/// Configuration for exhaustive search phase.
#[derive(Debug, Clone)]
pub struct ExhaustiveSearchConfig {
    /// The exploration type to use.
    pub exploration_type: ExplorationType,
    /// Maximum number of nodes to explore (None = unlimited).
    pub node_limit: Option<u64>,
    /// Maximum depth to explore (None = unlimited).
    pub depth_limit: Option<usize>,
    /// Whether to enable pruning based on bounds.
    pub enable_pruning: bool,
}

impl Default for ExhaustiveSearchConfig {
    fn default() -> Self {
        Self {
            exploration_type: ExplorationType::DepthFirst,
            node_limit: Some(10_000),
            depth_limit: None,
            enable_pruning: true,
        }
    }
}

/// A node wrapper for priority queue ordering.
struct PriorityNode<S: PlanningSolution> {
    index: usize,
    node: ExhaustiveSearchNode<S>,
    exploration_type: ExplorationType,
}

impl<S: PlanningSolution> PriorityNode<S> {
    fn new(index: usize, node: ExhaustiveSearchNode<S>, exploration_type: ExplorationType) -> Self {
        Self {
            index,
            node,
            exploration_type,
        }
    }
}

impl<S: PlanningSolution> Eq for PriorityNode<S> {}

impl<S: PlanningSolution> PartialEq for PriorityNode<S> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<S: PlanningSolution> Ord for PriorityNode<S> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.exploration_type {
            ExplorationType::DepthFirst => {
                // Higher depth = higher priority
                self.node.depth().cmp(&other.node.depth())
            }
            ExplorationType::BreadthFirst => {
                // Lower depth = higher priority (reversed)
                other.node.depth().cmp(&self.node.depth())
            }
            ExplorationType::ScoreFirst => {
                // Better score = higher priority
                self.node
                    .score()
                    .partial_cmp(other.node.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            ExplorationType::OptimisticBoundFirst => {
                // Better bound = higher priority
                match (self.node.optimistic_bound(), other.node.optimistic_bound()) {
                    (Some(a), Some(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
        }
    }
}

impl<S: PlanningSolution> PartialOrd for PriorityNode<S> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Exhaustive search phase that explores all possible solutions.
///
/// This phase systematically explores the solution space using a
/// branch-and-bound algorithm. It maintains a tree of partial solutions
/// and uses pruning to avoid exploring branches that cannot improve
/// on the best solution found.
///
/// # Type Parameters
/// * `Dec` - The decider type that generates child nodes
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::exhaustive::{ExhaustiveSearchPhase, SimpleDecider, ExhaustiveSearchConfig};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct MySolution {
///     values: Vec<Option<i32>>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn set_value(s: &mut MySolution, idx: usize, v: Option<i32>) {
///     if let Some(slot) = s.values.get_mut(idx) { *slot = v; }
/// }
///
/// let decider = SimpleDecider::<MySolution, i32>::new(0, "value", vec![1, 2, 3], set_value);
/// let phase = ExhaustiveSearchPhase::depth_first(decider);
/// ```
pub struct ExhaustiveSearchPhase<Dec> {
    /// The decider that generates child nodes.
    decider: Dec,
    /// Configuration for this phase.
    config: ExhaustiveSearchConfig,
}

impl<Dec: Debug> Debug for ExhaustiveSearchPhase<Dec> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExhaustiveSearchPhase")
            .field("decider", &self.decider)
            .field("config", &self.config)
            .finish()
    }
}

impl<Dec> ExhaustiveSearchPhase<Dec> {
    /// Creates a new exhaustive search phase.
    pub fn new(decider: Dec, config: ExhaustiveSearchConfig) -> Self {
        Self { decider, config }
    }

    /// Creates a depth-first exhaustive search phase.
    pub fn depth_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::DepthFirst,
                ..Default::default()
            },
        )
    }

    /// Creates a breadth-first exhaustive search phase.
    pub fn breadth_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::BreadthFirst,
                ..Default::default()
            },
        )
    }

    /// Creates a score-first exhaustive search phase.
    pub fn score_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::ScoreFirst,
                ..Default::default()
            },
        )
    }

    /// Returns the phase type name.
    pub fn phase_type_name(&self) -> &'static str {
        "ExhaustiveSearch"
    }
}

impl<S, D, Dec> Phase<S, D> for ExhaustiveSearchPhase<Dec>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    Dec: ExhaustiveSearchDecider<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Get total entities
        let total_entities = self.decider.total_entities(phase_scope.score_director());
        if total_entities == 0 {
            return;
        }

        // Calculate initial score
        let initial_score = phase_scope.calculate_score();

        // Create root node
        let root = ExhaustiveSearchNode::root(initial_score);

        // Initialize best score
        let mut best_score: Option<S::Score> = None;
        let mut nodes_explored: u64 = 0;

        // Priority queue for node exploration
        let mut frontier: BinaryHeap<PriorityNode<S>> = BinaryHeap::new();
        frontier.push(PriorityNode::new(0, root, self.config.exploration_type));

        // Node storage
        let mut all_nodes: Vec<ExhaustiveSearchNode<S>> = Vec::new();

        while let Some(priority_node) = frontier.pop() {
            let node = priority_node.node;
            let node_index = all_nodes.len();

            // Check node limit
            if let Some(limit) = self.config.node_limit {
                if nodes_explored >= limit {
                    break;
                }
            }

            // Check depth limit
            if let Some(limit) = self.config.depth_limit {
                if node.depth() >= limit {
                    continue;
                }
            }

            // Check pruning
            if self.config.enable_pruning {
                if let Some(ref best) = best_score {
                    if node.can_prune(best) {
                        continue;
                    }
                }
            }

            nodes_explored += 1;

            // Check if this is a complete solution (leaf node)
            if node.is_leaf(total_entities) {
                let is_better = match &best_score {
                    None => true,
                    Some(best) => node.score() > best,
                };

                if is_better {
                    best_score = Some(*node.score());
                    phase_scope.update_best_solution();
                }

                all_nodes.push(node);
                continue;
            }

            // Expand node to generate children
            let children = self
                .decider
                .expand(node_index, &node, phase_scope.score_director_mut());

            // Store current node
            all_nodes.push(node);

            // Add children to frontier
            for child in children {
                // Skip if prunable
                if self.config.enable_pruning {
                    if let Some(ref best) = best_score {
                        if child.can_prune(best) {
                            continue;
                        }
                    }
                }

                let child_index = all_nodes.len() + frontier.len();
                frontier.push(PriorityNode::new(
                    child_index,
                    child,
                    self.config.exploration_type,
                ));
            }
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "ExhaustiveSearch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    // Dummy setter for tests
    fn set_row(_s: &mut TestSolution, _idx: usize, _v: Option<i32>) {}

    #[test]
    fn test_exploration_type_display() {
        assert_eq!(format!("{}", ExplorationType::DepthFirst), "DepthFirst");
        assert_eq!(format!("{}", ExplorationType::BreadthFirst), "BreadthFirst");
        assert_eq!(format!("{}", ExplorationType::ScoreFirst), "ScoreFirst");
        assert_eq!(
            format!("{}", ExplorationType::OptimisticBoundFirst),
            "OptimisticBoundFirst"
        );
    }

    #[test]
    fn test_exploration_type_default() {
        assert_eq!(ExplorationType::default(), ExplorationType::DepthFirst);
    }

    #[test]
    fn test_config_default() {
        let config = ExhaustiveSearchConfig::default();
        assert_eq!(config.exploration_type, ExplorationType::DepthFirst);
        assert_eq!(config.node_limit, Some(10_000));
        assert!(config.depth_limit.is_none());
        assert!(config.enable_pruning);
    }

    #[test]
    fn test_phase_type_name() {
        let decider: SimpleDecider<TestSolution, i32> =
            SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);
        let phase = ExhaustiveSearchPhase::depth_first(decider);

        assert_eq!(phase.phase_type_name(), "ExhaustiveSearch");
    }

    #[test]
    fn test_phase_debug() {
        let decider: SimpleDecider<TestSolution, i32> =
            SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);
        let phase = ExhaustiveSearchPhase::depth_first(decider);

        let debug = format!("{:?}", phase);
        assert!(debug.contains("ExhaustiveSearchPhase"));
        assert!(debug.contains("DepthFirst"));
    }
}
