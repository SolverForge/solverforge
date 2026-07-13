//! Canonical streamed contiguous-sublist exchange cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::{SelectedListOwners, SublistSwapEmitter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ListSegment {
    start: usize,
    end: usize,
}

#[derive(Clone)]
struct SublistSegmentCursor {
    entity: usize,
    route_len: usize,
    min_segment_size: usize,
    max_segment_size: usize,
    context: MoveStreamContext,
    descriptor_index: usize,
    start_offset: usize,
    size_offset: usize,
    current_start: Option<usize>,
    current_size_count: usize,
}

impl SublistSegmentCursor {
    fn new(
        entity: usize,
        route_len: usize,
        min_segment_size: usize,
        max_segment_size: usize,
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity,
            route_len,
            min_segment_size,
            max_segment_size,
            context,
            descriptor_index,
            start_offset: 0,
            size_offset: 0,
            current_start: None,
            current_size_count: 0,
        }
    }
}

impl Iterator for SublistSegmentCursor {
    type Item = ListSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.route_len < self.min_segment_size {
            return None;
        }
        loop {
            if let Some(start) = self.current_start {
                if self.size_offset < self.current_size_count {
                    let segment_size = self.min_segment_size
                        + ordered_index(
                            self.size_offset,
                            self.current_size_count,
                            self.context,
                            0x5B15_75A0_9000_0003 ^ self.entity as u64 ^ start as u64,
                        );
                    self.size_offset += 1;
                    return Some(ListSegment {
                        start,
                        end: start + segment_size,
                    });
                }
                self.current_start = None;
            }

            if self.start_offset >= self.route_len {
                return None;
            }
            let start = ordered_index(
                self.start_offset,
                self.route_len,
                self.context,
                0x5B15_75A0_9000_0002 ^ self.entity as u64 ^ self.descriptor_index as u64,
            );
            self.start_offset += 1;
            let max_valid = self.max_segment_size.min(self.route_len - start);
            if max_valid < self.min_segment_size {
                continue;
            }
            self.current_start = Some(start);
            self.current_size_count = max_valid - self.min_segment_size + 1;
            self.size_offset = 0;
        }
    }
}

pub(crate) const STATIC_SUBLIST_SWAP_ENTITY_SALT: u64 = 0x5B15_75A0_9000_0001;

/// Streams contiguous-sublist exchange coordinates exactly once.
pub(crate) struct SublistSwapCursor<S, E>
where
    S: PlanningSolution,
    E: SublistSwapEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    owners: SelectedListOwners,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    first_entity_idx: usize,
    first_segments: Option<SublistSegmentCursor>,
    first_segment: Option<ListSegment>,
    second_entity_idx: usize,
    second_segments: Option<SublistSegmentCursor>,
    min_segment_size: usize,
    max_segment_size: usize,
    descriptor_index: usize,
}

impl<S, E> SublistSwapCursor<S, E>
where
    S: PlanningSolution,
    E: SublistSwapEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        min_segment_size: usize,
        max_segment_size: usize,
        owners: SelectedListOwners,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            entities,
            route_lens,
            context,
            owners,
            precedence_route_graph: None,
            first_entity_idx: 0,
            first_segments: None,
            first_segment: None,
            second_entity_idx: 0,
            second_segments: None,
            min_segment_size,
            max_segment_size,
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

    fn segment_cursor(&self, entity_idx: usize) -> SublistSegmentCursor {
        SublistSegmentCursor::new(
            self.entities[entity_idx],
            self.route_lens[entity_idx],
            self.min_segment_size,
            self.max_segment_size,
            self.context,
            self.descriptor_index,
        )
    }

    fn segment_allows(
        &self,
        entity_idx: usize,
        segment: ListSegment,
        destination_entity: usize,
    ) -> bool {
        self.owners
            .segment_allows(entity_idx, segment.start, segment.end, destination_entity)
    }

    fn owner_allows_swap(
        &self,
        first_entity_idx: usize,
        first: ListSegment,
        first_entity: usize,
        second_entity_idx: usize,
        second: ListSegment,
        second_entity: usize,
    ) -> bool {
        if first_entity == second_entity {
            self.segment_allows(first_entity_idx, first, first_entity)
                && self.segment_allows(second_entity_idx, second, first_entity)
        } else {
            self.segment_allows(first_entity_idx, first, second_entity)
                && self.segment_allows(second_entity_idx, second, first_entity)
        }
    }

    fn next_first_segment(&mut self) -> Option<ListSegment> {
        loop {
            if self.first_entity_idx >= self.entities.len() {
                return None;
            }
            if self.first_segments.is_none() {
                self.first_segments = Some(self.segment_cursor(self.first_entity_idx));
            }
            if let Some(first) = self
                .first_segments
                .as_mut()
                .and_then(SublistSegmentCursor::next)
            {
                self.first_segment = Some(first);
                self.second_entity_idx = self.first_entity_idx;
                self.second_segments = None;
                return Some(first);
            }
            self.first_entity_idx += 1;
            self.first_segments = None;
            self.first_segment = None;
            self.second_entity_idx = self.first_entity_idx;
            self.second_segments = None;
        }
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            if self.first_entity_idx >= self.entities.len() {
                return None;
            }
            let first = match self.first_segment {
                Some(first) => first,
                None => self.next_first_segment()?,
            };
            let first_entity = self.entities[self.first_entity_idx];
            if self.second_entity_idx < self.first_entity_idx {
                self.second_entity_idx = self.first_entity_idx;
                self.second_segments = None;
            }

            while self.second_entity_idx < self.entities.len() {
                if self.owners.is_fixed_to_current()
                    && self.second_entity_idx != self.first_entity_idx
                {
                    break;
                }
                let second_entity = self.entities[self.second_entity_idx];
                if self.second_segments.is_none() {
                    self.second_segments = Some(self.segment_cursor(self.second_entity_idx));
                }
                while let Some(second) = self
                    .second_segments
                    .as_mut()
                    .and_then(SublistSegmentCursor::next)
                {
                    if self.first_entity_idx == self.second_entity_idx {
                        if second.start < first.end {
                            continue;
                        }
                        if first.start == second.start && first.end == second.end {
                            continue;
                        }
                    }
                    if self.owners.is_present()
                        && !self.owner_allows_swap(
                            self.first_entity_idx,
                            first,
                            first_entity,
                            self.second_entity_idx,
                            second,
                            second_entity,
                        )
                    {
                        continue;
                    }
                    if first_entity == second_entity
                        && self.precedence_route_graph.as_ref().is_some_and(|graph| {
                            graph.intra_sublist_swap_introduces_cycle(
                                first_entity,
                                first.start,
                                first.end,
                                second.start,
                                second.end,
                            )
                        })
                    {
                        continue;
                    }
                    return Some(self.emitter.emit_sublist_swap(
                        first_entity,
                        first.start,
                        first.end,
                        second_entity,
                        second.start,
                        second.end,
                    ));
                }
                self.second_entity_idx += 1;
                self.second_segments = None;
            }
            self.first_segment = None;
            self.second_entity_idx = self.first_entity_idx;
            self.second_segments = None;
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for SublistSwapCursor<S, E>
where
    S: PlanningSolution,
    E: SublistSwapEmitter<S>,
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

    #[inline(never)]
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

impl<S, E> Iterator for SublistSwapCursor<S, E>
where
    S: PlanningSolution,
    E: SublistSwapEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
