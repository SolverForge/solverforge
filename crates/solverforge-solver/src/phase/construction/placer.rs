/* Entity placers for construction heuristic

Placers enumerate the entities that need values assigned and
generate candidate moves for each entity.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ChangeMove, Move};
use crate::heuristic::selector::move_selector::{CandidateId, MoveCandidateRef, MoveCursor};
use crate::heuristic::selector::{EntityReference, EntitySelector, ValueSelector};
use crate::stats::CandidateTracePullToken;

use super::{ConstructionGroupSlotId, ConstructionSlotId};

#[derive(Clone, Debug, Default)]
pub(crate) struct ConstructionTarget {
    scalar_slots: Vec<ConstructionSlotId>,
    group_slot: Option<ConstructionGroupSlotId>,
}

impl ConstructionTarget {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_scalar_slots(mut self, mut scalar_slots: Vec<ConstructionSlotId>) -> Self {
        scalar_slots.sort_unstable();
        scalar_slots.dedup();
        self.scalar_slots = scalar_slots;
        self
    }

    pub(crate) fn with_group_slot(mut self, group_slot: ConstructionGroupSlotId) -> Self {
        self.group_slot = Some(group_slot);
        self
    }

    pub(crate) fn scalar_slots(&self) -> &[ConstructionSlotId] {
        &self.scalar_slots
    }

    pub(crate) fn group_slot(&self) -> Option<&ConstructionGroupSlotId> {
        self.group_slot.as_ref()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.scalar_slots.is_empty() && self.group_slot.is_none()
    }
}

/// A placement represents an entity that needs a value assigned,
/// along with the candidate moves to assign values.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub struct Placement<S, M, C = crate::heuristic::selector::move_selector::ArenaMoveCursor<S, M>>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    // The entity reference.
    pub entity_ref: EntityReference,
    candidates: C,
    // Whether keeping the current value is a legal construction choice.
    keep_current_legal: bool,
    target: ConstructionTarget,
    candidate_target: fn(&C, CandidateId) -> Option<&ConstructionTarget>,
    // Captured-only mapping from a cursor candidate to its bounded diagnostic
    // pull token. It stays empty trace-off and after trace saturation.
    candidate_trace_tokens: Vec<(CandidateId, CandidateTracePullToken)>,
    candidate_scores: Vec<(CandidateId, S::Score)>,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, C> Placement<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    pub fn new(entity_ref: EntityReference, candidates: C) -> Self {
        Self {
            entity_ref,
            candidates,
            keep_current_legal: false,
            target: ConstructionTarget::new(),
            candidate_target: |_, _| None,
            candidate_trace_tokens: Vec::new(),
            candidate_scores: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_keep_current_legal(mut self, legal: bool) -> Self {
        self.keep_current_legal = legal;
        self
    }

    pub fn keep_current_legal(&self) -> bool {
        self.keep_current_legal
    }

    pub(crate) fn with_slot_id(mut self, slot_id: ConstructionSlotId) -> Self {
        self.target = self.target.with_scalar_slots(vec![slot_id]);
        self
    }

    pub(crate) fn with_scalar_slots(mut self, mut scalar_slots: Vec<ConstructionSlotId>) -> Self {
        scalar_slots.sort_unstable();
        scalar_slots.dedup();
        self.target = self.target.with_scalar_slots(scalar_slots);
        self
    }

    pub(crate) fn with_group_slot(mut self, group_slot: ConstructionGroupSlotId) -> Self {
        self.target = self.target.with_group_slot(group_slot);
        self
    }

    pub(crate) fn with_candidate_target(
        mut self,
        candidate_target: fn(&C, CandidateId) -> Option<&ConstructionTarget>,
    ) -> Self {
        self.candidate_target = candidate_target;
        self
    }

    pub(crate) fn construction_target(&self) -> &ConstructionTarget {
        &self.target
    }

    pub(crate) fn construction_target_for_move(
        &self,
        candidate_id: CandidateId,
    ) -> &ConstructionTarget {
        (self.candidate_target)(&self.candidates, candidate_id).unwrap_or(&self.target)
    }

    pub fn candidates(&self) -> &C {
        &self.candidates
    }

    pub fn candidates_mut(&mut self) -> &mut C {
        &mut self.candidates
    }

    pub fn take_move(&mut self, candidate_id: CandidateId) -> M {
        self.candidates.take_candidate(candidate_id)
    }

    pub(crate) fn record_candidate_trace_token(
        &mut self,
        candidate_id: CandidateId,
        token: CandidateTracePullToken,
    ) {
        self.candidate_trace_tokens.push((candidate_id, token));
    }

    pub(crate) fn candidate_trace_token(
        &self,
        candidate_id: CandidateId,
    ) -> Option<CandidateTracePullToken> {
        self.candidate_trace_tokens
            .iter()
            .find_map(|(recorded_id, token)| (*recorded_id == candidate_id).then_some(*token))
    }

    pub(crate) fn record_candidate_score(&mut self, candidate_id: CandidateId, score: S::Score) {
        self.candidate_scores.push((candidate_id, score));
    }

    pub(crate) fn candidate_score(&self, candidate_id: CandidateId) -> Option<S::Score>
    where
        S::Score: Copy,
    {
        self.candidate_scores
            .iter()
            .find_map(|(recorded_id, score)| (*recorded_id == candidate_id).then_some(*score))
    }
}

impl<S, M, C> Debug for Placement<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placement")
            .field("entity_ref", &self.entity_ref)
            .field("keep_current_legal", &self.keep_current_legal)
            .field("target", &self.target)
            .finish()
    }
}

/// Trait for placing entities during construction.
///
/// Entity placers iterate over uninitialized entities and generate
/// candidate moves for each.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait EntityPlacerCursor<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type CandidateCursor: MoveCursor<S, M>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        is_completed: IsCompleted,
        should_stop: ShouldStop,
    ) -> Option<Placement<S, M, Self::CandidateCursor>>
    where
        D: Director<S>,
        IsCompleted: FnMut(&Placement<S, M, Self::CandidateCursor>) -> bool,
        ShouldStop: FnMut() -> bool;
}

pub trait EntityPlacer<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Cursor<'a>: EntityPlacerCursor<S, M> + 'a
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a>;
}

include!("placer/queued.rs");
/// Entity placer that sorts placements by a comparator function.
///
/// Wraps an inner placer and sorts its placements using a concrete comparator.
/// This enables FIRST_FIT_DECREASING and similar construction variants.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::construction::{SortedEntityPlacer, QueuedEntityPlacer, EntityPlacer};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::{FromSolutionEntitySelector, StaticValueSelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
/// use solverforge_scoring::ScoreDirector;
/// use std::cmp::Ordering;
///
/// #[derive(Clone, Debug)]
/// struct Task { difficulty: i32, assigned: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_assigned(s: &Solution, i: usize) -> Option<i32> {
///     s.tasks.get(i).and_then(|t| t.assigned)
/// }
/// fn set_assigned(s: &mut Solution, i: usize, v: Option<i32>) {
///     if let Some(t) = s.tasks.get_mut(i) { t.assigned = v; }
/// }
///
/// // Sort entities by difficulty (descending) for FIRST_FIT_DECREASING
/// fn difficulty_descending(s: &Solution, a: usize, b: usize) -> Ordering {
///     let da = s.tasks.get(a).map(|t| t.difficulty).unwrap_or(0);
///     let db = s.tasks.get(b).map(|t| t.difficulty).unwrap_or(0);
///     db.cmp(&da)  // Descending order
/// }
/// ```
pub struct SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    inner: Inner,
    // Comparator function: takes (solution, entity_index_a, entity_index_b) -> Ordering
    comparator: fn(&S, usize, usize) -> std::cmp::Ordering,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, Inner> SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    /// Creates a new sorted entity placer.
    ///
    /// # Arguments
    /// * `inner` - The inner placer to wrap
    /// * `comparator` - Function to compare entities: `(solution, idx_a, idx_b) -> Ordering`
    pub fn new(inner: Inner, comparator: fn(&S, usize, usize) -> std::cmp::Ordering) -> Self {
        Self {
            inner,
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner> Debug for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortedEntityPlacer")
            .field("inner", &self.inner)
            .finish()
    }
}

type InnerPlacementCandidateCursor<'a, S, M, Inner> =
    <<Inner as EntityPlacer<S, M>>::Cursor<'a> as EntityPlacerCursor<S, M>>::CandidateCursor;

pub struct SortedEntityPlacerCursor<'a, S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M> + 'a,
{
    inner: Inner::Cursor<'a>,
    comparator: fn(&S, usize, usize) -> std::cmp::Ordering,
    placements:
        Option<std::vec::IntoIter<Placement<S, M, InnerPlacementCandidateCursor<'a, S, M, Inner>>>>,
}

impl<'a, S, M, Inner> EntityPlacerCursor<S, M> for SortedEntityPlacerCursor<'a, S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M> + 'a,
{
    type CandidateCursor = InnerPlacementCandidateCursor<'a, S, M, Inner>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<S, M, Self::CandidateCursor>>
    where
        D: Director<S>,
        IsCompleted: FnMut(&Placement<S, M, Self::CandidateCursor>) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        if self.placements.is_none() {
            let mut placements = Vec::new();
            while let Some(placement) =
                self.inner
                    .next_placement(score_director, |_| false, &mut should_stop)
            {
                placements.push(placement);
            }
            let solution = score_director.working_solution();
            let comparator = self.comparator;
            placements.sort_by(|left, right| {
                comparator(
                    solution,
                    left.entity_ref.entity_index,
                    right.entity_ref.entity_index,
                )
            });
            self.placements = Some(placements.into_iter());
        }

        let placements = self
            .placements
            .as_mut()
            .expect("sorted placement cursor must be initialized");
        while !should_stop() {
            let placement = placements.next()?;
            if !is_completed(&placement) {
                return Some(placement);
            }
        }
        None
    }
}

impl<S, M, Inner> EntityPlacer<S, M> for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    type Cursor<'a>
        = SortedEntityPlacerCursor<'a, S, M, Inner>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        SortedEntityPlacerCursor {
            inner: self.inner.open_cursor(score_director),
            comparator: self.comparator,
            placements: None,
        }
    }
}

#[cfg(test)]
mod tests;
