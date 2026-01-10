//! Caching move selector decorator.
//!
//! Caches moves from an inner selector to avoid regenerating them on each call.
//! Call `reset()` at the start of each step to clear the cache.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Caches moves from an inner selector for repeated iteration.
///
/// On the first call to `iter_moves`, collects all moves from the inner
/// selector and caches them. Subsequent calls return an iterator over
/// the cached moves. Call `reset()` to clear the cache (typically at
/// each step boundary).
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::CachingMoveSelector;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
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
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![1, 2, 3],
/// );
/// type SolChange = ChangeMove<Solution, i32>;
/// let caching: CachingMoveSelector<Solution, SolChange, _> =
///     CachingMoveSelector::new(inner);
///
/// // First call populates cache, subsequent calls use cache
/// // Call caching.reset() at step boundaries
/// ```
pub struct CachingMoveSelector<S, M, Inner> {
    inner: Inner,
    cache: RefCell<Option<Vec<M>>>,
    _phantom: PhantomData<S>,
}

impl<S, M, Inner> CachingMoveSelector<S, M, Inner> {
    /// Creates a new caching selector wrapping the given inner selector.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner,
            cache: RefCell::new(None),
            _phantom: PhantomData,
        }
    }

    /// Clears the cache, forcing re-evaluation on next `iter_moves`.
    ///
    /// Call this at the start of each step to pick up solution changes.
    pub fn reset(&self) {
        *self.cache.borrow_mut() = None;
    }

    /// Returns the inner selector.
    pub fn inner(&self) -> &Inner {
        &self.inner
    }
}

impl<S, M, Inner: Debug> Debug for CachingMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachingMoveSelector")
            .field("inner", &self.inner)
            .field("cached", &self.cache.borrow().is_some())
            .finish()
    }
}

// SAFETY: The cache is only accessed from single-threaded contexts.
// MoveSelector requires Send but iter_moves is called from one thread.
unsafe impl<S: Send, M: Send, Inner: Send> Send for CachingMoveSelector<S, M, Inner> {}

impl<S, M, Inner> MoveSelector<S, M> for CachingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        // Populate cache if empty
        {
            let mut cache = self.cache.borrow_mut();
            if cache.is_none() {
                let moves: Vec<M> = self.inner.iter_moves(score_director).collect();
                *cache = Some(moves);
            }
        }

        // Return iterator over cached moves
        let cache = self.cache.borrow();
        let moves = cache.as_ref().unwrap().clone();
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let cache = self.cache.borrow();
        if let Some(ref moves) = *cache {
            moves.len()
        } else {
            self.inner.size(score_director)
        }
    }

    fn is_never_ending(&self) -> bool {
        false // Caching makes it finite
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
    fn caches_moves_on_first_call() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30],
        );
        let caching = CachingMoveSelector::new(inner);

        // First call
        let moves1: Vec<_> = caching.iter_moves(&director).collect();
        assert_eq!(moves1.len(), 3);

        // Second call returns same (cached) moves
        let moves2: Vec<_> = caching.iter_moves(&director).collect();
        assert_eq!(moves2.len(), 3);

        // Verify same content
        assert_eq!(moves1[0].to_value(), moves2[0].to_value());
    }

    #[test]
    fn reset_clears_cache() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20],
        );
        let caching = CachingMoveSelector::new(inner);

        // Populate cache
        let _ = caching.iter_moves(&director).count();

        // Reset
        caching.reset();

        // Cache should be rebuilt on next call
        let moves: Vec<_> = caching.iter_moves(&director).collect();
        assert_eq!(moves.len(), 2);
    }

    #[test]
    fn size_uses_cache_when_available() {
        let director = create_director(vec![Task { priority: Some(1) }]);

        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30],
        );
        let caching = CachingMoveSelector::new(inner);

        // Before caching
        assert_eq!(caching.size(&director), 3);

        // Populate cache
        let _ = caching.iter_moves(&director).count();

        // After caching - uses cached size
        assert_eq!(caching.size(&director), 3);
    }

    #[test]
    fn is_never_ending_returns_false() {
        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![1],
        );
        let caching = CachingMoveSelector::new(inner);

        assert!(!caching.is_never_ending());
    }
}
