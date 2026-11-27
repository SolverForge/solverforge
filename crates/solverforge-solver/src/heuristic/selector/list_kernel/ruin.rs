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

    /// Number of source lists in the pool.
    pub(super) fn route_count(&self) -> usize {
        match self {
            Self::Unrestricted(entities) => entities.len(),
            Self::OwnerRestricted(entities) => entities.len(),
        }
    }

    /// Entity index of pool entry `route`.
    pub(super) fn entity(&self, route: usize) -> usize {
        match self {
            Self::Unrestricted(entities) => entities[route].0,
            Self::OwnerRestricted(entities) => entities[route].0,
        }
    }

    /// Number of eligible positions of pool entry `route`.
    pub(super) fn position_count(&self, route: usize) -> usize {
        match self {
            Self::Unrestricted(entities) => entities[route].1,
            Self::OwnerRestricted(entities) => entities[route].1.len(),
        }
    }

    /// The `slot`-th eligible position of pool entry `route`, ascending.
    pub(super) fn position(&self, route: usize, slot: usize) -> usize {
        match self {
            Self::Unrestricted(_) => slot,
            Self::OwnerRestricted(entities) => entities[route].1[slot],
        }
    }

    /// Whether `position` of pool entry `route` may be removed.
    pub(super) fn is_eligible(&self, route: usize, position: usize) -> bool {
        match self {
            Self::Unrestricted(entities) => position < entities[route].1,
            Self::OwnerRestricted(entities) => entities[route].1.binary_search(&position).is_ok(),
        }
    }

    pub(super) fn contains(&self, entity: usize, position: usize) -> bool {
        let route = match self {
            Self::Unrestricted(entities) => entities.binary_search_by_key(&entity, |entry| entry.0),
            Self::OwnerRestricted(entities) => {
                entities.binary_search_by_key(&entity, |entry| entry.0)
            }
        };
        route.is_ok_and(|route| self.is_eligible(route, position))
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

    #[inline(always)]
    fn choose_ruin_count(&mut self, eligible_len: usize) -> usize {
        let min = self.min_ruin_count.min(eligible_len);
        let max = self.max_ruin_count.min(eligible_len);
        if min == max {
            min
        } else {
            self.rng.random_range(min..=max)
        }
    }

    fn next_ruin_move(&mut self) -> Option<E::Move> {
        let route = self.rng.random_range(0..self.source_pool.route_count());
        let eligible_len = self.source_pool.position_count(route);
        let ruin_count = self.choose_ruin_count(eligible_len);
        let indices = partial_shuffle_prefix(&mut self.rng, eligible_len, ruin_count, |slot| {
            self.source_pool.position(route, slot)
        });
        Some(
            self.emitter
                .emit_ruin(self.source_pool.entity(route), indices),
        )
    }
}

/// The first `count` entries of a partial Fisher-Yates shuffle of the
/// sequence `value(0)..value(len)`, with the draws of an in-place shuffle.
///
/// Only the swapped slots are recorded, so the sequence itself is never
/// materialized or copied.
fn partial_shuffle_prefix(
    rng: &mut SmallRng,
    len: usize,
    count: usize,
    value: impl Fn(usize) -> usize,
) -> SmallVec<[usize; 8]> {
    let mut swapped = SmallVec::<[(usize, usize); 16]>::new();
    let read = |swapped: &SmallVec<[(usize, usize); 16]>, slot: usize| {
        swapped
            .iter()
            .find(|(candidate, _)| *candidate == slot)
            .map_or_else(|| value(slot), |&(_, entry)| entry)
    };
    let mut prefix = SmallVec::with_capacity(count);
    for slot in 0..count {
        let swap_slot = rng.random_range(slot..len);
        let drawn = read(&swapped, swap_slot);
        let displaced = read(&swapped, slot);
        match swapped
            .iter_mut()
            .find(|(candidate, _)| *candidate == swap_slot)
        {
            Some(entry) => entry.1 = displaced,
            None => swapped.push((swap_slot, displaced)),
        }
        prefix.push(drawn);
    }
    prefix
}

impl<S, E> MoveCursor<S, E::Move> for RuinCursor<S, E>
where
    S: PlanningSolution,
    E: RuinEmitter<S>,
{
    fn next_candidate(&mut self, _solution: &S) -> Option<CandidateId> {
        if self.remaining_moves == 0 || self.source_pool.is_empty() {
            return None;
        }
        self.remaining_moves -= 1;
        let move_value = self.next_ruin_move()?;
        Some(self.store.push(move_value))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, E::Move>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> E::Move {
        self.store.take_candidate(id)
    }

    fn next_owned_candidate(&mut self, solution: &S) -> Option<E::Move> {
        let id = self.next_candidate(solution)?;
        Some(self.take_candidate(id))
    }

    fn next_owned_candidate_matching(
        &mut self,
        solution: &S,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, E::Move>) -> bool,
    ) -> Option<E::Move> {
        loop {
            let id = self.next_candidate(solution)?;
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
