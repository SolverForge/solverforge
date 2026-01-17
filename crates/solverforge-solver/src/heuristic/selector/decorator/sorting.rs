//! Sorting move selector decorator.
//!
//! Sorts moves from an inner selector using a comparator function.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

/// Sorts moves from an inner selector using a comparator function.
///
/// Collects all moves from the inner selector and yields them in sorted order.
/// Uses a function pointer for zero-erasure comparison.
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
    M: Move<S>,
    Inner: MoveSelector<S, M>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a> {
        let comparator = self.comparator;
        let mut moves: Vec<M> = self.inner.iter_moves(score_director).collect();
        moves.sort_by(comparator);
        Box::new(moves.into_iter())
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
