//! Filtering move selector decorator.
//!
//! Filters moves from an inner selector based on a predicate function.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Filters moves from an inner selector using a predicate function.
///
/// Only moves for which the predicate returns `true` are yielded.
/// Uses a function pointer for zero-erasure filtering.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::FilteringMoveSelector;
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
/// // Filter to only high-priority moves (value > 50)
/// fn high_priority_filter(m: &ChangeMove<Solution, i32>) -> bool {
///     m.to_value().map_or(false, |v| *v > 50)
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![10, 60, 80],
/// );
/// let filtered: FilteringMoveSelector<Solution, _, _> =
///     FilteringMoveSelector::new(inner, high_priority_filter);
/// assert!(!filtered.is_never_ending());
/// ```
pub struct FilteringMoveSelector<S, M, Inner> {
    inner: Inner,
    predicate: fn(&M) -> bool,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> FilteringMoveSelector<S, M, Inner> {
    /// Creates a new filtering selector with the given predicate.
    pub fn new(inner: Inner, predicate: fn(&M) -> bool) -> Self {
        Self {
            inner,
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for FilteringMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteringMoveSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        let predicate = self.predicate;
        self.inner.iter_moves(score_director).filter(predicate)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{
        create_director, get_priority, set_priority, Task, TaskSolution,
    };
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::ChangeMoveSelector;

    fn high_value_filter(m: &ChangeMove<TaskSolution, i32>) -> bool {
        m.to_value().is_some_and(|v| *v > 50)
    }

    #[test]
    fn filters_moves_by_predicate() {
        let director = create_director(vec![Task { priority: Some(1) }]);
        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 60, 80, 30],
        );
        let filtered = FilteringMoveSelector::new(inner, high_value_filter);

        let moves: Vec<_> = filtered.iter_moves(&director).collect();
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].to_value(), Some(&60));
        assert_eq!(moves[1].to_value(), Some(&80));
    }

    #[test]
    fn empty_when_no_moves_pass() {
        let director = create_director(vec![Task { priority: Some(1) }]);
        let inner =
            ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
        let filtered = FilteringMoveSelector::new(inner, high_value_filter);

        let moves: Vec<_> = filtered.iter_moves(&director).collect();
        assert!(moves.is_empty());
    }
}
