/* Vec-backed union move selector for config-driven selector composition.

Unlike `UnionMoveSelector` (which combines exactly two selectors), `VecUnionSelector`
holds a `Vec<Leaf>` of selectors and lazily traverses them in order. This is the
backbone of config-driven solver construction where the number of selectors is
determined at runtime from `solver.toml`.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_config::UnionSelectionOrder;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Combines moves from an arbitrary number of leaf selectors into a single stream.
pub struct VecUnionSelector<S, M, Leaf> {
    selectors: Vec<Leaf>,
    selection_order: UnionSelectionOrder,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, Leaf> VecUnionSelector<S, M, Leaf> {
    pub fn new(selectors: Vec<Leaf>) -> Self {
        Self::with_selection_order(selectors, UnionSelectionOrder::Sequential)
    }

    pub fn with_selection_order(
        selectors: Vec<Leaf>,
        selection_order: UnionSelectionOrder,
    ) -> Self {
        Self {
            selectors,
            selection_order,
            _phantom: PhantomData,
        }
    }

    pub fn selectors(&self) -> &[Leaf] {
        &self.selectors
    }

    pub fn into_selectors(self) -> Vec<Leaf> {
        self.selectors
    }

    pub fn selection_order(&self) -> UnionSelectionOrder {
        self.selection_order
    }
}

impl<S, M, Leaf: Debug> Debug for VecUnionSelector<S, M, Leaf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VecUnionSelector")
            .field("selectors", &self.selectors)
            .field("selection_order", &self.selection_order)
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
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        VecUnionMoveCursor::new(
            self.selectors
                .iter()
                .map(|selector| selector.open_cursor_with_context(score_director, context))
                .collect(),
            self.selection_order,
            context,
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
    selection_order: UnionSelectionOrder,
    exhausted: Vec<bool>,
    live_cursor_count: usize,
    discovered: Vec<(usize, CandidateId)>,
    cursor_offset: usize,
    cursor_stride: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn new(
        cursors: Vec<C>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
    ) -> Self {
        let live_cursor_count = cursors.len();
        let cursor_offset = match selection_order {
            UnionSelectionOrder::RotatingRoundRobin | UnionSelectionOrder::StratifiedRandom => {
                context.start_offset(live_cursor_count, 0xA11C_E5E1_EC70_0001)
            }
            UnionSelectionOrder::Sequential | UnionSelectionOrder::RoundRobin => 0,
        };
        let cursor_stride = match selection_order {
            UnionSelectionOrder::StratifiedRandom => {
                context.stride(live_cursor_count, 0xA11C_E5E1_EC70_0002)
            }
            UnionSelectionOrder::Sequential
            | UnionSelectionOrder::RoundRobin
            | UnionSelectionOrder::RotatingRoundRobin => 1,
        };
        let current_cursor = match selection_order {
            UnionSelectionOrder::StratifiedRandom => 0,
            _ => cursor_offset,
        };
        Self {
            cursors,
            current_cursor,
            selection_order,
            exhausted: vec![false; live_cursor_count],
            live_cursor_count,
            discovered: Vec::new(),
            cursor_offset,
            cursor_stride,
            _phantom: PhantomData,
        }
    }

    fn next_sequential_candidate(&mut self) -> Option<CandidateId> {
        if self.current_cursor >= self.cursors.len() {
            return None;
        }
        let cursor_index = self.current_cursor;
        let Some(child_index) = self.cursors[cursor_index].next_candidate() else {
            self.current_cursor += 1;
            return self.next_sequential_candidate();
        };
        Some(self.push_discovered(cursor_index, child_index))
    }

    fn next_round_robin_candidate(&mut self) -> Option<CandidateId> {
        if self.live_cursor_count == 0 {
            return None;
        }

        while self.live_cursor_count > 0 {
            let cursor_index = self.current_cursor % self.cursors.len();
            self.current_cursor = (self.current_cursor + 1) % self.cursors.len();
            if self.exhausted[cursor_index] {
                continue;
            }

            if let Some(child_index) = self.cursors[cursor_index].next_candidate() {
                return Some(self.push_discovered(cursor_index, child_index));
            }

            self.exhausted[cursor_index] = true;
            self.live_cursor_count -= 1;
        }

        None
    }

    fn next_stratified_candidate(&mut self) -> Option<CandidateId> {
        if self.live_cursor_count == 0 {
            return None;
        }

        while self.live_cursor_count > 0 {
            let cursor_index = (self.cursor_offset + self.current_cursor * self.cursor_stride)
                % self.cursors.len();
            self.current_cursor = (self.current_cursor + 1) % self.cursors.len();
            if self.exhausted[cursor_index] {
                continue;
            }

            if let Some(child_index) = self.cursors[cursor_index].next_candidate() {
                return Some(self.push_discovered(cursor_index, child_index));
            }

            self.exhausted[cursor_index] = true;
            self.live_cursor_count -= 1;
        }

        None
    }

    fn push_discovered(&mut self, cursor_index: usize, child_index: CandidateId) -> CandidateId {
        let global_id = CandidateId::new(self.discovered.len());
        self.discovered.push((cursor_index, child_index));
        self.cursors[cursor_index]
            .candidate(child_index)
            .expect("vec union candidate must remain valid");
        global_id
    }
}

impl<S, M, C> MoveCursor<S, M> for VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self.selection_order {
            UnionSelectionOrder::Sequential => self.next_sequential_candidate(),
            UnionSelectionOrder::RoundRobin | UnionSelectionOrder::RotatingRoundRobin => {
                self.next_round_robin_candidate()
            }
            UnionSelectionOrder::StratifiedRandom => self.next_stratified_candidate(),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index.index())?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].take_candidate(child_index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.discovered
            .get(index.index())
            .map(|(cursor_index, _)| *cursor_index)
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
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

#[cfg(test)]
#[path = "vec_union/tests.rs"]
mod tests;
