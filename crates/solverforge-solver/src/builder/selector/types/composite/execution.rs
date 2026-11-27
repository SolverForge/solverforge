//! Retained indexed composition state and resource-lending cursor sources.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};
use crate::phase::localsearch::{CursorResources, MoveCursorSource};

use super::state::{
    new_stream_state, open_cursor_with_owned_stream_state, SelectorCompositionStreamState,
};
use super::{
    ComposedFlat, SelectorComposition, SelectorCompositionCursor, SequentialMoveCarrier,
    StatefulComposedFlat,
};

/// Retained state for one frozen indexed composition graph.
///
/// Neither the move carrier nor the resource type is stored here. A runner can
/// therefore keep this state for the whole solve, lend one solve-owned
/// resource to every phase source, and hand out moves that borrow the frozen
/// declaration for exactly one phase call.
pub(crate) struct SelectorCompositionState<S, Flat, FlatState, Resources = ()>
where
    S: PlanningSolution,
    Flat: ComposedFlat<S, FlatState>,
{
    selector: SelectorComposition<S, Flat, FlatState>,
    stream_state: Option<SelectorCompositionStreamState<FlatState>>,
    _resources: PhantomData<fn() -> Resources>,
}

impl<S, Flat, FlatState, Resources> SelectorCompositionState<S, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    Flat: ComposedFlat<S, FlatState>,
{
    pub(crate) fn new(selector: SelectorComposition<S, Flat, FlatState>) -> Self {
        Self {
            stream_state: Some(new_stream_state(&selector)),
            selector,
            _resources: PhantomData,
        }
    }

    /// Discards retained stream state derived from an earlier working
    /// solution, such as grouped assignment bases, after the problem changed.
    pub(crate) fn reset_stream_state(&mut self) {
        self.stream_state = Some(new_stream_state(&self.selector));
    }

    /// Lends this retained arena for one phase call.
    ///
    /// The returned source shares the frozen declaration and owns the
    /// stream-state slot for `'a`, so every move it yields may borrow the
    /// declaration for the whole phase call. Its cursor returns only the state
    /// arena on drop; the runner keeps the resource for the next reached phase
    /// and for pause/resume.
    pub(crate) fn borrowed_source<'a, M>(
        &'a mut self,
    ) -> SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources>
    where
        M: SequentialMoveCarrier<S>,
        Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
    {
        SelectorCompositionBorrowedSource {
            selector: &self.selector,
            stream_state: &mut self.stream_state,
            _marker: PhantomData,
        }
    }

    pub(crate) fn open_cursor_with_resources<'a, M, D: Director<S>>(
        &'a mut self,
        resources: &mut Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> SelectorCompositionBorrowedCursor<'a, 'a, S, M, Flat, FlatState, Resources>
    where
        M: SequentialMoveCarrier<S>,
        Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
    {
        open_borrowed_cursor(
            &self.selector,
            &mut self.stream_state,
            resources,
            score_director,
            context,
        )
    }
}

impl<S, Flat, FlatState, Resources> Debug
    for SelectorCompositionState<S, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    Flat: ComposedFlat<S, FlatState> + Debug,
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
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
{
    selector: &'a SelectorComposition<S, Flat, FlatState>,
    stream_state: &'a mut Option<SelectorCompositionStreamState<FlatState>>,
    _marker: PhantomData<fn() -> (M, Resources)>,
}

impl<'a, S, M, Flat, FlatState, Resources>
    SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
{
    pub(crate) fn open_cursor_with_resources<'state, D: Director<S>>(
        &'state mut self,
        resources: &mut Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> SelectorCompositionBorrowedCursor<'state, 'a, S, M, Flat, FlatState, Resources> {
        open_borrowed_cursor(
            self.selector,
            self.stream_state,
            resources,
            score_director,
            context,
        )
    }
}

fn open_borrowed_cursor<'state, 'a, S, M, Flat, FlatState, Resources, D>(
    selector: &'a SelectorComposition<S, Flat, FlatState>,
    state_slot: &'state mut Option<SelectorCompositionStreamState<FlatState>>,
    resources: &mut Resources,
    score_director: &D,
    context: MoveStreamContext,
) -> SelectorCompositionBorrowedCursor<'state, 'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
    D: Director<S>,
{
    let stream_state = state_slot
        .take()
        .expect("a composition state allows only one live cursor");
    let cursor = open_cursor_with_owned_stream_state(
        selector,
        stream_state,
        resources,
        score_director,
        context,
    );
    SelectorCompositionBorrowedCursor {
        state_slot,
        cursor: Some(cursor),
    }
}

impl<'a, S, M, Flat, FlatState, Resources> Debug
    for SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources> + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorCompositionBorrowedSource")
            .field("selector", self.selector)
            .field("stream_state_available", &self.stream_state.is_some())
            .finish()
    }
}

impl<'a, S, M, Flat, FlatState, Resources> MoveCursorSource<S, M>
    for SelectorCompositionBorrowedSource<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
    Resources: CursorResources,
{
    /// The runner owns this resource for the whole solve and lends it to the
    /// shared phase loop at each candidate pull. The source and its cursor
    /// retain only selector state.
    type Resources = Resources;

    type Cursor<'state>
        = SelectorCompositionBorrowedCursor<'state, 'a, S, M, Flat, FlatState, Resources>
    where
        Self: 'state;

    fn open_cursor<'state, D: Director<S>>(
        &'state mut self,
        resources: &mut Self::Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'state> {
        self.open_cursor_with_resources(resources, score_director, context)
    }
}

/// A live cursor that returns its exact arena state. It owns no mutable
/// resource; the shared phase loop lends a resource only to a reachable
/// `next_candidate_with_resources` pull.
pub(crate) struct SelectorCompositionBorrowedCursor<'state, 'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
{
    state_slot: &'state mut Option<SelectorCompositionStreamState<FlatState>>,
    cursor: Option<SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>>,
}

impl<'a, S, M, Flat, FlatState, Resources> ResourceMoveCursor<S, M, Resources>
    for SelectorCompositionBorrowedCursor<'_, 'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
{
    fn next_candidate_with_resources<D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId> {
        self.inner_mut()
            .next_candidate_with_resources(director, resources)
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.inner().candidate(index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        self.inner_mut().take_candidate(index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        self.inner_mut()
            .apply_owned_candidate(index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        self.inner_mut().release_candidate(index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.inner().selector_index(index)
    }
}

impl<'a, S, M, Flat, FlatState, Resources>
    SelectorCompositionBorrowedCursor<'_, 'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
{
    fn inner(&self) -> &SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources> {
        self.cursor
            .as_ref()
            .expect("live composition cursor must retain its inner cursor")
    }

    fn inner_mut(
        &mut self,
    ) -> &mut SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources> {
        self.cursor
            .as_mut()
            .expect("live composition cursor must retain its inner cursor")
    }
}

impl<'a, S, M, Flat, FlatState, Resources> Drop
    for SelectorCompositionBorrowedCursor<'_, 'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S>,
    Flat: StatefulComposedFlat<'a, S, M, FlatState, Resources>,
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
