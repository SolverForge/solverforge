//! Streamed distance-pruned exchange cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_support::{
    sort_and_limit_nearby_candidates, NearbyCandidate,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;
use crate::list_placement::OwnerRestriction;

use super::{NearbySwapProbe, SwapEmitter};

pub(crate) const STATIC_NEARBY_SWAP_ENTITY_SALT: u64 = 0xA1EA_25A0_9000_0001;
pub(crate) const STATIC_NEARBY_SWAP_SOURCE_SALT: u64 = 0xA1EA_25A0_9000_0002;

/// Canonical distance-pruned list-swap cursor.
pub(crate) struct NearbySwapCursor<S, E, P>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
    P: NearbySwapProbe<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    solution: S,
    probe: P,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    entity_count: usize,
    context: MoveStreamContext,
    source_idx: usize,
    source_pos_offset: usize,
    current_source: (usize, usize),
    destinations: Vec<(usize, usize)>,
    destination_offset: usize,
    fixed_to_current_entity: bool,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    max_nearby: usize,
    descriptor_index: usize,
    source_salt: u64,
}

impl<S, E, P> NearbySwapCursor<S, E, P>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
    P: NearbySwapProbe<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        solution: S,
        probe: P,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        entity_count: usize,
        context: MoveStreamContext,
        fixed_to_current_entity: bool,
        max_nearby: usize,
        descriptor_index: usize,
        source_salt: u64,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            solution,
            probe,
            entities,
            route_lens,
            entity_count,
            context,
            source_idx: 0,
            source_pos_offset: 0,
            current_source: (0, 0),
            destinations: Vec::new(),
            destination_offset: 0,
            fixed_to_current_entity,
            precedence_route_graph: None,
            max_nearby,
            descriptor_index,
            source_salt,
        }
    }

    pub(crate) fn with_precedence_route_graph(
        mut self,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
    ) -> Self {
        self.precedence_route_graph = precedence_route_graph;
        self
    }

    fn restriction_at(&self, entity: usize, position: usize) -> Option<OwnerRestriction> {
        self.probe
            .owner_restriction(&self.solution, self.entity_count, entity, position)
    }

    fn load_next_source(&mut self) -> bool {
        let mut candidates: Vec<NearbyCandidate> = Vec::new();
        while self.source_idx < self.entities.len() {
            let source_entity = self.entities[self.source_idx];
            let source_len = self.route_lens[self.source_idx];
            if source_len == 0 {
                self.source_idx += 1;
                self.source_pos_offset = 0;
                continue;
            }

            while self.source_pos_offset < source_len {
                let source_position = ordered_index(
                    self.source_pos_offset,
                    source_len,
                    self.context,
                    self.source_salt ^ source_entity as u64 ^ self.descriptor_index as u64,
                );
                self.source_pos_offset += 1;
                let source_restriction = self
                    .probe
                    .has_owner_binding()
                    .then(|| self.restriction_at(source_entity, source_position))
                    .flatten();
                if self.probe.has_owner_binding() && source_restriction.is_none() {
                    continue;
                }

                candidates.clear();
                for destination_position in source_position + 1..source_len {
                    let destination_restriction = self
                        .probe
                        .has_owner_binding()
                        .then(|| self.restriction_at(source_entity, destination_position))
                        .flatten();
                    if self.probe.has_owner_binding() && destination_restriction.is_none() {
                        continue;
                    }
                    if source_restriction
                        .is_some_and(|restriction| !restriction.allows(source_entity))
                        || destination_restriction
                            .is_some_and(|restriction| !restriction.allows(source_entity))
                    {
                        continue;
                    }
                    if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                        graph.intra_list_swap_introduces_cycle(
                            source_entity,
                            source_position,
                            destination_position,
                        )
                    }) {
                        continue;
                    }
                    let distance = self.probe.distance(
                        &self.solution,
                        source_entity,
                        source_position,
                        source_entity,
                        destination_position,
                    );
                    if distance.is_finite() {
                        candidates.push((source_entity, destination_position, distance));
                    }
                }

                if !self.fixed_to_current_entity {
                    for (destination_idx, &destination_entity) in self.entities.iter().enumerate() {
                        if destination_idx <= self.source_idx {
                            continue;
                        }
                        let destination_len = self.route_lens[destination_idx];
                        if destination_len == 0 {
                            continue;
                        }
                        for destination_position in 0..destination_len {
                            let destination_restriction = self
                                .probe
                                .has_owner_binding()
                                .then(|| {
                                    self.restriction_at(destination_entity, destination_position)
                                })
                                .flatten();
                            if self.probe.has_owner_binding() && destination_restriction.is_none() {
                                continue;
                            }
                            if source_restriction
                                .is_some_and(|restriction| !restriction.allows(destination_entity))
                                || destination_restriction
                                    .is_some_and(|restriction| !restriction.allows(source_entity))
                            {
                                continue;
                            }
                            let distance = self.probe.distance(
                                &self.solution,
                                source_entity,
                                source_position,
                                destination_entity,
                                destination_position,
                            );
                            if distance.is_finite() {
                                candidates.push((
                                    destination_entity,
                                    destination_position,
                                    distance,
                                ));
                            }
                        }
                    }
                }

                sort_and_limit_nearby_candidates(&mut candidates, self.max_nearby);
                if candidates.is_empty() {
                    continue;
                }
                self.current_source = (source_entity, source_position);
                self.destinations.clear();
                self.destinations.extend(
                    candidates
                        .iter()
                        .map(|&(entity, position, _)| (entity, position)),
                );
                self.destination_offset = 0;
                return true;
            }

            self.source_idx += 1;
            self.source_pos_offset = 0;
        }
        false
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        if self.destination_offset >= self.destinations.len() && !self.load_next_source() {
            return None;
        }
        let (source_entity, source_position) = self.current_source;
        let (destination_entity, destination_position) = self.destinations[self.destination_offset];
        self.destination_offset += 1;
        Some(self.emitter.emit_swap(
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        ))
    }
}

impl<S, E, P> MoveCursor<S, E::Move> for NearbySwapCursor<S, E, P>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
    P: NearbySwapProbe<S>,
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

impl<S, E, P> Iterator for NearbySwapCursor<S, E, P>
where
    S: PlanningSolution,
    E: SwapEmitter<S>,
    P: NearbySwapProbe<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
