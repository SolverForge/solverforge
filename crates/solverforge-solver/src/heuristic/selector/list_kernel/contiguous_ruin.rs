//! Contiguous list-ruin coordinates.
//!
//! One move removes consecutive positions around a random seed and, on up to
//! `max_routes - 1` other lists, consecutive positions around the position
//! nearest to that seed. Recreate stays in the emitted move, exactly as for
//! [`super::RuinCursor`].

use rand::rngs::SmallRng;
use rand::RngExt;
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::list_kernel::RuinSources;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};

use super::{MultiRuinEmitter, RuinSourcePool};

/// Distance between two list positions, read when a contiguous ruin looks for
/// the position nearest to its seed on another list.
pub(crate) trait RuinDistanceProbe<S>: Send + Sync {
    fn distance(
        &self,
        solution: &S,
        seed_entity: usize,
        seed_position: usize,
        entity: usize,
        position: usize,
    ) -> f64;
}

/// One growing segment: inclusive `start..=end` on the pool entry `route`.
struct Segment {
    route: usize,
    start: usize,
    end: usize,
    left_open: bool,
    right_open: bool,
}

impl Segment {
    fn new(route: usize, position: usize) -> Self {
        Self {
            route,
            start: position,
            end: position,
            left_open: true,
            right_open: true,
        }
    }

    /// Extends the segment by one eligible neighbour, trying a random side
    /// first. A side closes at the list end or at the first position that is
    /// not eligible, so a segment never crosses one.
    fn grow(&mut self, pool: &RuinSourcePool, rng: &mut SmallRng) -> bool {
        let left_first = rng.random::<bool>();
        for left in [left_first, !left_first] {
            if left && self.left_open {
                match self.start.checked_sub(1) {
                    Some(position) if pool.is_eligible(self.route, position) => {
                        self.start = position;
                        return true;
                    }
                    _ => self.left_open = false,
                }
            } else if !left && self.right_open {
                let position = self.end + 1;
                if pool.is_eligible(self.route, position) {
                    self.end = position;
                    return true;
                }
                self.right_open = false;
            }
        }
        false
    }
}

/// Contiguous random ruin coordinate cursor.
pub(crate) struct ContiguousRuinCursor<S, E, P>
where
    S: PlanningSolution,
    E: MultiRuinEmitter<S>,
{
    store: CandidateStore<S, E::Move>,
    emitter: E,
    probe: P,
    rng: SmallRng,
    source_pool: RuinSourcePool,
    remaining_moves: usize,
    min_ruin_count: usize,
    max_ruin_count: usize,
    max_routes: usize,
}

impl<S, E, P> ContiguousRuinCursor<S, E, P>
where
    S: PlanningSolution,
    E: MultiRuinEmitter<S>,
    P: RuinDistanceProbe<S>,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        emitter: E,
        probe: P,
        rng: SmallRng,
        source_pool: RuinSourcePool,
        remaining_moves: usize,
        min_ruin_count: usize,
        max_ruin_count: usize,
        max_routes: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            emitter,
            probe,
            rng,
            source_pool,
            remaining_moves,
            min_ruin_count,
            max_ruin_count,
            max_routes,
        }
    }

    /// The pool entry and position nearest to the seed on a list no segment
    /// uses yet; the first one wins a tie.
    fn nearest_unused(
        &self,
        solution: &S,
        segments: &[Segment],
        seed_entity: usize,
        seed_position: usize,
    ) -> Option<(usize, usize)> {
        let pool = &self.source_pool;
        let mut nearest: Option<(usize, usize, f64)> = None;
        for route in 0..pool.route_count() {
            if segments.iter().any(|segment| segment.route == route) {
                continue;
            }
            let entity = pool.entity(route);
            for slot in 0..pool.position_count(route) {
                let position = pool.position(route, slot);
                let distance =
                    self.probe
                        .distance(solution, seed_entity, seed_position, entity, position);
                if nearest.is_none_or(|(_, _, best)| distance.total_cmp(&best).is_lt()) {
                    nearest = Some((route, position, distance));
                }
            }
        }
        nearest.map(|(route, position, _)| (route, position))
    }

    fn next_move(&mut self, solution: &S) -> Option<E::Move> {
        let seed_route = self.rng.random_range(0..self.source_pool.route_count());
        let seed_slot = self
            .rng
            .random_range(0..self.source_pool.position_count(seed_route));
        let seed_position = self.source_pool.position(seed_route, seed_slot);
        let seed_entity = self.source_pool.entity(seed_route);
        let target = if self.min_ruin_count == self.max_ruin_count {
            self.min_ruin_count
        } else {
            self.rng
                .random_range(self.min_ruin_count..=self.max_ruin_count)
        };

        let mut segments = SmallVec::<[Segment; 4]>::new();
        segments.push(Segment::new(seed_route, seed_position));
        while segments.len() < self.max_routes.min(target) {
            let Some((route, position)) =
                self.nearest_unused(solution, &segments, seed_entity, seed_position)
            else {
                break;
            };
            segments.push(Segment::new(route, position));
        }

        let mut removed = segments.len();
        while removed < target {
            let mut grew = false;
            for segment in &mut segments {
                if removed == target {
                    break;
                }
                if segment.grow(&self.source_pool, &mut self.rng) {
                    removed += 1;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }

        if removed < self.min_ruin_count {
            return None;
        }
        let mut sources = segments
            .iter()
            .map(|segment| {
                (
                    self.source_pool.entity(segment.route),
                    (segment.start..=segment.end).collect::<SmallVec<[usize; 8]>>(),
                )
            })
            .collect::<RuinSources>();
        sources.sort_unstable_by_key(|(entity, _)| *entity);
        self.emitter.emit_ruin_sources(
            sources,
            self.max_ruin_count,
            self.max_routes,
            |entity, position| self.source_pool.contains(entity, position),
        )
    }
}

impl<S, E, P> MoveCursor<S, E::Move> for ContiguousRuinCursor<S, E, P>
where
    S: PlanningSolution,
    E: MultiRuinEmitter<S>,
    P: RuinDistanceProbe<S>,
{
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        if self.source_pool.is_empty() {
            return None;
        }
        while self.remaining_moves > 0 {
            self.remaining_moves -= 1;
            if let Some(move_value) = self.next_move(solution) {
                return Some(self.store.push(move_value));
            }
        }
        None
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
