//! Count limit move selector decorator.
//!
//! Limits the number of moves yielded from an inner selector.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Limits the number of moves yielded from an inner selector.
///
/// Useful for restricting the search space in large-scale problems
/// or when using never-ending selectors.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::SelectedCountLimitMoveSelector;
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
///     get_priority, set_priority, 0, "priority", vec![1, 2, 3, 4, 5],
/// );
/// // Only consider first 3 moves per step
/// let limited: SelectedCountLimitMoveSelector<Solution, ChangeMove<Solution, i32>, _> =
///     SelectedCountLimitMoveSelector::new(inner, 3);
/// assert_eq!(limited.limit(), 3);
/// ```
pub struct SelectedCountLimitMoveSelector<S, M, Inner> {
    inner: Inner,
    limit: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> SelectedCountLimitMoveSelector<S, M, Inner> {
    /// Creates a new count-limited selector.
    pub fn new(inner: Inner, limit: usize) -> Self {
        Self {
            inner,
            limit,
            _phantom: PhantomData,
        }
    }

    /// Returns the configured limit.
    pub fn limit(&self) -> usize {
        self.limit
    }
}

impl<S, M, Inner: Debug> Debug for SelectedCountLimitMoveSelector<S, M, Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedCountLimitMoveSelector")
            .field("inner", &self.inner)
            .field("limit", &self.limit)
            .finish()
    }
}

impl<S, M, Inner> MoveSelector<S, M> for SelectedCountLimitMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        self.inner.iter_moves(score_director).take(self.limit)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director).min(self.limit)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{create_director, get_priority, set_priority, Task};
    use super::*;
    use crate::heuristic::selector::ChangeMoveSelector;

    #[test]
    fn limits_move_count() {
        let director = create_director(vec![Task { priority: Some(1) }]);
        let inner = ChangeMoveSelector::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30, 40, 50],
        );
        let limited = SelectedCountLimitMoveSelector::new(inner, 3);

        let moves: Vec<_> = limited.iter_moves(&director).collect();
        assert_eq!(moves.len(), 3);
        assert_eq!(limited.size(&director), 3);
    }

    #[test]
    fn returns_all_when_under_limit() {
        let director = create_director(vec![Task { priority: Some(1) }]);
        let inner =
            ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);
        let limited = SelectedCountLimitMoveSelector::new(inner, 10);

        let moves: Vec<_> = limited.iter_moves(&director).collect();
        assert_eq!(moves.len(), 2);
        assert_eq!(limited.size(&director), 2);
    }

    #[test]
    fn zero_limit_yields_nothing() {
        let director = create_director(vec![Task { priority: Some(1) }]);
        let inner =
            ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);
        let limited = SelectedCountLimitMoveSelector::new(inner, 0);

        let moves: Vec<_> = limited.iter_moves(&director).collect();
        assert!(moves.is_empty());
        assert_eq!(limited.size(&director), 0);
    }
}
