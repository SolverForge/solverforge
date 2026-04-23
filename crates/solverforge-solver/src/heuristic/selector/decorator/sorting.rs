/* Sorting move selector decorator.

Sorts moves from an inner selector using a comparator function.
*/

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor, MoveSelector};

use super::indexed_cursor::IndexedMoveCursor;

/// Sorts moves from an inner selector using a comparator function.
///
/// Collects all moves from the inner selector and yields them in sorted order.
/// Uses a function pointer for zero-erasure comparison.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::SortingMoveSelector;
/// use solverforge_solver::heuristic::selector::move_selector::MoveCandidateRef;
/// use solverforge_solver::heuristic::selector::{ChangeMoveSelector, MoveSelector};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
/// use std::cmp::Ordering;
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
/// // Sort by target value descending
/// fn by_value_desc(
///     a: MoveCandidateRef<'_, Solution, ChangeMove<Solution, i32>>,
///     b: MoveCandidateRef<'_, Solution, ChangeMove<Solution, i32>>,
/// ) -> Ordering {
///     match (a, b) {
///         (MoveCandidateRef::Borrowed(a), MoveCandidateRef::Borrowed(b)) => {
///             b.to_value().cmp(&a.to_value())
///         }
///         _ => Ordering::Equal,
///     }
/// }
///
/// let inner = ChangeMoveSelector::simple(
///     get_priority, set_priority, 0, "priority", vec![30, 10, 50, 20],
/// );
/// let sorted: SortingMoveSelector<Solution, _, _> =
///     SortingMoveSelector::new(inner, by_value_desc);
/// assert!(!sorted.is_never_ending());
/// ```
pub struct SortingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    inner: Inner,
    comparator: for<'a> fn(MoveCandidateRef<'a, S, M>, MoveCandidateRef<'a, S, M>) -> Ordering,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> SortingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub fn new(
        inner: Inner,
        comparator: for<'a> fn(MoveCandidateRef<'a, S, M>, MoveCandidateRef<'a, S, M>) -> Ordering,
    ) -> Self {
        Self {
            inner,
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner: Debug> Debug for SortingMoveSelector<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
{
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
    type Cursor<'a>
        = IndexedMoveCursor<S, M, Inner::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let mut inner = self.inner.open_cursor(score_director);
        let comparator = self.comparator;
        let mut indices = Vec::new();
        while let Some((child_index, _)) = inner.next_candidate() {
            indices.push(child_index);
        }
        indices.sort_by(|left, right| {
            comparator(
                inner
                    .candidate(*left)
                    .expect("sorting candidate must remain valid"),
                inner
                    .candidate(*right)
                    .expect("sorting candidate must remain valid"),
            )
        });
        IndexedMoveCursor::new(inner, indices)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests;
