mod cursor;
pub use cursor::CartesianProductCursor;
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
    Move, MoveArena, MoveTabuSignature, SequentialCompositeMove, SequentialCompositeMoveRef,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveSelectorIter,
    MoveStreamContext,
};

/// Holds two owned move arenas and provides indexed pair iteration.
pub struct CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    arena_1: MoveArena<M1>,
    arena_2: MoveArena<M2>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M1, M2> CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    pub fn new() -> Self {
        Self {
            arena_1: MoveArena::new(),
            arena_2: MoveArena::new(),
            _phantom: PhantomData,
        }
    }

    pub fn reset(&mut self) {
        self.arena_1.reset();
        self.arena_2.reset();
    }

    pub fn populate_first<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M1>,
    {
        self.arena_1.extend(selector.iter_moves(score_director));
    }

    pub fn populate_second<D, MS>(&mut self, selector: &MS, score_director: &D)
    where
        D: Director<S>,
        MS: MoveSelector<S, M2>,
    {
        self.arena_2.extend(selector.iter_moves(score_director));
    }

    pub fn len(&self) -> usize {
        self.arena_1.len() * self.arena_2.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arena_1.is_empty() || self.arena_2.is_empty()
    }

    pub fn get_first(&self, index: usize) -> Option<&M1> {
        self.arena_1.get(index)
    }

    pub fn get_second(&self, index: usize) -> Option<&M2> {
        self.arena_2.get(index)
    }

    pub fn iter_indices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let len_1 = self.arena_1.len();
        let len_2 = self.arena_2.len();
        (0..len_1).flat_map(move |i| (0..len_2).map(move |j| (i, j)))
    }

    pub fn iter_pairs(&self) -> impl Iterator<Item = (usize, usize, &M1, &M2)> + '_ {
        self.iter_indices().filter_map(|(i, j)| {
            let m1 = self.arena_1.get(i)?;
            let m2 = self.arena_2.get(j)?;
            Some((i, j, m1, m2))
        })
    }
}

impl<S, M1, M2> Default for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M1, M2> Debug for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductArena")
            .field("arena_1_len", &self.arena_1.len())
            .field("arena_2_len", &self.arena_2.len())
            .finish()
    }
}

/// Cartesian product selector that evaluates the right child after the left
/// child on a preview solution and yields borrowable sequential candidates.
pub struct CartesianProductSelector<S, M, Left, Right>
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
    pub fn new(left: Left, right: Right) -> Self {
        Self {
            left,
            right,
            require_hard_improvement: false,
            scratch: Mutex::new(CartesianScratch::new()),
            _phantom: PhantomData,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
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

    fn append_moves<D: Director<S>>(&self, _score_director: &D, _arena: &mut MoveArena<M>) {
        panic!(
            "cartesian selectors do not support append_moves(); use open_cursor() and candidate() to evaluate or commit the selected borrowed sequential candidate",
        );
    }
}

#[cfg(test)]
mod tests;
