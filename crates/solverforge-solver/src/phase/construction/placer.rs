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
    pub(crate) fn new(entity_ref: EntityReference, candidates: C) -> Self {
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

    pub(crate) fn with_keep_current_legal(mut self, legal: bool) -> Self {
        self.keep_current_legal = legal;
        self
    }

    pub(crate) fn keep_current_legal(&self) -> bool {
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

    pub(crate) fn take_move(&mut self, candidate_id: CandidateId) -> M {
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

#[cfg(test)]
mod tests;
