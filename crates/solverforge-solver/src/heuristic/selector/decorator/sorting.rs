/* Sorting move selector decorator.

Sorts moves from an inner selector using a comparator function.
*/

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveSelector;

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
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Inner> SortingMoveSelector<S, M, Inner> {
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
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = M> + 'a {
        let comparator = self.comparator;
        let mut moves: Vec<M> = self.inner.open_cursor(score_director).collect();
        moves.sort_by(comparator);
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "sorting_tests.rs"]
mod tests;
