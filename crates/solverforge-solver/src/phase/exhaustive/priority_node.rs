// Priority node for exhaustive search frontier ordering.

use solverforge_core::domain::PlanningSolution;

use super::exploration_type::ExplorationType;
use super::node::ExhaustiveSearchNode;

/// A node wrapper for priority queue ordering.
pub(super) struct PriorityNode<S: PlanningSolution> {
    pub(super) index: usize,
    pub(super) node: ExhaustiveSearchNode<S>,
    pub(super) exploration_type: ExplorationType,
}

impl<S: PlanningSolution> PriorityNode<S> {
    pub(super) fn new(
        index: usize,
        node: ExhaustiveSearchNode<S>,
        exploration_type: ExplorationType,
    ) -> Self {
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
