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
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        Box::new(self.inner.iter_moves(score_director).take(self.limit))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director).min(self.limit)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
