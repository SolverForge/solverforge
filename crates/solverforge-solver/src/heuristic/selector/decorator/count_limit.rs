//! Count limit move selector decorator.
//!
//! Limits the number of moves yielded from an inner selector.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
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
/// use solverforge_solver::heuristic::selector::MoveSelector;
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_scoring::ScoreDirector;
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
/// // Simple mock selector that yields a fixed number of moves
/// #[derive(Debug)]
/// struct MockSelector { count: usize }
/// impl MoveSelector<Solution, ChangeMove<Solution, i32>> for MockSelector {
///     fn iter_moves<'a, C>(&'a self, _: &'a ScoreDirector<Solution, C>)
///         -> Box<dyn Iterator<Item = ChangeMove<Solution, i32>> + 'a>
///         where C: ConstraintSet<Solution, SimpleScore> { Box::new(std::iter::empty()) }
///     fn size<C>(&self, _: &ScoreDirector<Solution, C>) -> usize
///         where C: ConstraintSet<Solution, SimpleScore> { self.count }
///     fn is_never_ending(&self) -> bool { false }
/// }
///
/// let inner = MockSelector { count: 5 };
/// // Only consider first 3 moves per step
/// let limited: SelectedCountLimitMoveSelector<Solution, ChangeMove<Solution, i32>, _> =
///     SelectedCountLimitMoveSelector::new(inner, 3);
/// assert_eq!(limited.limit(), 3);
/// ```
pub struct SelectedCountLimitMoveSelector<S, M, Inner> {
    inner: Inner,
    limit: usize,
    _phantom: PhantomData<(S, M)>,
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
        Box::new(self.inner.iter_moves(score_director).take(self.limit))
    }

    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.inner.size(score_director).min(self.limit)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
