/* Vec-backed union move selector for config-driven selector composition.

Unlike `UnionMoveSelector` (which combines exactly two selectors), `VecUnionSelector`
holds a `Vec<Leaf>` of selectors and lazily traverses them in order. This is the
backbone of config-driven solver construction where the number of selectors is
determined at runtime from `solver.toml`.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor, MoveSelector};

/// Combines moves from an arbitrary number of leaf selectors into a single stream.
pub struct VecUnionSelector<S, M, Leaf> {
    selectors: Vec<Leaf>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Leaf> VecUnionSelector<S, M, Leaf> {
    pub fn new(selectors: Vec<Leaf>) -> Self {
        Self {
            selectors,
            _phantom: PhantomData,
        }
    }

    pub fn selectors(&self) -> &[Leaf] {
        &self.selectors
    }

    pub fn into_selectors(self) -> Vec<Leaf> {
        self.selectors
    }
}

impl<S, M, Leaf: Debug> Debug for VecUnionSelector<S, M, Leaf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VecUnionSelector")
            .field("selectors", &self.selectors)
            .finish()
    }
}

impl<S, M, Leaf> MoveSelector<S, M> for VecUnionSelector<S, M, Leaf>
where
    S: PlanningSolution,
    M: Move<S>,
    Leaf: MoveSelector<S, M>,
{
    type Cursor<'a>
        = VecUnionMoveCursor<S, M, Leaf::Cursor<'a>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        VecUnionMoveCursor::new(
            self.selectors
                .iter()
                .map(|selector| selector.open_cursor(score_director))
                .collect(),
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.selectors.iter().map(|s| s.size(score_director)).sum()
    }
}

pub struct VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursors: Vec<C>,
    current_cursor: usize,
    discovered: Vec<(usize, usize)>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(cursors: Vec<C>) -> Self {
        Self {
            cursors,
            current_cursor: 0,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> MoveCursor<S, M> for VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<(usize, MoveCandidateRef<'_, S, M>)> {
        if self.current_cursor >= self.cursors.len() {
            return None;
        }
        let cursor_index = self.current_cursor;
        let Some((child_index, _)) = self.cursors[cursor_index].next_candidate() else {
            self.current_cursor += 1;
            return self.next_candidate();
        };
        let global_index = self.discovered.len();
        self.discovered.push((cursor_index, child_index));
        let candidate = self.cursors[cursor_index]
            .candidate(child_index)
            .expect("vec union candidate must remain valid");
        Some((global_index, candidate))
    }

    fn candidate(&self, index: usize) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index)?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: usize) -> M {
        let (cursor_index, child_index) = self.discovered[index];
        self.cursors[cursor_index].take_candidate(child_index)
    }
}

impl<S, M, C> Iterator for VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        let index = {
            let (index, _) = self.next_candidate()?;
            index
        };
        Some(self.take_candidate(index))
    }
}
