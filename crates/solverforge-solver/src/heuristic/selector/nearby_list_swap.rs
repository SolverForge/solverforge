//! Public static adapter for the canonical nearby list-swap kernel.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListSwapMove;
use crate::heuristic::selector::list_kernel::{
    NativeNearbyProbe, NativeSwapEmitter, NearbySwapCursor, STATIC_NEARBY_SWAP_ENTITY_SALT,
    STATIC_NEARBY_SWAP_SOURCE_SALT,
};

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::nearby_list_change::CrossEntityDistanceMeter;

/// Public nearby list-swap adapter.
pub struct NearbyListSwapMoveSelector<S, V, D, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

/// Public cursor facade that hides the crate-private generic emitter/probe.
pub struct NearbyListSwapMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    inner: NearbySwapCursor<S, NativeSwapEmitter<S, V>, NativeNearbyProbe<'a, S, V, D>>,
}

impl<'a, S, V, D> NearbyListSwapMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    fn new(
        inner: NearbySwapCursor<S, NativeSwapEmitter<S, V>, NativeNearbyProbe<'a, S, V, D>>,
    ) -> Self {
        Self { inner }
    }
}

impl<S, V, D> MoveCursor<S, ListSwapMove<S, V>> for NearbyListSwapMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListSwapMove<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListSwapMove<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListSwapMove<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'b> fn(MoveCandidateRef<'b, S, ListSwapMove<S, V>>) -> bool,
    ) -> Option<ListSwapMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V, D> Iterator for NearbyListSwapMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    type Item = ListSwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, D, ES: Debug> Debug for NearbyListSwapMoveSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("distance_meter", &"<distance_meter>")
            .field("max_nearby", &self.max_nearby)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, D, ES> NearbyListSwapMoveSelector<S, V, D, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            list_len,
            list_get,
            list_set,
            element_owner_fn: None,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, V, D, ES> MoveSelector<S, ListSwapMove<S, V>> for NearbyListSwapMoveSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = NearbyListSwapMoveCursor<'a, S, V, D>
    where
        Self: 'a;

    fn open_cursor<'a, SD: Director<S>>(&'a self, score_director: &SD) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, SD: Director<S>>(
        &'a self,
        score_director: &SD,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let entity_count = score_director
            .entity_count(self.descriptor_index)
            .unwrap_or(0);
        let mut selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.apply_stream_order(
            context,
            STATIC_NEARBY_SWAP_ENTITY_SALT ^ self.descriptor_index as u64,
        );
        let fixed_to_current_entity = self.element_owner_fn.is_some_and(|_| {
            crate::list_placement::selected_elements_fixed_to_current_entities(
                self.element_owner_fn,
                solution,
                entity_count,
                &selected.entities,
                &selected.route_lens,
                self.list_get,
            )
        });
        NearbyListSwapMoveCursor::new(NearbySwapCursor::new(
            NativeSwapEmitter::new(
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            ),
            solution.clone(),
            NativeNearbyProbe::new(&self.distance_meter, self.list_get, self.element_owner_fn),
            selected.entities,
            selected.route_lens,
            entity_count,
            context,
            fixed_to_current_entity,
            self.max_nearby,
            self.descriptor_index,
            STATIC_NEARBY_SWAP_SOURCE_SALT,
        ))
    }

    fn size<SD: Director<S>>(&self, score_director: &SD) -> usize {
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.total_elements() * self.max_nearby / 2
    }
}
