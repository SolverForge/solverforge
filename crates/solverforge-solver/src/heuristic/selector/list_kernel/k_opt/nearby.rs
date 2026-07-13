//! Canonical distance-pruned K-opt cursor.

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::r#move::CutPoint;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveStreamContext,
};

use super::{KOptDistanceProbe, KOptEmitter, NearbyCutState};

/// Streams nearby cut sets and preserves the original candidate-store
/// ownership behavior for every emitted reconnection.
pub(crate) struct NearbyKOptCursor<'a, S, E, P>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
    P: KOptDistanceProbe<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    solution: S,
    distance: P,
    entity_lens: Vec<(usize, usize)>,
    entity_offset: usize,
    cut_state: Option<NearbyCutState>,
    pending_cuts: Option<Vec<CutPoint>>,
    pattern_offset: usize,
    k: usize,
    min_segment_len: usize,
    max_nearby: usize,
    patterns: &'a [KOptReconnection],
    context: MoveStreamContext,
    descriptor_index: usize,
}

impl<'a, S, E, P> NearbyKOptCursor<'a, S, E, P>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
    P: KOptDistanceProbe<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        solution: S,
        distance: P,
        entities: Vec<usize>,
        k: usize,
        min_segment_len: usize,
        max_nearby: usize,
        patterns: &'a [KOptReconnection],
        route_len: impl Fn(&S, usize) -> usize,
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        let mut entity_lens = entities
            .into_iter()
            .map(|entity| (entity, route_len(&solution, entity)))
            .collect::<Vec<_>>();
        context.apply_selection_order_without_replacement(
            &mut entity_lens,
            0x4B0F_7E11_72EA_0003 ^ descriptor_index as u64,
        );
        Self {
            store: CandidateStore::new(),
            emitter,
            solution,
            distance,
            entity_lens,
            entity_offset: 0,
            cut_state: None,
            pending_cuts: None,
            pattern_offset: 0,
            k,
            min_segment_len,
            max_nearby,
            patterns,
            context,
            descriptor_index,
        }
    }

    fn load_next_cut_state(&mut self) -> bool {
        while let Some(&(entity, route_len)) = self.entity_lens.get(self.entity_offset) {
            self.entity_offset += 1;
            let state = NearbyCutState::new(
                entity,
                self.k,
                route_len,
                self.min_segment_len,
                self.max_nearby,
                self.context,
                0x4B0F_7E11_72EA_0004 ^ self.descriptor_index as u64 ^ entity as u64,
            );
            if !state.is_done() {
                self.cut_state = Some(state);
                return true;
            }
        }
        false
    }
}

impl<S, E, P> MoveCursor<S, E::Move> for NearbyKOptCursor<'_, S, E, P>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
    P: KOptDistanceProbe<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if let Some(cuts) = self.pending_cuts.as_ref() {
                if self.pattern_offset < self.patterns.len() {
                    let salt = cuts.iter().fold(
                        0x4B0F_7E11_72EA_0005 ^ self.descriptor_index as u64,
                        |salt, cut| {
                            salt ^ (cut.entity_index() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                ^ (cut.position() as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
                        },
                    );
                    let pattern = &self.patterns[self.context.selection_index(
                        self.pattern_offset,
                        self.patterns.len(),
                        salt,
                    )];
                    self.pattern_offset += 1;
                    return Some(self.store.push(self.emitter.emit_k_opt(cuts, pattern)));
                }
                self.pending_cuts = None;
                self.pattern_offset = 0;
            }
            if self.cut_state.is_none() && !self.load_next_cut_state() {
                return None;
            }
            if let Some(state) = self.cut_state.as_mut() {
                if let Some(mut cuts) = state.next_cuts(&self.solution, &self.distance) {
                    cuts.sort_by_key(|cut| cut.position());
                    self.pending_cuts = Some(cuts);
                    self.pattern_offset = 0;
                    continue;
                }
            }
            self.cut_state = None;
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
        predicate: for<'b> fn(MoveCandidateRef<'b, S, E::Move>) -> bool,
    ) -> Option<E::Move> {
        loop {
            let id = self.next_candidate()?;
            let matched = self.candidate(id).is_some_and(predicate);
            let move_value = self.take_candidate(id);
            if matched {
                return Some(move_value);
            }
        }
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S, E, P> Iterator for NearbyKOptCursor<'_, S, E, P>
where
    S: PlanningSolution,
    E: KOptEmitter<S>,
    P: KOptDistanceProbe<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
