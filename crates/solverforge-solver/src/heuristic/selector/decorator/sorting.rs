//! Sorting move selector decorator.
//!
//! Sorts moves from an inner selector using a comparator function.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Sorts moves from an inner selector using a comparator function.
///
/// Collects all moves from the inner selector and yields them in sorted order.
/// Uses a function pointer for zero-erasure comparison.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::SortingMoveSelector;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use std::cmp::Ordering;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_priority(s: &Solution, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.priority) }
/// fn set_priority(s: &mut Solution, i: usize, v: Option<i32>) { if let Some(t) = s.tasks.get_mut(i) { t.priority = v; } }
///
/// // Sort by target value descending
/// fn by_value_desc(a: &ChangeMove<Solution, i32>, b: &ChangeMove<Solution, i32>) -> Ordering {
///     b.to_value().cmp(&a.to_value())
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![30, 10, 50, 20],
/// );
/// let sorted: SortingMoveSelector<Solution, _, _> =
///     SortingMoveSelector::new(inner, by_value_desc);
/// assert!(!sorted.is_never_ending());
/// ```
pub struct SortingMoveSelector<S, M, Inner> {
    inner: Inner,
    comparator: fn(&M, &M) -> Ordering,
    _phantom: PhantomData<(S, M)>,
}

impl<S, M, Inner> SortingMoveSelector<S, M, Inner> {
    /// Creates a new sorting selector with the given comparator.
    ///
    /// # Arguments
    /// * `inner` - The inner selector to sort
    /// * `comparator` - Function pointer that compares two moves
    pub fn new(inner: Inner, comparator: fn(&M, &M) -> Ordering) -> Self {
        Self {
            inner,
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for SortingMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortingMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for SortingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        let comparator = self.comparator;
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.sort_by(comparator);
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        // Sorting collects all moves, so it's always finite
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::ChangeMoveSelector;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
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

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }
    fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
        s.tasks.get(i).and_then(|t| t.priority)
    }
    fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(i) {
            t.priority = v;
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    fn by_value_asc(
        a: &ChangeMove<TaskSolution, i32>,
        b: &ChangeMove<TaskSolution, i32>,
    ) -> Ordering {
        a.to_value().cmp(&b.to_value())
    }

    fn by_value_desc(
        a: &ChangeMove<TaskSolution, i32>,
        b: &ChangeMove<TaskSolution, i32>,
    ) -> Ordering {
        b.to_value().cmp(&a.to_value())
    }

    #[test]
    fn sorts_ascending() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![30, 10, 50, 20, 40],
        );
        let sorted = SortingMoveSelector::new(inner, by_value_asc);

        let values: Vec<_> = sorted
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(values, vec![10, 20, 30, 40, 50]);
    }

    #[test]
    fn sorts_descending() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![30, 10, 50, 20, 40],
        );
        let sorted = SortingMoveSelector::new(inner, by_value_desc);

        let values: Vec<_> = sorted
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(values, vec![50, 40, 30, 20, 10]);
    }

    #[test]
    fn preserves_size() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner =
            ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![30, 10, 50]);
        let sorted = SortingMoveSelector::new(inner, by_value_asc);

        assert_eq!(sorted.size(&director), 3);
    }
}
