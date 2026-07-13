//! Public static adapter for the canonical nearby list-change kernel.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListChangeMove;
use crate::heuristic::selector::list_kernel::{
    NativeChangeEmitter, NativeNearbyProbe, NearbyChangeCursor, STATIC_NEARBY_CHANGE_ENTITY_SALT,
    STATIC_NEARBY_CHANGE_SOURCE_SALT,
};

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Measures distance between two list positions, potentially across entities.
pub trait CrossEntityDistanceMeter<S>: Send + Sync {
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64;
}

/// Historic default distance behavior for intra-route-only nearby selection.
#[derive(Debug, Clone, Copy)]
pub struct DefaultCrossEntityDistanceMeter;

impl Default for DefaultCrossEntityDistanceMeter {
    fn default() -> Self {
        Self
    }
}

impl<S> CrossEntityDistanceMeter<S> for DefaultCrossEntityDistanceMeter {
    fn distance(
        &self,
        _solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        if source_entity == destination_entity {
            (source_position as f64 - destination_position as f64).abs()
        } else {
            f64::INFINITY
        }
    }
}

/// Public nearby list-change adapter.
pub struct NearbyListChangeMoveSelector<S, V, D, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

/// Public cursor facade that hides the crate-private generic emitter/probe.
pub struct NearbyListChangeMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    inner: NearbyChangeCursor<S, NativeChangeEmitter<S, V>, NativeNearbyProbe<'a, S, V, D>>,
}

impl<'a, S, V, D> NearbyListChangeMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    fn new(
        inner: NearbyChangeCursor<S, NativeChangeEmitter<S, V>, NativeNearbyProbe<'a, S, V, D>>,
    ) -> Self {
        Self { inner }
    }
}

impl<S, V, D> MoveCursor<S, ListChangeMove<S, V>> for NearbyListChangeMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListChangeMove<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListChangeMove<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListChangeMove<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'b> fn(MoveCandidateRef<'b, S, ListChangeMove<S, V>>) -> bool,
    ) -> Option<ListChangeMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V, D> Iterator for NearbyListChangeMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    type Item = ListChangeMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, D, ES: Debug> Debug for NearbyListChangeMoveSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("distance_meter", &"<distance_meter>")
            .field("max_nearby", &self.max_nearby)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, D, ES> NearbyListChangeMoveSelector<S, V, D, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            list_len,
            list_get,
            list_remove,
            list_insert,
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

impl<S, V, D, ES> MoveSelector<S, ListChangeMove<S, V>>
    for NearbyListChangeMoveSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = NearbyListChangeMoveCursor<'a, S, V, D>
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
            STATIC_NEARBY_CHANGE_ENTITY_SALT ^ self.descriptor_index as u64,
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
        NearbyListChangeMoveCursor::new(NearbyChangeCursor::new(
            NativeChangeEmitter::new(
                self.list_len,
                self.list_get,
                self.list_remove,
                self.list_insert,
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
            STATIC_NEARBY_CHANGE_SOURCE_SALT,
        ))
    }

    fn size<SD: Director<S>>(&self, score_director: &SD) -> usize {
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.total_elements() * self.max_nearby
    }
}
