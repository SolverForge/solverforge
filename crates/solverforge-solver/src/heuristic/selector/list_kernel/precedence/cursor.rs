//! Streamed mixed move cursor for a precedence-critical neighborhood.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};

use super::coordinates::{
    critical_adjacent_sublist_swap, critical_permutation, critical_reverse, critical_ruin_indices,
    critical_sublist_change, critical_swap, move_introduces_route_cycle, non_adjacent_change,
    tiered_precedence_move_index, CriticalBlock,
};
use super::emission::PrecedenceEmitter;
use super::support::{
    critical_adjacent_swaps, multi_critical_ruin_count, multi_critical_ruin_sources,
    multi_support_swap_count, multi_support_swaps, support_adjacent_swaps,
};
use crate::heuristic::selector::precedence_route::PrecedenceRouteGraph;

pub(crate) struct PrecedenceCursor<S, E>
where
    S: PlanningSolution,
    E: PrecedenceEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    blocks: Vec<CriticalBlock>,
    route_graph: PrecedenceRouteGraph,
    context: MoveStreamContext,
    block_index: usize,
    move_index: usize,
    multi_swap_index: usize,
    multi_swap_count: usize,
    multi_ruin_index: usize,
    multi_ruin_count: usize,
    critical_swaps: Vec<super::support::AdjacentSwap>,
    support_swaps: Vec<super::support::AdjacentSwap>,
    emitter: E,
    descriptor_index: usize,
}

impl<S, E> PrecedenceCursor<S, E>
where
    S: PlanningSolution,
    E: PrecedenceEmitter<S>,
{
    pub(crate) fn new(
        blocks: Vec<CriticalBlock>,
        route_graph: PrecedenceRouteGraph,
        context: MoveStreamContext,
        emitter: E,
        descriptor_index: usize,
    ) -> Self {
        let critical_swaps = critical_adjacent_swaps(&blocks);
        let support_swaps = support_adjacent_swaps(&blocks, &route_graph);
        let multi_swap_count = multi_support_swap_count(&critical_swaps, &support_swaps);
        let multi_ruin_count = multi_critical_ruin_count(&blocks);
        Self {
            store: CandidateStore::new(),
            blocks,
            route_graph,
            context,
            block_index: 0,
            move_index: 0,
            multi_swap_index: 0,
            multi_swap_count,
            multi_ruin_index: 0,
            multi_ruin_count,
            critical_swaps,
            support_swaps,
            emitter,
            descriptor_index,
        }
    }

    fn push_move(&mut self, block: CriticalBlock, move_index: usize) -> CandidateId {
        let adjacent_count = block.len().saturating_sub(1);
        if move_index < adjacent_count {
            let source = block.start + move_index;
            return self
                .store
                .push(self.emitter.emit_change(block.entity, source, source + 2));
        }
        let change_count = block.change_move_count();
        if move_index < change_count {
            let (source, destination) = non_adjacent_change(block, move_index - adjacent_count);
            return self
                .store
                .push(self.emitter.emit_change(block.entity, source, destination));
        }
        let swap_count = block.swap_move_count();
        if move_index < change_count + swap_count {
            let (first, second) = critical_swap(block, move_index - change_count);
            return self
                .store
                .push(self.emitter.emit_swap(block.entity, first, second));
        }
        let reverse_count = block.reverse_move_count();
        if move_index < change_count + swap_count + reverse_count {
            let (start, end) = critical_reverse(block, move_index - change_count - swap_count);
            return self
                .store
                .push(self.emitter.emit_reverse(block.entity, start, end));
        }
        let sublist_swap_count = block.adjacent_sublist_swap_move_count();
        if move_index < change_count + swap_count + reverse_count + sublist_swap_count {
            let (first_start, first_end, second_start, second_end) = critical_adjacent_sublist_swap(
                block,
                move_index - change_count - swap_count - reverse_count,
            );
            return self.store.push(self.emitter.emit_sublist_swap(
                block.entity,
                first_start,
                first_end,
                second_start,
                second_end,
            ));
        }
        let ruin_count = block.ruin_move_count();
        if move_index < change_count + swap_count + reverse_count + sublist_swap_count + ruin_count
        {
            let indices = critical_ruin_indices(
                block,
                move_index - change_count - swap_count - reverse_count - sublist_swap_count,
            );
            let sources: SmallVec<[(usize, SmallVec<[usize; 8]>); 1]> =
                smallvec![(block.entity, indices)];
            return self.store.push(self.emitter.emit_ruin(&sources));
        }
        let sublist_change_count = block.sublist_change_move_count();
        if move_index
            < change_count
                + swap_count
                + reverse_count
                + sublist_swap_count
                + ruin_count
                + sublist_change_count
        {
            let (source_start, size, destination) = critical_sublist_change(
                block.start,
                block.len(),
                block.route_len,
                move_index
                    - change_count
                    - swap_count
                    - reverse_count
                    - sublist_swap_count
                    - ruin_count,
            );
            let source_start = block.start + source_start;
            return self.store.push(self.emitter.emit_sublist_change(
                block.entity,
                source_start,
                source_start + size,
                destination,
            ));
        }
        let (start_offset, size, permutation) = critical_permutation(
            block.len(),
            move_index
                - change_count
                - swap_count
                - reverse_count
                - sublist_swap_count
                - ruin_count
                - sublist_change_count,
        );
        let start = block.start + start_offset;
        self.store.push(
            self.emitter
                .emit_permute(block.entity, start, start + size, permutation),
        )
    }
}

impl<S, E> MoveCursor<S, E::Move> for PrecedenceCursor<S, E>
where
    S: PlanningSolution,
    E: PrecedenceEmitter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.multi_swap_index < self.multi_swap_count {
                let move_index = crate::heuristic::selector::list_support::ordered_index(
                    self.multi_swap_index,
                    self.multi_swap_count,
                    self.context,
                    0xC917_1EAF_5EED_0004 ^ self.descriptor_index as u64,
                );
                self.multi_swap_index += 1;
                let swaps =
                    multi_support_swaps(&self.critical_swaps, &self.support_swaps, move_index);
                if self
                    .route_graph
                    .multi_intra_list_swaps_introduce_cycle(&swaps)
                {
                    continue;
                }
                return Some(self.store.push(self.emitter.emit_multi_swap(&swaps)));
            }
            if self.multi_ruin_index < self.multi_ruin_count {
                let move_index = crate::heuristic::selector::list_support::ordered_index(
                    self.multi_ruin_index,
                    self.multi_ruin_count,
                    self.context,
                    0xC917_1EAF_5EED_0003 ^ self.descriptor_index as u64,
                );
                self.multi_ruin_index += 1;
                let sources = multi_critical_ruin_sources(&self.blocks, move_index);
                return Some(self.store.push(self.emitter.emit_ruin(&sources)));
            }
            if self.block_index >= self.blocks.len() {
                return None;
            }
            let block =
                *self
                    .blocks
                    .get(crate::heuristic::selector::list_support::ordered_index(
                        self.block_index,
                        self.blocks.len(),
                        self.context,
                        0xC917_1EAF_5EED_0001 ^ self.descriptor_index as u64,
                    ))?;
            let move_count = block.move_count();
            if self.move_index < move_count {
                let move_index = tiered_precedence_move_index(
                    block,
                    self.move_index,
                    self.context,
                    0xC917_1EAF_5EED_0002
                        ^ self.descriptor_index as u64
                        ^ block.entity as u64
                        ^ ((block.start as u64) << 16)
                        ^ ((block.end as u64) << 32),
                );
                self.move_index += 1;
                if move_introduces_route_cycle(block, move_index, &self.route_graph) {
                    continue;
                }
                return Some(self.push_move(block, move_index));
            }
            self.block_index += 1;
            self.move_index = 0;
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, E::Move>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> E::Move {
        self.store.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<E::Move> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, E::Move>) -> bool,
    ) -> Option<E::Move> {
        loop {
            let id = self.next_candidate()?;
            let matches = self.candidate(id).is_some_and(predicate);
            if matches {
                return Some(self.take_candidate(id));
            }
            self.release_candidate(id);
        }
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S, E> Iterator for PrecedenceCursor<S, E>
where
    S: PlanningSolution,
    E: PrecedenceEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
