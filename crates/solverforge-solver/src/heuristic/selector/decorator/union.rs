//! Union move selector combinator.
//!
//! Combines moves from two selectors into a single stream.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Combines moves from two selectors into a single stream.
///
/// Yields all moves from the first selector, then all moves from the second.
/// Both selectors must produce the same move type.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::UnionMoveSelector;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
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
/// let low_values = ChangeMoveSelector::<Solution, i32>::simple(
///     get_priority, set_priority, 0, "priority", vec![1, 2, 3],
/// );
/// let high_values = ChangeMoveSelector::<Solution, i32>::simple(
///     get_priority, set_priority, 0, "priority", vec![100, 200],
/// );
/// // Union yields moves with values: 1, 2, 3, 100, 200
/// let combined: UnionMoveSelector<Solution, _, _, _> =
///     UnionMoveSelector::new(low_values, high_values);
/// assert!(!combined.is_never_ending());
/// ```
pub struct UnionMoveSelector<S, M, A, B> {
    first: A,
    second: B,
    _phantom: PhantomData<(S, M)>,
}

impl<S, M, A, B> UnionMoveSelector<S, M, A, B> {
    /// Creates a new union selector combining two selectors.
    ///
    /// # Arguments
    /// * `first` - The first selector (yields moves first)
    /// * `second` - The second selector (yields moves after first is exhausted)
    pub fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, A: Debug, B: Debug> Debug for UnionMoveSelector<S, M, A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnionMoveSelector")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<S, M, A, B> MoveSelector<S, M> for UnionMoveSelector<S, M, A, B>
where
    S: PlanningSolution,
    M: Move<S>,
    A: MoveSelector<S, M>,
    B: MoveSelector<S, M>,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        Box::new(
            self.first
                .iter_moves(score_director)
                .chain(self.second.iter_moves(score_director)),
        )
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        self.first.size(score_director) + self.second.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.first.is_never_ending() || self.second.is_never_ending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn combines_both_selectors() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let first = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20],
        );
        let second = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![100, 200, 300],
        );
        let union = UnionMoveSelector::new(first, second);

        let values: Vec<_> = union
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(values, vec![10, 20, 100, 200, 300]);
        assert_eq!(union.size(&director), 5);
    }

    #[test]
    fn handles_empty_first() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let first = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![],
        );
        let second = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![100, 200],
        );
        let union = UnionMoveSelector::new(first, second);

        let values: Vec<_> = union
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(values, vec![100, 200]);
    }

    #[test]
    fn handles_empty_second() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let first = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20],
        );
        let second = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![],
        );
        let union = UnionMoveSelector::new(first, second);

        let values: Vec<_> = union
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(values, vec![10, 20]);
    }

    #[test]
    fn both_empty_yields_nothing() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let first = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![],
        );
        let second = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![],
        );
        let union = UnionMoveSelector::new(first, second);

        let moves: Vec<_> = union.iter_moves(&director).collect();

        assert!(moves.is_empty());
        assert_eq!(union.size(&director), 0);
    }
}
