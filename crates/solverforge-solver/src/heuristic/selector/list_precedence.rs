//! Public static facade for the canonical critical-path precedence kernel.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListMoveUnion;
use crate::heuristic::selector::list_kernel::{
    critical_analysis, filtered_move_count, filtered_multi_support_swap_count,
    multi_critical_ruin_count, CriticalAnalysis, NativePrecedenceEmitter, PrecedenceCursor,
};

use super::entity::EntitySelector;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::precedence_route::PrecedenceRouteHooks;

/// Generates move families around critical same-list precedence arcs.
///
/// The public constructor and static move union are unchanged. Critical-path
/// analysis, ordering, cycle pruning, candidate ownership, and mixed move
/// emission are shared with the future runtime-list leaf.
pub struct ListPrecedenceMoveSelector<S, V, ES> {
    entity_selector: ES,
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    node_duration: fn(&S, V) -> usize,
    fixed_successors: fn(&S, V, &mut Vec<V>),
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    ruin_remove: fn(&mut S, usize, usize) -> V,
    ruin_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

/// Public cursor facade over the canonical precedence stream.
pub struct ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: PrecedenceCursor<S, NativePrecedenceEmitter<S, V>>,
}

impl<S, V> ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: PrecedenceCursor<S, NativePrecedenceEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListMoveUnion<S, V>> for ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListMoveUnion<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListMoveUnion<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListMoveUnion<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListMoveUnion<S, V>>) -> bool,
    ) -> Option<ListMoveUnion<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListPrecedenceMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListMoveUnion<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListPrecedenceMoveSelector<S, V, ES> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ListPrecedenceMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, ES> ListPrecedenceMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        element_count: fn(&S) -> usize,
        index_to_element: fn(&S, usize) -> V,
        node_duration: fn(&S, V) -> usize,
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            element_count,
            index_to_element,
            node_duration,
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            list_set,
            list_reverse,
            ruin_remove,
            ruin_insert,
            element_owner_fn,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveUnion<S, V>> for ListPrecedenceMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListPrecedenceMoveCursor<S, V>
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
        let analysis = self.analysis(score_director);
        let emitter = self.emitter();
        ListPrecedenceMoveCursor::new(PrecedenceCursor::new(
            analysis.blocks,
            analysis.route_graph,
            context,
            emitter,
            self.descriptor_index,
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let analysis = self.analysis(score_director);
        analysis
            .blocks
            .iter()
            .copied()
            .map(|block| filtered_move_count(block, &analysis.route_graph))
            .sum::<usize>()
            + filtered_multi_support_swap_count(&analysis.blocks, &analysis.route_graph)
            + multi_critical_ruin_count(&analysis.blocks)
    }
}

impl<S, V, ES> ListPrecedenceMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn analysis<D: Director<S>>(&self, score_director: &D) -> CriticalAnalysis {
        critical_analysis(
            score_director,
            &self.entity_selector,
            self.element_count,
            self.index_to_element,
            self.node_duration,
            self.list_len,
            PrecedenceRouteHooks::new(
                self.fixed_successors,
                self.entity_count,
                self.list_len,
                self.list_get,
            ),
        )
    }

    fn emitter(&self) -> NativePrecedenceEmitter<S, V> {
        NativePrecedenceEmitter::new(
            self.element_count,
            self.index_to_element,
            self.fixed_successors,
            self.entity_count,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.list_set,
            self.list_reverse,
            self.ruin_remove,
            self.ruin_insert,
            self.element_owner_fn,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }
}
