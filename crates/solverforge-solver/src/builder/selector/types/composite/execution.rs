//! Retained recursive composition state and resource-lending cursor sources.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};
use crate::phase::localsearch::MoveCursorSource;

use super::state::{
    new_stream_state, open_cursor_with_owned_stream_state, SelectorCompositionStreamState,
};
use super::{
    SelectorComposition, SelectorCompositionCursor, SequentialMoveCarrier, StatefulComposedFlat,
};

/// Retained state for one frozen recursive composition tree.
///
/// The resource type is part of the composition's static contract but is not
/// stored here. A runner can therefore lend one solve-owned resource to every
/// phase source without cloning it or tying a phase's lifetime to the runner.
pub(crate) struct SelectorCompositionState<S, M, Flat, FlatState, Resources = ()>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    selector: SelectorComposition<S, M, Flat, FlatState>,
    stream_state: Option<SelectorCompositionStreamState<FlatState>>,
    _resources: PhantomData<fn() -> Resources>,
}

impl<S, M, Flat, FlatState, Resources> SelectorCompositionState<S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    pub(crate) fn new(selector: SelectorComposition<S, M, Flat, FlatState>) -> Self {
        Self {
            stream_state: Some(new_stream_state(&selector)),
            selector,
            _resources: PhantomData,
        }
    }

    /// Lends this retained tree one mutable solve resource for a phase call.
    ///
    /// The returned source opens ordinary phase cursors through the same
    /// recursive composition kernel as the owned execution below. Its cursor
    /// returns only the state tree on drop; the runner keeps the resource for
    /// the next reached phase and for pause/resume.
    pub(crate) fn borrowed_source<'a>(
        &'a mut self,
    ) -> SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources> {
        SelectorCompositionBorrowedSource { state: self }
    }

    fn open_cursor_with_resources<'state, D: Director<S>>(
        &'state mut self,
        resources: &mut Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> SelectorCompositionBorrowedCursor<'state, S, M, Flat, FlatState, Resources> {
        let stream_state = self
            .stream_state
            .take()
            .expect("a composition state allows only one live cursor");
        let cursor = open_cursor_with_owned_stream_state(
            &self.selector,
            stream_state,
            resources,
            score_director,
            context,
        );
        SelectorCompositionBorrowedCursor {
            state_slot: &mut self.stream_state,
            cursor: Some(cursor),
        }
    }
}

impl<S, M, Flat, FlatState, Resources> Debug
    for SelectorCompositionState<S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorCompositionState")
            .field("selector", &self.selector)
            .field("stream_state_available", &self.stream_state.is_some())
            .finish()
    }
}

/// A phase-facing borrowed source over retained composition state.
///
/// `CompiledRuntimePhaseRunner` creates this only while it executes a
/// compiled local-search phase. It captures no solve resource; the one
/// runner-owned value is lent by the shared [`MoveCursorSource`] contract
/// when a cursor opens.
pub(crate) struct SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    state: &'a mut SelectorCompositionState<S, M, Flat, FlatState, Resources>,
}

impl<S, M, Flat, FlatState, Resources> Debug
    for SelectorCompositionBorrowedSource<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SelectorCompositionBorrowedSource")
            .field(&self.state)
            .finish()
    }
}

impl<'state, S, M, Flat, FlatState, Resources> MoveCursorSource<S, M>
    for SelectorCompositionBorrowedSource<'state, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
    Resources: Send,
{
    /// The runner owns this resource for the whole solve and lends it to the
    /// shared phase loop at each candidate pull. The source and its cursor
    /// retain only selector state.
    type Resources = Resources;

    type Cursor<'a>
        = SelectorCompositionBorrowedCursor<'a, S, M, Flat, FlatState, Resources>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(
        &'a mut self,
        resources: &mut Self::Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.state
            .open_cursor_with_resources(resources, score_director, context)
    }
}

/// A live cursor that returns its exact tree state. It owns no mutable
/// resource; the shared phase loop lends a resource only to a reachable
/// `next_candidate_with_resources` pull.
pub(crate) struct SelectorCompositionBorrowedCursor<'state, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    state_slot: &'state mut Option<SelectorCompositionStreamState<FlatState>>,
    cursor: Option<SelectorCompositionCursor<'state, S, M, Flat, FlatState, Resources>>,
}

impl<S, M, Flat, FlatState, Resources> ResourceMoveCursor<S, M, Resources>
    for SelectorCompositionBorrowedCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId> {
        self.cursor
            .as_mut()
            .expect("live composition cursor must retain its inner cursor")
            .next_candidate_with_resources(resources)
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.cursor
            .as_ref()
            .expect("live composition cursor must retain its inner cursor")
            .candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        self.cursor
            .as_mut()
            .expect("live composition cursor must retain its inner cursor")
            .take_candidate(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.cursor
            .as_mut()
            .expect("live composition cursor must retain its inner cursor")
            .apply_owned_candidate(index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        self.cursor
            .as_mut()
            .expect("live composition cursor must retain its inner cursor")
            .release_candidate(index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.cursor
            .as_ref()
            .expect("live composition cursor must retain its inner cursor")
            .selector_index(index)
    }
}

impl<S, M, Flat, FlatState, Resources> Drop
    for SelectorCompositionBorrowedCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    fn drop(&mut self) {
        let cursor = self
            .cursor
            .take()
            .expect("live composition cursor must retain its inner cursor");
        assert!(
            self.state_slot.is_none(),
            "composition stream state must remain moved while its cursor is live"
        );
        *self.state_slot = Some(cursor.into_stream_state());
    }
}
