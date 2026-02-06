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
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The value type
pub trait TypedValueSelector<S: PlanningSolution, V>: Send + Debug {
    /// Returns an iterator over typed values for the given entity.
    fn iter_typed<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a;

    /// Returns the number of values.
    fn size<D: ScoreDirector<S>>(
        &self,
        score_director: &D,
        descriptor_index: usize,
        entity_index: usize,
    ) -> usize;

    /// Returns true if this selector may return the same value multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A typed value selector with a static list of values.
pub struct StaticTypedValueSelector<S, V> {
    values: Vec<V>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, V: Clone> Clone for StaticTypedValueSelector<S, V> {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            _phantom: PhantomData,
        }
    }
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
    fn iter_typed<'a, D: ScoreDirector<S>>(
        &'a self,
        _score_director: &'a D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        self.values.iter().cloned()
    }

    fn size<D: ScoreDirector<S>>(
        &self,
        _score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        self.values.len()
    }
}

/// A typed value selector that extracts values from the solution using a function pointer.
pub struct FromSolutionTypedValueSelector<S, V> {
    extractor: fn(&S) -> Vec<V>,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V> Debug for FromSolutionTypedValueSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FromSolutionTypedValueSelector").finish()
    }
}

impl<S, V> FromSolutionTypedValueSelector<S, V> {
    /// Creates a new selector with the given extractor function pointer.
    pub fn new(extractor: fn(&S) -> Vec<V>) -> Self {
        Self {
            extractor,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> TypedValueSelector<S, V> for FromSolutionTypedValueSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Debug + 'static,
{
    fn iter_typed<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = V> + 'a {
        let values = (self.extractor)(score_director.working_solution());
        values.into_iter()
    }

    fn size<D: ScoreDirector<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        (self.extractor)(score_director.working_solution()).len()
    }
}

/// A typed value selector that generates a range of usize values 0..count.
///
/// Uses a function pointer to get the count from the solution.
pub struct RangeValueSelector<S> {
    count_fn: fn(&S) -> usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for RangeValueSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeValueSelector").finish()
    }
}

impl<S> RangeValueSelector<S> {
    /// Creates a new range value selector with the given count function.
    pub fn new(count_fn: fn(&S) -> usize) -> Self {
        Self {
            count_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S> TypedValueSelector<S, usize> for RangeValueSelector<S>
where
    S: PlanningSolution,
{
    fn iter_typed<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        let count = (self.count_fn)(score_director.working_solution());
        0..count
    }

    fn size<D: ScoreDirector<S>>(
        &self,
        score_director: &D,
        _descriptor_index: usize,
        _entity_index: usize,
    ) -> usize {
        (self.count_fn)(score_director.working_solution())
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
        fn extract_priorities(s: &TaskSolution) -> Vec<i32> {
            s.tasks.iter().filter_map(|t| t.priority).collect()
        }

        let selector = FromSolutionTypedValueSelector::new(extract_priorities);

        let values: Vec<_> = selector.iter_typed(&director, 0, 0).collect();
        assert_eq!(values, vec![10, 20]);
    }
}
