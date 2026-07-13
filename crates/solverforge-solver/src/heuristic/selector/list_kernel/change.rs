//! Streamed full relocation cursor.
//!
//! Coordinate order, selected-move storage, ownership pruning, and precedence
//! filtering live here so all public relocation facades exercise one kernel.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::{ChangeEmitter, SelectedListOwners};

/// Stable salts for native static list-change enumeration.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChangeOrderSalts {
    pub(crate) entity: u64,
    pub(crate) source: u64,
    pub(crate) intra_destination: u64,
    pub(crate) inter_destination: u64,
}

pub(crate) const STATIC_CHANGE_SALTS: ChangeOrderSalts = ChangeOrderSalts {
    entity: 0x1157_C4A4_6E00_0001,
    source: 0x1157_C4A4_6E00_0002,
    intra_destination: 0x1157_C4A4_6E00_0003,
    inter_destination: 0x1157_C4A4_6E00_0004,
};

/// Stable salts for dynamic list-change enumeration.
pub(crate) const DYNAMIC_CHANGE_SALTS: ChangeOrderSalts = ChangeOrderSalts {
    entity: 0xD158_C4A4_6E00_0001,
    source: 0xD158_C4A4_6E00_0002,
    intra_destination: 0xD158_C4A4_6E00_0003,
    inter_destination: 0xD158_C4A4_6E00_0004,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChangeStage {
    Intra,
    Inter,
}

/// Canonical full relocation cursor.
pub(crate) struct ChangeCursor<S, E>
where
    S: PlanningSolution,
    E: ChangeEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    salts: ChangeOrderSalts,
    src_idx: usize,
    src_pos_offset: usize,
    stage: ChangeStage,
    intra_dst_offset: usize,
    dst_idx: usize,
    inter_dst_pos_offset: usize,
    owners: SelectedListOwners,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    descriptor_index: usize,
}

impl<S, E> ChangeCursor<S, E>
where
    S: PlanningSolution,
    E: ChangeEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        salts: ChangeOrderSalts,
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
            src_idx: 0,
            src_pos_offset: 0,
            stage: ChangeStage::Intra,
            intra_dst_offset: 0,
            dst_idx: 0,
            inter_dst_pos_offset: 0,
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

    fn current_source(&self) -> Option<(usize, usize, usize)> {
        let source_entity = *self.entities.get(self.src_idx)?;
        let source_len = self.route_lens[self.src_idx];
        if source_len == 0 {
            return Some((source_entity, source_len, 0));
        }
        let source_position = ordered_index(
            self.src_pos_offset,
            source_len,
            self.context,
            self.salts.source ^ source_entity as u64 ^ self.descriptor_index as u64,
        );
        Some((source_entity, source_len, source_position))
    }

    fn advance_source_position(&mut self) {
        self.src_pos_offset += 1;
        self.stage = ChangeStage::Intra;
        self.intra_dst_offset = 0;
        self.dst_idx = 0;
        self.inter_dst_pos_offset = 0;

        while self.src_idx < self.route_lens.len()
            && self.src_pos_offset >= self.route_lens[self.src_idx]
        {
            self.src_idx += 1;
            self.src_pos_offset = 0;
        }
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            let (source_entity, source_len, source_position) = self.current_source()?;
            if source_len == 0 {
                self.src_idx += 1;
                continue;
            }

            match self.stage {
                ChangeStage::Intra => {
                    while self.intra_dst_offset <= source_len {
                        let destination_position = ordered_index(
                            self.intra_dst_offset,
                            source_len + 1,
                            self.context,
                            self.salts.intra_destination
                                ^ source_entity as u64
                                ^ source_position as u64,
                        );
                        self.intra_dst_offset += 1;
                        if source_position == destination_position
                            || destination_position == source_position + 1
                        {
                            continue;
                        }
                        if self.owners.is_present()
                            && !self
                                .owners
                                .allows(self.src_idx, source_position, source_entity)
                        {
                            continue;
                        }
                        if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                            graph.intra_list_change_introduces_cycle(
                                source_entity,
                                source_position,
                                destination_position,
                            )
                        }) {
                            continue;
                        }
                        return Some(self.emitter.emit_change(
                            source_entity,
                            source_position,
                            source_entity,
                            destination_position,
                        ));
                    }
                    if self.owners.is_fixed_to_current() {
                        self.advance_source_position();
                        continue;
                    }
                    self.stage = ChangeStage::Inter;
                    self.dst_idx = 0;
                    self.inter_dst_pos_offset = 0;
                }
                ChangeStage::Inter => {
                    while self.dst_idx < self.entities.len() {
                        if self.dst_idx == self.src_idx {
                            self.dst_idx += 1;
                            self.inter_dst_pos_offset = 0;
                            continue;
                        }
                        let destination_entity = self.entities[self.dst_idx];
                        let destination_len = self.route_lens[self.dst_idx];
                        if self.inter_dst_pos_offset <= destination_len {
                            let destination_position = ordered_index(
                                self.inter_dst_pos_offset,
                                destination_len + 1,
                                self.context,
                                self.salts.inter_destination
                                    ^ source_entity as u64
                                    ^ destination_entity as u64
                                    ^ source_position as u64,
                            );
                            self.inter_dst_pos_offset += 1;
                            if self.owners.is_present()
                                && !self.owners.allows(
                                    self.src_idx,
                                    source_position,
                                    destination_entity,
                                )
                            {
                                continue;
                            }
                            return Some(self.emitter.emit_change(
                                source_entity,
                                source_position,
                                destination_entity,
                                destination_position,
                            ));
                        }
                        self.dst_idx += 1;
                        self.inter_dst_pos_offset = 0;
                    }
                    self.advance_source_position();
                }
            }
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for ChangeCursor<S, E>
where
    S: PlanningSolution,
    E: ChangeEmitter<S>,
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

    #[inline(always)]
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

impl<S, E> Iterator for ChangeCursor<S, E>
where
    S: PlanningSolution,
    E: ChangeEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
