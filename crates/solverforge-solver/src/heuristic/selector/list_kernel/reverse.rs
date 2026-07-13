//! Streamed intra-list reversal cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::ReverseEmitter;

pub(crate) const STATIC_REVERSE_ENTITY_SALT: u64 = 0x1157_2A07_0000_0001;
const STATIC_REVERSE_START_SALT: u64 = 0x1157_2A07_0000_0002;
const STATIC_REVERSE_END_SALT: u64 = 0x1157_2A07_0000_0003;

/// Canonical intra-list 2-opt/reverse cursor.
pub(crate) struct ReverseCursor<S, E>
where
    S: PlanningSolution,
    E: ReverseEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    entity_idx: usize,
    start_offset: usize,
    end_offset: usize,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    descriptor_index: usize,
}

impl<S, E> ReverseCursor<S, E>
where
    S: PlanningSolution,
    E: ReverseEmitter<S>,
{
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            entities,
            route_lens,
            context,
            entity_idx: 0,
            start_offset: 0,
            end_offset: 0,
            precedence_route_graph: None,
            descriptor_index,
        }
    }

    pub(crate) fn with_precedence_route_graph(
        mut self,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
    ) -> Self {
        self.precedence_route_graph = precedence_route_graph;
        self
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            let entity = *self.entities.get(self.entity_idx)?;
            let len = self.route_lens[self.entity_idx];
            if len < 2 {
                self.entity_idx += 1;
                self.start_offset = 0;
                self.end_offset = 0;
                continue;
            }
            while self.start_offset < len {
                let start = ordered_index(
                    self.start_offset,
                    len,
                    self.context,
                    STATIC_REVERSE_START_SALT ^ entity as u64 ^ self.descriptor_index as u64,
                );
                let end_count = len.saturating_sub(start + 1);
                if self.end_offset < end_count {
                    let end = start
                        + 2
                        + ordered_index(
                            self.end_offset,
                            end_count,
                            self.context,
                            STATIC_REVERSE_END_SALT ^ entity as u64 ^ start as u64,
                        );
                    self.end_offset += 1;
                    if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                        graph.intra_list_reverse_introduces_cycle(entity, start, end)
                    }) {
                        continue;
                    }
                    return Some(self.emitter.emit_reverse(entity, start, end));
                }
                self.start_offset += 1;
                self.end_offset = 0;
            }
            self.entity_idx += 1;
            self.start_offset = 0;
            self.end_offset = 0;
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for ReverseCursor<S, E>
where
    S: PlanningSolution,
    E: ReverseEmitter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.next_move().map(|mov| self.store.push(mov))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, E::Move>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> E::Move {
        self.store.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<E::Move> {
        self.next_move()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, E::Move>) -> bool,
    ) -> Option<E::Move> {
        loop {
            let mov = self.next_move()?;
            if predicate(MoveCandidateRef::Borrowed(&mov)) {
                return Some(mov);
            }
        }
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S, E> Iterator for ReverseCursor<S, E>
where
    S: PlanningSolution,
    E: ReverseEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
