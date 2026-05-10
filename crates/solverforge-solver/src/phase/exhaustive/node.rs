/* Exhaustive search node representation.

Each node represents a partial solution state in the search tree.
*/

use solverforge_core::domain::PlanningSolution;

/* A node in the exhaustive search tree.

Each node represents a partial solution state, containing:
- The depth in the search tree (number of variables assigned)
- The score at this node
- An optimistic bound (best possible score from this node)
- The scalar assignment index tuple to reach this node from its parent
*/
#[derive(Clone, Debug)]
pub struct ExhaustiveSearchNode<S: PlanningSolution> {
    // Depth in the search tree (0 = root, number of assignments made).
    depth: usize,

    // The score at this node after applying all moves.
    score: S::Score,

    // Optimistic bound: best possible score achievable from this node.
    // Used for pruning branches that cannot improve on the best solution.
    optimistic_bound: Option<S::Score>,

    // Descriptor index of the scalar entity collection being assigned.
    descriptor_index: Option<usize>,

    // Variable index within the descriptor being assigned.
    variable_index: Option<usize>,

    // Index of the entity being assigned at this node.
    entity_index: Option<usize>,

    // Index of the candidate value assigned at this node.
    candidate_value_index: Option<usize>,

    // Parent node index in the node list (None for root).
    parent_index: Option<usize>,

    // Whether this node has been expanded.
    expanded: bool,
}

impl<S: PlanningSolution> ExhaustiveSearchNode<S> {
    pub fn root(score: S::Score) -> Self {
        Self {
            depth: 0,
            score,
            optimistic_bound: None,
            descriptor_index: None,
            variable_index: None,
            entity_index: None,
            candidate_value_index: None,
            parent_index: None,
            expanded: false,
        }
    }

    pub fn child(
        parent_index: usize,
        depth: usize,
        score: S::Score,
        descriptor_index: usize,
        variable_index: usize,
        entity_index: usize,
        candidate_value_index: usize,
    ) -> Self {
        Self {
            depth,
            score,
            optimistic_bound: None,
            descriptor_index: Some(descriptor_index),
            variable_index: Some(variable_index),
            entity_index: Some(entity_index),
            candidate_value_index: Some(candidate_value_index),
            parent_index: Some(parent_index),
            expanded: false,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn score(&self) -> &S::Score {
        &self.score
    }

    pub fn set_score(&mut self, score: S::Score) {
        self.score = score;
    }

    #[inline]
    pub fn optimistic_bound(&self) -> Option<&S::Score> {
        self.optimistic_bound.as_ref()
    }

    pub fn set_optimistic_bound(&mut self, bound: S::Score) {
        self.optimistic_bound = Some(bound);
    }

    #[inline]
    pub fn descriptor_index(&self) -> Option<usize> {
        self.descriptor_index
    }

    #[inline]
    pub fn variable_index(&self) -> Option<usize> {
        self.variable_index
    }

    #[inline]
    pub fn entity_index(&self) -> Option<usize> {
        self.entity_index
    }

    #[inline]
    pub fn candidate_value_index(&self) -> Option<usize> {
        self.candidate_value_index
    }

    #[inline]
    pub fn parent_index(&self) -> Option<usize> {
        self.parent_index
    }

    // Returns whether this node has been expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Marks this node as expanded.
    pub fn mark_expanded(&mut self) {
        self.expanded = true;
    }

    pub fn is_leaf(&self, total_entities: usize) -> bool {
        self.depth >= total_entities
    }

    pub fn can_prune(&self, best_score: &S::Score) -> bool {
        match &self.optimistic_bound {
            Some(bound) => bound <= best_score,
            None => false,
        }
    }

    pub fn assignment_path<'a>(&'a self, all_nodes: &'a [Self]) -> Vec<&'a Self> {
        let mut path = Vec::with_capacity(self.depth);
        let mut current = Some(self);

        while let Some(node) = current {
            if node.parent_index.is_some() {
                path.push(node);
            }
            current = node.parent_index.and_then(|index| all_nodes.get(index));
        }

        path.reverse();
        path
    }
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
