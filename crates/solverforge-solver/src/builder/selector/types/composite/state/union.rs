//! Recursive union cursor backed by the canonical scheduler.

use solverforge_config::UnionSelectionOrder;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::ResourceVecUnionMoveCursor;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};

use super::{SelectorCompositionCursor, SelectorCompositionStreamState};
use crate::builder::selector::types::composite::{SequentialMoveCarrier, StatefulComposedFlat};

/// One recursive union over already-opened child composition cursors.
///
/// The cursor owns each child's moved stream state. On drop/selection-tree
/// return it consumes the one canonical union cursor and rebuilds the exact
/// state tree in frozen child order.
pub(crate) struct SelectorCompositionUnionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    cursor: ResourceVecUnionMoveCursor<
        S,
        M,
        SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>,
    >,
}

impl<'a, S, M, Flat, FlatState, Resources>
    SelectorCompositionUnionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    pub(super) fn new(
        cursors: Vec<SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            cursor: ResourceVecUnionMoveCursor::new(cursors, selection_order, context, weights),
        }
    }

    pub(super) fn into_stream_state(self) -> SelectorCompositionStreamState<FlatState>
    where
        M: SequentialMoveCarrier<S>,
    {
        SelectorCompositionStreamState::Union(
            self.cursor
                .into_cursors()
                .into_iter()
                .map(SelectorCompositionCursor::into_stream_state)
                .collect(),
        )
    }
}

impl<S, M, Flat, FlatState, Resources> ResourceMoveCursor<S, M, Resources>
    for SelectorCompositionUnionCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId> {
        self.cursor.next_candidate_with_resources(resources)
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.cursor.candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        self.cursor.take_candidate(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.cursor.apply_owned_candidate(index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        self.cursor.release_candidate(index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.cursor.selector_index(index)
    }
}
