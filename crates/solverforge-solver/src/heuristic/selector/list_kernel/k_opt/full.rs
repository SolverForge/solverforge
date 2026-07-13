//! Canonical exhaustive K-opt cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::selector::k_opt::{count_cut_combinations, cut_combination_at};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};

use super::KOptEmitter;

/// Streams every valid cut combination in declaration order, then every
/// reconnection pattern for that combination.
pub(crate) struct KOptCursor<'a, S, E>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    entity_lens: Vec<(usize, usize)>,
    entity_offset: usize,
    move_offset: usize,
    k: usize,
    min_segment_len: usize,
    patterns: &'a [KOptReconnection],
    context: MoveStreamContext,
    descriptor_index: usize,
}

impl<'a, S, E> KOptCursor<'a, S, E>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        entity_lens: Vec<(usize, usize)>,
        k: usize,
        min_segment_len: usize,
        patterns: &'a [KOptReconnection],
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        let mut entity_lens = entity_lens;
        context.apply_selection_order_without_replacement(
            &mut entity_lens,
            0x4B0F_7E11_7100_0001 ^ descriptor_index as u64,
        );
        Self {
            store: CandidateStore::new(),
            emitter,
            entity_lens,
            entity_offset: 0,
            move_offset: 0,
            k,
            min_segment_len,
            patterns,
            context,
            descriptor_index,
        }
    }

    #[inline(always)]
    fn next_move(&mut self) -> Option<E::Move> {
        if self.patterns.is_empty() {
            return None;
        }
        loop {
            let &(entity, route_len) = self.entity_lens.get(self.entity_offset)?;
            let cut_count = count_cut_combinations(self.k, route_len, self.min_segment_len);
            let move_count = cut_count.saturating_mul(self.patterns.len());
            if self.move_offset >= move_count {
                self.entity_offset += 1;
                self.move_offset = 0;
                continue;
            }
            let selected = self.context.selection_index(
                self.move_offset,
                move_count,
                0x4B0F_7E11_7100_0002 ^ self.descriptor_index as u64 ^ entity as u64,
            );
            self.move_offset += 1;
            let cut_offset = selected / self.patterns.len();
            let pattern_offset = selected % self.patterns.len();
            let cuts =
                cut_combination_at(self.k, route_len, self.min_segment_len, entity, cut_offset)
                    .expect("selected K-opt cut rank must map to a valid combination");
            return Some(
                self.emitter
                    .emit_k_opt(&cuts, &self.patterns[pattern_offset]),
            );
        }
    }
}

impl<S, E> MoveCursor<S, E::Move> for KOptCursor<'_, S, E>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
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
        predicate: for<'b> fn(MoveCandidateRef<'b, S, E::Move>) -> bool,
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

impl<S, E> Iterator for KOptCursor<'_, S, E>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
