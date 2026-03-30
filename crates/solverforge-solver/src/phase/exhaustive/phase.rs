// ExhaustiveSearchPhase implementation.

use std::collections::BinaryHeap;
use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PhaseScope, SolverScope};

use super::config::ExhaustiveSearchConfig;
use super::decider::ExhaustiveSearchDecider;
use super::exploration_type::ExplorationType;
use super::node::ExhaustiveSearchNode;
use super::priority_node::PriorityNode;

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
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct MySolution {
///     values: Vec<Option<i32>>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for MySolution {
///     type Score = SoftScore;
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
    // The decider that generates child nodes.
    decider: Dec,
    // Configuration for this phase.
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
    pub fn new(decider: Dec, config: ExhaustiveSearchConfig) -> Self {
        Self { decider, config }
    }

    pub fn depth_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::DepthFirst,
                ..Default::default()
            },
        )
    }

    pub fn breadth_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::BreadthFirst,
                ..Default::default()
            },
        )
    }

    pub fn score_first(decider: Dec) -> Self {
        Self::new(
            decider,
            ExhaustiveSearchConfig {
                exploration_type: ExplorationType::ScoreFirst,
                ..Default::default()
            },
        )
    }

    pub fn phase_type_name(&self) -> &'static str {
        "ExhaustiveSearch"
    }
}

impl<S, D, BestCb, Dec> Phase<S, D, BestCb> for ExhaustiveSearchPhase<Dec>
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    Dec: ExhaustiveSearchDecider<S, D>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
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
    use solverforge_core::domain::PlanningSolution;
    use solverforge_core::score::SoftScore;

    use super::*;
    use crate::phase::exhaustive::decider::SimpleDecider;
    use crate::phase::exhaustive::exploration_type::ExplorationType;

    #[derive(Clone, Debug)]
    struct TestSolution {
        score: Option<SoftScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SoftScore;

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
