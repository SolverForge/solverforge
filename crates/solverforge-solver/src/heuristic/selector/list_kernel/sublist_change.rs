//! Canonical streamed contiguous-sublist relocation cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::{SelectedListOwners, SublistChangeEmitter};

#[derive(Clone, Copy, Debug)]
pub(crate) struct SublistChangeOrderSalts {
    pub(crate) entity: u64,
    pub(crate) segment_start: u64,
    pub(crate) segment_size: u64,
    pub(crate) intra_destination: u64,
    pub(crate) inter_destination: u64,
}

pub(crate) const STATIC_SUBLIST_CHANGE_SALTS: SublistChangeOrderSalts = SublistChangeOrderSalts {
    entity: 0x5B15_7C4A_46E0_0001,
    segment_start: 0x5B15_7C4A_46E0_0002,
    segment_size: 0x5B15_7C4A_46E0_0003,
    intra_destination: 0x5B15_7C4A_46E0_0004,
    inter_destination: 0x5B15_7C4A_46E0_0005,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SublistChangeStage {
    Intra,
    Inter,
}

/// Streams Or-opt / contiguous-sublist relocation coordinates exactly once.
pub(crate) struct SublistChangeCursor<S, E>
where
    S: PlanningSolution,
    E: SublistChangeEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    salts: SublistChangeOrderSalts,
    source_idx: usize,
    segment_start_offset: usize,
    segment_size_offset: usize,
    stage: SublistChangeStage,
    intra_destination_offset: usize,
    destination_idx: usize,
    inter_destination_offset: usize,
    min_segment_size: usize,
    max_segment_size: usize,
    owners: SelectedListOwners,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    descriptor_index: usize,
}

impl<S, E> SublistChangeCursor<S, E>
where
    S: PlanningSolution,
    E: SublistChangeEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        salts: SublistChangeOrderSalts,
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
            salts,
            source_idx: 0,
            segment_start_offset: 0,
            segment_size_offset: 0,
            stage: SublistChangeStage::Intra,
            intra_destination_offset: 0,
            destination_idx: 0,
            inter_destination_offset: 0,
            min_segment_size,
            max_segment_size,
            owners,
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

    fn segment_size_count(&self, source_len: usize, segment_start: usize) -> usize {
        let max_valid = self
            .max_segment_size
            .min(source_len.saturating_sub(segment_start));
        max_valid.saturating_sub(self.min_segment_size)
            + usize::from(max_valid >= self.min_segment_size)
    }

    fn current_segment(&self) -> Option<(usize, usize, usize, usize, usize)> {
        let source_entity = *self.entities.get(self.source_idx)?;
        let source_len = self.route_lens[self.source_idx];
        if source_len < self.min_segment_size {
            return Some((source_entity, source_len, 0, 0, 0));
        }
        let segment_start = ordered_index(
            self.segment_start_offset,
            source_len,
            self.context,
            self.salts.segment_start ^ source_entity as u64 ^ self.descriptor_index as u64,
        );
        let size_count = self.segment_size_count(source_len, segment_start);
        if size_count == 0 {
            return Some((source_entity, source_len, segment_start, 0, 0));
        }
        let size_offset = ordered_index(
            self.segment_size_offset,
            size_count,
            self.context,
            self.salts.segment_size ^ source_entity as u64 ^ segment_start as u64,
        );
        let segment_size = self.min_segment_size + size_offset;
        Some((
            source_entity,
            source_len,
            segment_start,
            segment_start + segment_size,
            segment_size,
        ))
    }

    fn advance_segment(&mut self) {
        let Some((_, source_len, segment_start, _, _)) = self.current_segment() else {
            return;
        };
        let size_count = self.segment_size_count(source_len, segment_start);
        self.segment_size_offset += 1;
        if self.segment_size_offset >= size_count {
            self.segment_size_offset = 0;
            self.segment_start_offset += 1;
        }
        while self.source_idx < self.route_lens.len()
            && self.segment_start_offset >= self.route_lens[self.source_idx]
        {
            self.source_idx += 1;
            self.segment_start_offset = 0;
            self.segment_size_offset = 0;
        }
        self.stage = SublistChangeStage::Intra;
        self.intra_destination_offset = 0;
        self.destination_idx = 0;
        self.inter_destination_offset = 0;
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            let (source_entity, source_len, segment_start, segment_end, segment_size) =
                self.current_segment()?;
            if source_len < self.min_segment_size || segment_size == 0 {
                self.advance_segment();
                continue;
            }
            match self.stage {
                SublistChangeStage::Intra => {
                    let post_removal_len = source_len - segment_size;
                    while self.intra_destination_offset <= post_removal_len {
                        let destination_position = ordered_index(
                            self.intra_destination_offset,
                            post_removal_len + 1,
                            self.context,
                            self.salts.intra_destination
                                ^ source_entity as u64
                                ^ segment_start as u64,
                        );
                        self.intra_destination_offset += 1;
                        if destination_position == segment_start {
                            continue;
                        }
                        if self.owners.is_present()
                            && !self.owners.segment_allows(
                                self.source_idx,
                                segment_start,
                                segment_end,
                                source_entity,
                            )
                        {
                            continue;
                        }
                        if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                            graph.intra_sublist_change_introduces_cycle(
                                source_entity,
                                segment_start,
                                segment_end,
                                destination_position,
                            )
                        }) {
                            continue;
                        }
                        return Some(self.emitter.emit_sublist_change(
                            source_entity,
                            segment_start,
                            segment_end,
                            source_entity,
                            destination_position,
                        ));
                    }
                    if self.owners.is_fixed_to_current() {
                        self.advance_segment();
                        continue;
                    }
                    self.stage = SublistChangeStage::Inter;
                    self.destination_idx = 0;
                    self.inter_destination_offset = 0;
                }
                SublistChangeStage::Inter => {
                    while self.destination_idx < self.entities.len() {
                        if self.destination_idx == self.source_idx {
                            self.destination_idx += 1;
                            continue;
                        }
                        let destination_entity = self.entities[self.destination_idx];
                        let destination_len = self.route_lens[self.destination_idx];
                        if self.inter_destination_offset <= destination_len {
                            let destination_position = ordered_index(
                                self.inter_destination_offset,
                                destination_len + 1,
                                self.context,
                                self.salts.inter_destination
                                    ^ source_entity as u64
                                    ^ destination_entity as u64
                                    ^ segment_start as u64,
                            );
                            self.inter_destination_offset += 1;
                            if self.owners.is_present()
                                && !self.owners.segment_allows(
                                    self.source_idx,
                                    segment_start,
                                    segment_end,
                                    destination_entity,
                                )
                            {
                                continue;
                            }
                            return Some(self.emitter.emit_sublist_change(
                                source_entity,
                                segment_start,
                                segment_end,
                                destination_entity,
                                destination_position,
                            ));
                        }
                        self.destination_idx += 1;
                        self.inter_destination_offset = 0;
                    }
                    self.advance_segment();
                }
            }
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for SublistChangeCursor<S, E>
where
    S: PlanningSolution,
    E: SublistChangeEmitter<S>,
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

impl<S, E> Iterator for SublistChangeCursor<S, E>
where
    S: PlanningSolution,
    E: SublistChangeEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
