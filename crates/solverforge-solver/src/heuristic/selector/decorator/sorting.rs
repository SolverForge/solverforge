//! Sorting move selector decorator.
//!
//! Sorts moves from an inner selector using a comparator function.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Sorts moves from an inner selector using a comparator function.
///
/// Collects all moves from the inner selector and yields them in sorted order.
/// Uses a function pointer for zero-erasure comparison.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::selector::decorator::SortingMoveSelector;
/// use solverforge_solver::heuristic::selector::MoveSelector;
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_scoring::ScoreDirector;
/// use std::cmp::Ordering;
///
/// #[derive(Clone, Debug)]
/// struct Solution { value: i32, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Simple mock selector
/// #[derive(Debug)]
/// struct MockSelector;
/// impl MoveSelector<Solution, ChangeMove<Solution, i32>> for MockSelector {
///     fn iter_moves<'a, C>(&'a self, _: &'a ScoreDirector<Solution, C>)
///         -> Box<dyn Iterator<Item = ChangeMove<Solution, i32>> + 'a>
///         where C: ConstraintSet<Solution, SimpleScore> { Box::new(std::iter::empty()) }
///     fn size<C>(&self, _: &ScoreDirector<Solution, C>) -> usize
///         where C: ConstraintSet<Solution, SimpleScore> { 4 }
///     fn is_never_ending(&self) -> bool { false }
/// }
///
/// // Sort by target value descending
/// fn by_value_desc(a: &ChangeMove<Solution, i32>, b: &ChangeMove<Solution, i32>) -> Ordering {
///     b.to_value().cmp(&a.to_value())
/// }
///
/// let inner = MockSelector;
/// let sorted: SortingMoveSelector<Solution, _, _> =
///     SortingMoveSelector::new(inner, by_value_desc);
/// assert!(!sorted.is_never_ending());
/// ```
pub struct SortingMoveSelector<S, M, Inner> {
    inner: Inner,
    comparator: fn(&M, &M) -> Ordering,
    _phantom: PhantomData<(S, M)>,
}

impl<S, M, Inner> SortingMoveSelector<S, M, Inner> {
    /// Creates a new sorting selector with the given comparator.
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
        let comparator = self.comparator;
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.sort_by(comparator);
        Box::new(moves.into_iter())
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
