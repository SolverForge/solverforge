//! Filtering move selector decorator.
//!
//! Filters moves from an inner selector based on a predicate function.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
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
    _phantom: PhantomData<(S, M)>,
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
    S::Score: Score,
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = M> + 'a>
    where
        C: ConstraintSet<S, S::Score>,
    {
        let predicate = self.predicate;
        Box::new(self.inner.iter_moves(score_director).filter(predicate))
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}
