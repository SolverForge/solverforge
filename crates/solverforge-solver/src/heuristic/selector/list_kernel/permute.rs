//! Streamed contiguous-window permutation cursor.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::MAX_LIST_PERMUTE_WINDOW_SIZE;
use crate::heuristic::selector::list_support::ordered_index;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

use super::{PermuteEmitter, SelectedListOwners};

const STATIC_PERMUTE_START_SALT: u64 = 0x91D7_9E8A_0000_0002;
const STATIC_PERMUTE_SIZE_SALT: u64 = 0x91D7_9E8A_0000_0003;
const STATIC_PERMUTE_ORDER_SALT: u64 = 0x91D7_9E8A_0000_0004;

/// Canonical contiguous-window permutation cursor.
pub(crate) struct PermuteCursor<S, E>
where
    S: PlanningSolution,
    E: PermuteEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    owners: SelectedListOwners,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    entity_idx: usize,
    start_offset: usize,
    size_offset: usize,
    permutation_offset: usize,
    current_window: Option<(usize, usize)>,
    min_window_size: usize,
    max_window_size: usize,
    descriptor_index: usize,
}

impl<S, E> PermuteCursor<S, E>
where
    S: PlanningSolution,
    E: PermuteEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        min_window_size: usize,
        max_window_size: usize,
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
            entity_idx: 0,
            start_offset: 0,
            size_offset: 0,
            permutation_offset: 0,
            current_window: None,
            min_window_size,
            max_window_size,
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
        self.start_offset = 0;
        self.size_offset = 0;
        self.permutation_offset = 0;
        self.current_window = None;
    }

    fn advance_start(&mut self) {
        self.start_offset += 1;
        self.size_offset = 0;
        self.permutation_offset = 0;
        self.current_window = None;
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        loop {
            if self.entity_idx >= self.entities.len() {
                return None;
            }
            let entity = self.entities[self.entity_idx];
            let route_len = self.route_lens[self.entity_idx];
            if route_len < self.min_window_size {
                self.advance_entity();
                continue;
            }

            if let Some((start, size)) = self.current_window {
                let permutation_count = factorial(size).saturating_sub(1);
                if self.permutation_offset < permutation_count {
                    let rank = ordered_index(
                        self.permutation_offset,
                        permutation_count,
                        self.context,
                        STATIC_PERMUTE_ORDER_SALT
                            ^ entity as u64
                            ^ start as u64
                            ^ size as u64
                            ^ self.descriptor_index as u64,
                    ) + 1;
                    self.permutation_offset += 1;
                    let permutation = nth_permutation(size, rank);
                    if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                        graph.intra_list_permutation_introduces_cycle(
                            entity,
                            start,
                            start + size,
                            &permutation,
                        )
                    }) {
                        continue;
                    }
                    return Some(self.emitter.emit_permute(
                        entity,
                        start,
                        start + size,
                        permutation,
                    ));
                }
                self.current_window = None;
                self.size_offset += 1;
                self.permutation_offset = 0;
            }

            if self.start_offset >= route_len {
                self.advance_entity();
                continue;
            }
            let start = ordered_index(
                self.start_offset,
                route_len,
                self.context,
                STATIC_PERMUTE_START_SALT ^ entity as u64 ^ self.descriptor_index as u64,
            );
            let max_valid = self.max_window_size.min(route_len - start);
            if max_valid < self.min_window_size {
                self.advance_start();
                continue;
            }
            let size_count = max_valid - self.min_window_size + 1;
            if self.size_offset >= size_count {
                self.advance_start();
                continue;
            }
            let size = self.min_window_size
                + ordered_index(
                    self.size_offset,
                    size_count,
                    self.context,
                    STATIC_PERMUTE_SIZE_SALT ^ entity as u64 ^ start as u64,
                );
            if self.owners.is_present()
                && !self
                    .owners
                    .segment_allows(self.entity_idx, start, start + size, entity)
            {
                self.size_offset += 1;
                continue;
            }
            self.current_window = Some((start, size));
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for PermuteCursor<S, E>
where
    S: PlanningSolution,
    E: PermuteEmitter<S>,
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

impl<S, E> Iterator for PermuteCursor<S, E>
where
    S: PlanningSolution,
    E: PermuteEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

pub(crate) fn count_list_permute_moves_for_len(
    route_len: usize,
    min_window_size: usize,
    max_window_size: usize,
) -> usize {
    if route_len < min_window_size {
        return 0;
    }
    let mut count = 0;
    for start in 0..route_len {
        let max_valid = max_window_size.min(route_len - start);
        for size in min_window_size..=max_valid {
            count += factorial(size).saturating_sub(1);
        }
    }
    count
}

pub(crate) fn factorial(value: usize) -> usize {
    (2..=value).product()
}

fn nth_permutation(len: usize, mut rank: usize) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut remaining: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> = (0..len).collect();
    let mut permutation = SmallVec::new();
    for position in 0..len {
        let suffix = len - position - 1;
        let step = factorial(suffix);
        let index = rank / step;
        rank %= step;
        permutation.push(remaining.remove(index));
    }
    permutation
}
