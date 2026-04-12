/* Count limit move selector decorator.

Limits the number of moves yielded from an inner selector.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveSelector;

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
    pub fn new(inner: Inner, limit: usize) -> Self {
        Self {
            inner,
            limit,
            _phantom: PhantomData,
        }
    }

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
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = M> + 'a {
        self.inner.iter_moves(score_director).take(self.limit)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director).min(self.limit)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "count_limit_tests.rs"]
mod tests;
