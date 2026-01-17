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
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        let predicate = self.predicate;
        Box::new(self.inner.iter_moves(score_director).filter(predicate))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.inner.is_never_ending()
    }
}
