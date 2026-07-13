//! Canonical streamed list-ruin coordinates.
//!
//! The cursor owns only randomized source selection and candidate ownership.
//! Recreate semantics stay in the emitted move, so public static and future
//! runtime carriers consume one coordinate stream without a selector fallback.

use rand::rngs::SmallRng;
use rand::RngExt;
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};

use super::RuinEmitter;

/// Frozen source positions eligible for one ruin cursor.
///
/// A restricted pool stores only positions whose owner restriction admits at
/// least one destination. The emitted move rechecks the live restriction when
/// it performs recreate, exactly as the historic move did.
#[derive(Clone, Debug)]
pub(crate) enum RuinSourcePool {
    Unrestricted(Vec<(usize, usize)>),
    OwnerRestricted(Vec<(usize, SmallVec<[usize; 8]>)>),
}

impl RuinSourcePool {
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Unrestricted(entities) => entities.is_empty(),
            Self::OwnerRestricted(entities) => entities.is_empty(),
        }
    }
}

/// Canonical random ruin coordinate cursor.
pub(crate) struct RuinCursor<S, E>
where
    S: PlanningSolution,
    E: RuinEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    rng: SmallRng,
    source_pool: RuinSourcePool,
    remaining_moves: usize,
    min_ruin_count: usize,
    max_ruin_count: usize,
}

impl<S, E> RuinCursor<S, E>
where
    S: PlanningSolution,
    E: RuinEmitter<S>,
{
    pub(crate) fn new(
        emitter: E,
        rng: SmallRng,
        source_pool: RuinSourcePool,
        remaining_moves: usize,
        min_ruin_count: usize,
        max_ruin_count: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            rng,
            source_pool,
            remaining_moves,
            min_ruin_count,
            max_ruin_count,
        }
    }

    fn choose_ruin_count(&mut self, eligible_len: usize) -> usize {
        let min = self.min_ruin_count.min(eligible_len);
        let max = self.max_ruin_count.min(eligible_len);
        if min == max {
            min
        } else {
            self.rng.random_range(min..=max)
        }
    }

    fn next_unrestricted_move(&mut self) -> Option<E::Move> {
        let RuinSourcePool::Unrestricted(entities) = &self.source_pool else {
            return None;
        };
        let &(entity, list_len) = entities.get(self.rng.random_range(0..entities.len()))?;
        let ruin_count = self.choose_ruin_count(list_len);
        let mut indices: SmallVec<[usize; 8]> = (0..list_len).collect();
        for index in 0..ruin_count {
            let swap_index = self.rng.random_range(index..list_len);
            indices.swap(index, swap_index);
        }
        indices.truncate(ruin_count);
        Some(self.emitter.emit_ruin(entity, &indices))
    }

    fn next_owner_restricted_move(&mut self) -> Option<E::Move> {
        let (entity, mut indices) = {
            let RuinSourcePool::OwnerRestricted(entities) = &self.source_pool else {
                return None;
            };
            let (entity, eligible_indices) =
                entities.get(self.rng.random_range(0..entities.len()))?;
            (*entity, eligible_indices.clone())
        };
        let eligible_len = indices.len();
        let ruin_count = self.choose_ruin_count(eligible_len);
        for index in 0..ruin_count {
            let swap_index = self.rng.random_range(index..eligible_len);
            indices.swap(index, swap_index);
        }
        indices.truncate(ruin_count);
        Some(self.emitter.emit_ruin(entity, &indices))
    }
}

impl<S, E> MoveCursor<S, E::Move> for RuinCursor<S, E>
where
    S: PlanningSolution,
    E: RuinEmitter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.remaining_moves == 0 || self.source_pool.is_empty() {
            return None;
        }
        self.remaining_moves -= 1;
        let move_value = match self.source_pool {
            RuinSourcePool::Unrestricted(_) => self.next_unrestricted_move(),
            RuinSourcePool::OwnerRestricted(_) => self.next_owner_restricted_move(),
        }?;
        Some(self.store.push(move_value))
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
            let move_value = self.take_candidate(id);
            if matches {
                return Some(move_value);
            }
        }
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S, E> Iterator for RuinCursor<S, E>
where
    S: PlanningSolution,
    E: RuinEmitter<S>,
{
    type Item = E::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
