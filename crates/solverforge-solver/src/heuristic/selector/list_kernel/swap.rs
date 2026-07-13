//! Streamed full exchange cursor.
//!
//! Coordinate order, selected-move storage, ownership pruning, and precedence
//! filtering live here so all public exchange facades exercise one kernel.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::{SelectedListOwners, SwapEmitter};

#[derive(Clone, Copy, Debug)]
pub(crate) struct SwapOrderSalts {
    pub(crate) entity: u64,
    pub(crate) first_position: u64,
    pub(crate) second_position: u64,
    pub(crate) inter_first_position: u64,
    pub(crate) inter_second_position: u64,
}

pub(crate) const STATIC_SWAP_SALTS: SwapOrderSalts = SwapOrderSalts {
    entity: 0x1157_5A09_0000_0001,
    first_position: 0x1157_5A09_0000_0002,
    second_position: 0x1157_5A09_0000_0003,
    inter_first_position: 0x1157_5A09_0000_0004,
    inter_second_position: 0x1157_5A09_0000_0005,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwapStage {
    Intra,
    Inter,
}

/// Canonical full list-swap cursor.
pub(crate) struct SwapCursor<S, E>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    salts: SwapOrderSalts,
    entity_idx: usize,
    stage: SwapStage,
    first_position_offset: usize,
    second_position_offset: usize,
    destination_idx: usize,
    inter_first_position_offset: usize,
    inter_second_position_offset: usize,
    owners: SelectedListOwners,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    descriptor_index: usize,
}

impl<S, E> SwapCursor<S, E>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        salts: SwapOrderSalts,
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
            entity_idx: 0,
            stage: SwapStage::Intra,
            first_position_offset: 0,
            second_position_offset: 0,
            destination_idx: 1,
            inter_first_position_offset: 0,
            inter_second_position_offset: 0,
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

    fn advance_entity(&mut self) {
        self.entity_idx += 1;
        self.stage = SwapStage::Intra;
        self.first_position_offset = 0;
        self.second_position_offset = 0;
        self.destination_idx = self.entity_idx + 1;
        self.inter_first_position_offset = 0;
        self.inter_second_position_offset = 0;
    }

    fn owner_allows_swap(
        &self,
        first_entity_index: usize,
        first_position: usize,
        first_entity: usize,
        second_entity_index: usize,
        second_position: usize,
        second_entity: usize,
    ) -> bool {
        if !self.owners.has_matrix() {
            return true;
        }
        self.owners
            .restriction_at(first_entity_index, first_position)
            .is_some_and(|restriction| restriction.allows(second_entity))
            && self
                .owners
                .restriction_at(second_entity_index, second_position)
                .is_some_and(|restriction| restriction.allows(first_entity))
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            if self.entity_idx >= self.entities.len() {
                return None;
            }
            let first_entity = self.entities[self.entity_idx];
            let first_len = self.route_lens[self.entity_idx];
            if first_len == 0 {
                self.advance_entity();
                continue;
            }

            match self.stage {
                SwapStage::Intra => {
                    while self.first_position_offset < first_len {
                        let first_position = ordered_index(
                            self.first_position_offset,
                            first_len,
                            self.context,
                            self.salts.first_position
                                ^ first_entity as u64
                                ^ self.descriptor_index as u64,
                        );
                        let second_count = first_len.saturating_sub(first_position + 1);
                        if self.second_position_offset < second_count {
                            let second_position = first_position
                                + 1
                                + ordered_index(
                                    self.second_position_offset,
                                    second_count,
                                    self.context,
                                    self.salts.second_position
                                        ^ first_entity as u64
                                        ^ first_position as u64,
                                );
                            self.second_position_offset += 1;
                            if self.owners.has_matrix()
                                && (!self
                                    .owners
                                    .restriction_at(self.entity_idx, first_position)
                                    .is_some_and(|restriction| restriction.allows(first_entity))
                                    || !self
                                        .owners
                                        .restriction_at(self.entity_idx, second_position)
                                        .is_some_and(|restriction| {
                                            restriction.allows(first_entity)
                                        }))
                            {
                                continue;
                            }
                            if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                                graph.intra_list_swap_introduces_cycle(
                                    first_entity,
                                    first_position,
                                    second_position,
                                )
                            }) {
                                continue;
                            }
                            return Some(self.emitter.emit_swap(
                                first_entity,
                                first_position,
                                first_entity,
                                second_position,
                            ));
                        }
                        self.first_position_offset += 1;
                        self.second_position_offset = 0;
                    }
                    if self.owners.is_fixed_to_current() {
                        self.advance_entity();
                        continue;
                    }
                    self.stage = SwapStage::Inter;
                    self.destination_idx = self.entity_idx + 1;
                    self.inter_first_position_offset = 0;
                    self.inter_second_position_offset = 0;
                }
                SwapStage::Inter => {
                    while self.destination_idx < self.entities.len() {
                        let second_entity = self.entities[self.destination_idx];
                        let second_len = self.route_lens[self.destination_idx];
                        if second_len == 0 {
                            self.destination_idx += 1;
                            continue;
                        }
                        while self.inter_first_position_offset < first_len {
                            let first_position = ordered_index(
                                self.inter_first_position_offset,
                                first_len,
                                self.context,
                                self.salts.inter_first_position
                                    ^ first_entity as u64
                                    ^ second_entity as u64,
                            );
                            if self.inter_second_position_offset < second_len {
                                let second_position = ordered_index(
                                    self.inter_second_position_offset,
                                    second_len,
                                    self.context,
                                    self.salts.inter_second_position
                                        ^ first_entity as u64
                                        ^ second_entity as u64
                                        ^ first_position as u64,
                                );
                                self.inter_second_position_offset += 1;
                                if !self.owner_allows_swap(
                                    self.entity_idx,
                                    first_position,
                                    first_entity,
                                    self.destination_idx,
                                    second_position,
                                    second_entity,
                                ) {
                                    continue;
                                }
                                return Some(self.emitter.emit_swap(
                                    first_entity,
                                    first_position,
                                    second_entity,
                                    second_position,
                                ));
                            }
                            self.inter_first_position_offset += 1;
                            self.inter_second_position_offset = 0;
                        }
                        self.destination_idx += 1;
                        self.inter_first_position_offset = 0;
                        self.inter_second_position_offset = 0;
                    }
                    self.advance_entity();
                }
            }
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for SwapCursor<S, E>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
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

    #[inline(always)]
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

impl<S, E> Iterator for SwapCursor<S, E>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
