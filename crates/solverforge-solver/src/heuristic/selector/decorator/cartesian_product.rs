mod cursor;
pub(crate) use cursor::CartesianProductCursor;
use cursor::CartesianScratch;

/* Cartesian product move selector.

Combines moves from two selectors and exposes borrowable sequential candidates.
The selected winner is materialized by ownership only after the search phase
chooses a stable candidate index.
*/

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Mutex;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::compose_sequential_tabu_signature;
use crate::heuristic::r#move::{
    Move, MoveTabuSignature, SequentialCompositeMove, SequentialCompositeMoveRef,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveSelectorIter,
    MoveStreamContext,
};

/// Cartesian product selector that evaluates the right child after the left
/// child on a preview solution and yields borrowable sequential candidates.
pub(crate) struct CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S>,
{
    left: Left,
    right: Right,
    require_hard_improvement: bool,
    scratch: Mutex<CartesianScratch<S, M>>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, Left, Right> CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    pub(crate) fn new(left: Left, right: Right) -> Self {
        Self {
            left,
            right,
            require_hard_improvement: false,
            scratch: Mutex::new(CartesianScratch::new()),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Left, Right> Debug for CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M> + Debug,
    Right: MoveSelector<S, M> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductSelector")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

impl<S, M, Left, Right> MoveSelector<S, M> for CartesianProductSelector<S, M, Left, Right>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Left: MoveSelector<S, M>,
    Right: MoveSelector<S, M>,
{
    type Cursor<'a>
        = CartesianProductCursor<'a, S, M, Left::Cursor<'a>, Right>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.right.validate_cursor(score_director);
        let mut scratch = self
            .scratch
            .lock()
            .expect("cartesian scratch pool must not be poisoned");
        scratch.reset();
        let rows = std::mem::take(&mut scratch.rows);
        let pairs = std::mem::take(&mut scratch.pairs);
        drop(scratch);
        CartesianProductCursor::new(
            self.require_hard_improvement,
            self.left.open_cursor_with_context(score_director, context),
            &self.right,
            context,
            rows,
            pairs,
            &self.scratch,
        )
    }

    fn validate_cursor<D: Director<S>>(&self, score_director: &D) {
        self.left.validate_cursor(score_director);
        self.right.validate_cursor(score_director);
    }

    fn iter_moves<'a, 'd, D: Director<S>>(
        &'a self,
        _score_director: &'d D,
    ) -> MoveSelectorIter<'d, S, M, Self::Cursor<'a>> {
        panic!(
            "cartesian selectors do not support owned iter_moves(); use open_cursor() and candidate() to evaluate or commit the selected borrowed sequential candidate",
        );
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.left
            .size(score_director)
            .saturating_mul(self.right.size(score_director))
    }
}

#[cfg(test)]
mod tests;
