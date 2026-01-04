//! Typed value selectors for high-performance value iteration.
//!
//! Unlike the type-erased `ValueSelector` that yields `Arc<dyn Any>`,
//! typed value selectors yield `V` directly with no heap allocation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

/// A typed value selector that yields values of type `V` directly.
///
/// Unlike `ValueSelector` which returns `Arc<dyn Any>`, this trait
/// returns `V` inline, eliminating heap allocation per value.
pub trait TypedValueSelector<S: PlanningSolution, V>: Send + Debug {
    /// Returns an iterator over typed values for the given entity.
    fn iter_typed<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
        descriptor_index: usize,
        entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a>;

    /// Returns the number of values.
    fn size(
        &self,
        score_director: &dyn ScoreDirector<S>,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize;

    /// Returns true if this selector may return the same value multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A typed value selector with a static list of values.
#[derive(Clone)]
pub struct StaticTypedValueSelector<S, V> {
    values: Vec<V>,
    _phantom: PhantomData<S>,
}

impl<S, V: Debug> Debug for StaticTypedValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticTypedValueSelector")
            .field("values", &self.values)
            .finish()
    }
}

impl<S, V: Clone> StaticTypedValueSelector<S, V> {
    /// Creates a new static value selector with the given values.
    pub fn new(values: Vec<V>) -> Self {
        Self {
            values,
            _phantom: PhantomData,
        }
    }

    /// Returns the values.
    pub fn values(&self) -> &[V] {
        &self.values
    }
}

impl<S, V> TypedValueSelector<S, V> for StaticTypedValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a>(
        &'a self,
        _score_director: &'a dyn ScoreDirector<S>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a> {
        Box::new(self.values.iter().cloned())
    }

    fn size(
        &self,
        _score_director: &dyn ScoreDirector<S>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        self.values.len()
    }
}

/// A typed value selector that extracts values from the solution.
pub struct FromSolutionTypedValueSelector<S, V, F>
where
    F: Fn(&dyn ScoreDirector<S>) -> Vec<V> + Send + Sync,
{
    extractor: F,
    _phantom: PhantomData<(S, V)>,
}

impl<S, V, F> Debug for FromSolutionTypedValueSelector<S, V, F>
where
    F: Fn(&dyn ScoreDirector<S>) -> Vec<V> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromSolutionTypedValueSelector").finish()
    }
}

impl<S, V, F> FromSolutionTypedValueSelector<S, V, F>
where
    F: Fn(&dyn ScoreDirector<S>) -> Vec<V> + Send + Sync,
{
    /// Creates a new selector with the given extractor function.
    pub fn new(extractor: F) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, F> TypedValueSelector<S, V> for FromSolutionTypedValueSelector<S, V, F>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
    F: Fn(&dyn ScoreDirector<S>) -> Vec<V> + Send + Sync,
{
    fn iter_typed<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> Box<dyn Iterator<Item = V> + 'a> {
        let values = (self.extractor)(score_director);
        Box::new(values.into_iter())
    }

    fn size(
        &self,
        score_director: &dyn ScoreDirector<S>,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        (self.extractor)(score_director).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        id: usize,
        priority: Option<i32>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            |s: &TaskSolution| &s.tasks,
            |s: &mut TaskSolution| &mut s.tasks,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn test_static_typed_value_selector() {
        let director = create_director(vec![Task {
            id: 0,
            priority: None,
        }]);
        let selector = StaticTypedValueSelector::<TaskSolution, i32>::new(vec![1, 2, 3, 4, 5]);

        let values: Vec<_> = selector.iter_typed(&director, 0, 0).collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
        assert_eq!(selector.size(&director, 0, 0), 5);
    }

    #[test]
    fn test_from_solution_typed_value_selector() {
        let director = create_director(vec![
            Task {
                id: 0,
                priority: Some(10),
            },
            Task {
                id: 1,
                priority: Some(20),
            },
        ]);

        // Verify entity IDs
        let solution = director.working_solution();
        assert_eq!(solution.tasks[0].id, 0);
        assert_eq!(solution.tasks[1].id, 1);

        // Extract priorities directly from solution - zero erasure
        let selector =
            FromSolutionTypedValueSelector::new(|sd: &dyn ScoreDirector<TaskSolution>| {
                sd.working_solution()
                    .tasks
                    .iter()
                    .filter_map(|t| t.priority)
                    .collect()
            });

        let values: Vec<_> = selector.iter_typed(&director, 0, 0).collect();
        assert_eq!(values, vec![10, 20]);
    }
}
