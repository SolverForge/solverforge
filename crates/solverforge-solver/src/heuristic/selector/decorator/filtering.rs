/* Filtering move selector decorator.

Filters moves from an inner selector based on a predicate function.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor, MoveSelector};

use super::indexed_cursor::IndexedMoveCursor;

/// Filters moves from an inner selector using a predicate function.
///
/// Only moves for which the predicate returns `true` are yielded.
/// Uses a function pointer for zero-erasure filtering.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::FilteringMoveSelector;
/// use solverforge_solver::heuristic::selector::move_selector::MoveCandidateRef;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_priority(s: &Solution, i: usize) -> Option<i32> { s.tasks.get(i).and_then(|t| t.priority) }
/// fn set_priority(s: &mut Solution, i: usize, v: Option<i32>) { if let Some(t) = s.tasks.get_mut(i) { t.priority = v; } }
///
/// // Filter to only high-priority moves (value > 50)
/// fn high_priority_filter(
///     m: MoveCandidateRef<'_, Solution, ChangeMove<Solution, i32>>,
/// ) -> bool {
///     matches!(m, MoveCandidateRef::Borrowed(mov) if mov.to_value().is_some_and(|v| *v > 50))
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![10, 60, 80],
/// );
/// let filtered: FilteringMoveSelector<Solution, _, _> =
///     FilteringMoveSelector::new(inner, high_priority_filter);
/// assert!(!filtered.is_never_ending());
/// ```
pub struct FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    inner: Inner,
    predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(inner: Inner, predicate: for<'a> fn(MoveCandidateRef<'a, S, M>) -> bool) -> Self {
        Self {
            inner,
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for FilteringMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
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
    type Cursor<'a>
        = IndexedMoveCursor<S, M, Inner::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let mut inner = self.inner.open_cursor(score_director);
        let predicate = self.predicate;
        let mut indices = Vec::new();
        while let Some((child_index, candidate)) = inner.next_candidate() {
            if predicate(candidate) {
                indices.push(child_index);
            }
        }
        IndexedMoveCursor::new(inner, indices)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}

#[cfg(test)]
mod tests;
