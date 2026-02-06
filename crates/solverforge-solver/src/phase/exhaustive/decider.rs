//! Exhaustive search decider for node expansion.
//!
//! The decider is responsible for expanding nodes and generating
//! child nodes in the search tree.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::bounder::ScoreBounder;
use super::node::ExhaustiveSearchNode;

/// Decides how to expand nodes in the exhaustive search.
///
/// The decider is responsible for:
/// - Finding the next entity to assign
/// - Generating all possible value assignments
/// - Creating child nodes for each assignment
pub trait ExhaustiveSearchDecider<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Expands a node by generating all child nodes.
    ///
    /// Returns a vector of child nodes, one for each possible assignment.
    fn expand(
        &self,
        parent_index: usize,
        parent: &ExhaustiveSearchNode<S>,
        score_director: &mut D,
    ) -> Vec<ExhaustiveSearchNode<S>>;

    /// Returns the total number of entities to assign.
    fn total_entities(&self, score_director: &D) -> usize;
}

/// A simple value-based decider that works with any value type.
///
/// Uses typed setter for zero-erasure variable assignment.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The value type to assign
/// * `B` - The bounder type (use `Option<B>` for optional bounding)
pub struct SimpleDecider<S: PlanningSolution, V: Clone + Send + Sync + 'static, B = ()> {
    /// Descriptor index of the entity collection.
    descriptor_index: usize,
    /// Variable name to assign.
    variable_name: String,
    /// Possible values to try.
    values: Vec<V>,
    /// Score bounder for optimistic bounds (None = no bounding).
    bounder: Option<B>,
    /// Typed setter for zero-erasure variable assignment.
    setter: fn(&mut S, usize, Option<V>),
}

impl<S: PlanningSolution, V: Clone + Send + Sync + 'static> SimpleDecider<S, V, ()> {
    /// Creates a new simple decider with typed setter and no bounder.
    ///
    /// # Arguments
    /// * `descriptor_index` - Index of the entity descriptor
    /// * `variable_name` - Name of the variable being assigned
    /// * `values` - Possible values to try
    /// * `setter` - Typed setter function `fn(&mut S, entity_index, value)`
    pub fn new(
        descriptor_index: usize,
        variable_name: impl Into<String>,
        values: Vec<V>,
        setter: fn(&mut S, usize, Option<V>),
    ) -> Self {
        Self {
            descriptor_index,
            variable_name: variable_name.into(),
            values,
            bounder: None,
            setter,
        }
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + 'static, B> SimpleDecider<S, V, B> {
    /// Sets the bounder for optimistic bound calculation.
    pub fn with_bounder<B2>(self, bounder: B2) -> SimpleDecider<S, V, B2> {
        SimpleDecider {
            descriptor_index: self.descriptor_index,
            variable_name: self.variable_name,
            values: self.values,
            bounder: Some(bounder),
            setter: self.setter,
        }
    }
}

impl<S: PlanningSolution, V: Clone + Send + Sync + Debug + 'static, B: Debug> Debug
    for SimpleDecider<S, V, B>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleDecider")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("value_count", &self.values.len())
            .finish()
    }
}

impl<S, V, B, D> ExhaustiveSearchDecider<S, D> for SimpleDecider<S, V, B>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    B: ScoreBounder<S, D>,
    D: ScoreDirector<S>,
{
    fn expand(
        &self,
        parent_index: usize,
        parent: &ExhaustiveSearchNode<S>,
        score_director: &mut D,
    ) -> Vec<ExhaustiveSearchNode<S>> {
        let entity_index = parent.depth();
        let new_depth = parent.depth() + 1;

        // Check if we've assigned all entities
        let total = self.total_entities(score_director);
        if entity_index >= total {
            return Vec::new();
        }

        let mut children = Vec::with_capacity(self.values.len());

        for (value_index, value) in self.values.iter().enumerate() {
            // Apply assignment using typed setter
            score_director.before_variable_changed(
                self.descriptor_index,
                entity_index,
                &self.variable_name,
            );

            (self.setter)(
                score_director.working_solution_mut(),
                entity_index,
                Some(value.clone()),
            );

            score_director.after_variable_changed(
                self.descriptor_index,
                entity_index,
                &self.variable_name,
            );

            // Calculate score for this assignment
            let score = score_director.calculate_score();

            // Create child node
            let mut child = ExhaustiveSearchNode::child(
                parent_index,
                new_depth,
                score,
                entity_index,
                value_index,
            );

            // Calculate optimistic bound if bounder is available
            if let Some(ref bounder) = self.bounder {
                if let Some(bound) = bounder.calculate_optimistic_bound(score_director) {
                    child.set_optimistic_bound(bound);
                }
            }

            children.push(child);

            // Undo the assignment for the next iteration
            score_director.before_variable_changed(
                self.descriptor_index,
                entity_index,
                &self.variable_name,
            );

            (self.setter)(score_director.working_solution_mut(), entity_index, None);

            score_director.after_variable_changed(
                self.descriptor_index,
                entity_index,
                &self.variable_name,
            );
        }

        children
    }

    fn total_entities(&self, score_director: &D) -> usize {
        score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0)
    }
}

// Implement ScoreBounder for () to allow SimpleDecider<S, V> (no bounder)
impl<S: PlanningSolution, D: ScoreDirector<S>> ScoreBounder<S, D> for () {
    fn calculate_optimistic_bound(&self, _score_director: &D) -> Option<S::Score> {
        None
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
    fn set_row(_s: &mut TestSolution, _idx: usize, _v: Option<i32>) {
        // No-op for this minimal test
    }

    #[test]
    fn test_simple_decider_creation() {
        let decider: SimpleDecider<TestSolution, i32> =
            SimpleDecider::new(0, "row", vec![0, 1, 2, 3], set_row);

        let debug = format!("{:?}", decider);
        assert!(debug.contains("SimpleDecider"));
        assert!(debug.contains("value_count: 4"));
    }
}
