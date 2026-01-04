//! Shuffling move selector decorator.
//!
//! Shuffles moves from an inner selector using Fisher-Yates.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::prelude::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Shuffles moves from an inner selector randomly.
///
/// Collects all moves from the inner selector and yields them in random order.
/// Uses interior mutability for the RNG since `iter_moves` takes `&self`.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::ShufflingMoveSelector;
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
/// let inner = ChangeMoveSelector::<Solution, i32>::simple(
///     get_priority, set_priority, 0, "priority", vec![1, 2, 3, 4, 5],
/// );
/// // Shuffle with a fixed seed for reproducibility
/// let shuffled: ShufflingMoveSelector<Solution, _, _> =
///     ShufflingMoveSelector::with_seed(inner, 42);
/// assert!(!shuffled.is_never_ending());
/// ```
pub struct ShufflingMoveSelector<S, M, Inner> {
    inner: Inner,
    rng: RefCell<StdRng>,
    _phantom: PhantomData<(S, M)>,
}

// SAFETY: RefCell<StdRng> is only accessed from a single thread at a time
// via the `iter_moves` method. The Send bound on MoveSelector ensures
// the selector itself is only used from one thread.
unsafe impl<S, M, Inner: Send> Send for ShufflingMoveSelector<S, M, Inner> {}

impl<S, M, Inner> ShufflingMoveSelector<S, M, Inner> {
    /// Creates a new shuffling selector with a random seed.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner,
            rng: RefCell::new(StdRng::from_os_rng()),
            _phantom: PhantomData,
        }
    }

    /// Creates a new shuffling selector with a specific seed.
    ///
    /// Use this for reproducible shuffling in tests.
    pub fn with_seed(inner: Inner, seed: u64) -> Self {
        Self {
            inner,
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for ShufflingMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShufflingMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for ShufflingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.shuffle(&mut *self.rng.borrow_mut());
        Box::new(moves.into_iter())
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        // Shuffling collects all moves, so it's always finite
        false
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
    fn preserves_all_moves() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let shuffled = ShufflingMoveSelector::with_seed(inner, 42);

        let moves: Vec<_> = shuffled.iter_moves(&director).collect();

        // All moves are present
        assert_eq!(moves.len(), 5);
        assert_eq!(shuffled.size(&director), 5);

        // Check all values are present (order may differ)
        let values: Vec<_> = moves.iter().filter_map(|m| m.to_value().copied()).collect();
        assert!(values.contains(&10));
        assert!(values.contains(&20));
        assert!(values.contains(&30));
        assert!(values.contains(&40));
        assert!(values.contains(&50));
    }

    #[test]
    fn same_seed_produces_same_order() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner1 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

        let inner2 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let shuffled2 = ShufflingMoveSelector::with_seed(inner2, 42);

        let moves1: Vec<_> = shuffled1
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();
        let moves2: Vec<_> = shuffled2
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        assert_eq!(moves1, moves2);
    }

    #[test]
    fn different_seeds_produce_different_order() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner1 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
        );
        let shuffled1 = ShufflingMoveSelector::with_seed(inner1, 42);

        let inner2 = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
        );
        let shuffled2 = ShufflingMoveSelector::with_seed(inner2, 123);

        let moves1: Vec<_> = shuffled1
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();
        let moves2: Vec<_> = shuffled2
            .iter_moves(&director)
            .filter_map(|m| m.to_value().copied())
            .collect();

        // With 10 elements, probability of same order with different seeds is 1/10! ≈ 0
        assert_ne!(moves1, moves2);
    }
}
