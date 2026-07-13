//! Dynamic public adapter for the canonical streamed list-change cursor.

use std::fmt::{self, Debug};

use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicListChangeMove, MoveArena};
use crate::heuristic::selector::list_kernel::{
    ChangeCursor, DynamicChangeEmitter, SelectedListOwners, DYNAMIC_CHANGE_SALTS,
};

use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Dynamic list change retains its public selector/move surface while sharing
/// the canonical coordinate cursor with static list change.
pub struct DynamicListChangeMoveSelector<S> {
    slot: DynamicListVariableSlot<S>,
}

/// Public cursor facade; the generic runtime emitter remains crate-private.
pub struct DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    inner: ChangeCursor<S, DynamicChangeEmitter<S>>,
}

impl<S> DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn new(inner: ChangeCursor<S, DynamicChangeEmitter<S>>) -> Self {
        Self { inner }
    }
}

impl<S> MoveCursor<S, DynamicListChangeMove<S>> for DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, DynamicListChangeMove<S>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> DynamicListChangeMove<S> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<DynamicListChangeMove<S>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, DynamicListChangeMove<S>>) -> bool,
    ) -> Option<DynamicListChangeMove<S>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S> Iterator for DynamicListChangeMoveCursor<S>
where
    S: PlanningSolution,
{
    type Item = DynamicListChangeMove<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S> DynamicListChangeMoveSelector<S> {
    pub fn new(slot: DynamicListVariableSlot<S>) -> Self {
        Self { slot }
    }
}

impl<S> Debug for DynamicListChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicListChangeMoveSelector")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicListChangeMove<S>> for DynamicListChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    type Cursor<'a>
        = DynamicListChangeMoveCursor<S>
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
        let solution = score_director.working_solution();
        let canonical_entities = (0..self.slot.entity_count(solution)).collect::<Vec<_>>();
        let canonical_route_lens = canonical_entities
            .iter()
            .map(|&entity| self.slot.list_len(solution, entity))
            .collect::<Vec<_>>();
        let salt = DYNAMIC_CHANGE_SALTS.entity ^ self.slot.descriptor_index() as u64;
        let entities = (0..canonical_entities.len())
            .map(|offset| {
                canonical_entities[context.selection_index(offset, canonical_entities.len(), salt)]
            })
            .collect::<Vec<_>>();
        let route_lens = (0..canonical_route_lens.len())
            .map(|offset| {
                canonical_route_lens
                    [context.selection_index(offset, canonical_route_lens.len(), salt)]
            })
            .collect::<Vec<_>>();
        DynamicListChangeMoveCursor::new(ChangeCursor::new(
            DynamicChangeEmitter::new(self.slot.clone()),
            entities,
            route_lens,
            context,
            DYNAMIC_CHANGE_SALTS,
            SelectedListOwners::Absent,
            self.slot.descriptor_index(),
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let route_lens = (0..self.slot.entity_count(solution))
            .map(|entity| self.slot.list_len(solution, entity))
            .collect::<Vec<_>>();
        let entity_count = route_lens.len();
        let total_elements: usize = route_lens.iter().sum();
        route_lens
            .iter()
            .map(|&source_len| {
                let intra_moves = source_len * source_len.saturating_sub(1);
                let inter_destinations =
                    total_elements.saturating_sub(source_len) + entity_count.saturating_sub(1);
                intra_moves + source_len * inter_destinations
            })
            .sum()
    }

    fn append_moves<D: Director<S>>(
        &self,
        score_director: &D,
        arena: &mut MoveArena<DynamicListChangeMove<S>>,
    ) {
        let mut cursor = self.open_cursor(score_director);
        while let Some(id) = cursor.next_candidate() {
            arena.push(cursor.take_candidate(id));
        }
    }
}
