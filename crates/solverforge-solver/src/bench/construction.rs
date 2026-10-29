//! The compiled scalar construction placer and its placement cursors.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::selector::move_selector::{CandidateId, MoveCandidateRef, MoveCursor};
use crate::phase::construction::{
    descriptor_placer, EntityPlacer, EntityPlacerCursor, Placement, RuntimeScalarCandidateCursor,
    RuntimeScalarConstructionCursor, RuntimeScalarConstructionMove,
    RuntimeScalarConstructionPlacer,
};
use crate::runtime::compiler::executor::ScalarConstructionHandoff;

use super::CandidateVisitor;

/// The placer a compiled descriptor-placement scalar construction drives.
pub struct CompiledScalarPlacer<S> {
    placer: RuntimeScalarConstructionPlacer<S>,
}

impl<S> CompiledScalarPlacer<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    pub(super) fn new(handoff: ScalarConstructionHandoff<S>) -> Self {
        Self {
            placer: descriptor_placer(&handoff.config, handoff.scalar_slots, handoff.slot_order),
        }
    }

    pub fn open_cursor<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> CompiledScalarPlacementCursor<'_, S> {
        CompiledScalarPlacementCursor {
            cursor: self.placer.open_cursor(score_director),
        }
    }
}

impl<S> Debug for CompiledScalarPlacer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledScalarPlacer")
            .field("placer", &self.placer)
            .finish()
    }
}

/// The placement cursor a construction phase opens over the placer.
pub struct CompiledScalarPlacementCursor<'a, S>
where
    S: PlanningSolution,
{
    cursor: RuntimeScalarConstructionCursor<'a, S>,
}

impl<S> CompiledScalarPlacementCursor<'_, S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    /// The next placement no completion check or stop request skips.
    pub fn next_placement<D: Director<S>>(
        &mut self,
        score_director: &D,
    ) -> Option<CompiledScalarPlacement<S>> {
        self.cursor
            .next_placement(score_director, |_| false, || false)
            .map(|placement| CompiledScalarPlacement { placement })
    }
}

impl<S: PlanningSolution> Debug for CompiledScalarPlacementCursor<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledScalarPlacementCursor")
            .finish_non_exhaustive()
    }
}

/// One placement and its candidate cursor.
pub struct CompiledScalarPlacement<S>
where
    S: PlanningSolution,
{
    placement: Placement<S, RuntimeScalarConstructionMove<S>, RuntimeScalarCandidateCursor<S>>,
}

impl<S> CompiledScalarPlacement<S>
where
    S: PlanningSolution,
{
    pub fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        self.placement.candidates_mut().next_candidate(solution)
    }

    /// Visits a live candidate; `None` when `id` is not live.
    pub fn visit_candidate<Visitor: CandidateVisitor<S>>(
        &self,
        id: CandidateId,
        visitor: Visitor,
    ) -> Option<Visitor::Output> {
        self.placement
            .candidates()
            .candidate(id)
            .map(|candidate| visitor.visit(candidate))
    }

    /// The `(entity index, value)` assignment a live candidate proposes.
    /// Static scalar slots expose no trace identity, so this is the
    /// representation-independent view of a construction candidate.
    pub fn candidate_assignment(&self, id: CandidateId) -> Option<(usize, usize)> {
        match self.placement.candidates().candidate(id)? {
            MoveCandidateRef::Borrowed(candidate) => {
                Some((candidate.entity_index(), candidate.value()))
            }
            MoveCandidateRef::Sequential(_) => {
                unreachable!("scalar construction candidates are single assignments")
            }
        }
    }

    pub fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.placement.candidates_mut().release_candidate(id)
    }
}

impl<S: PlanningSolution> Debug for CompiledScalarPlacement<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledScalarPlacement")
            .finish_non_exhaustive()
    }
}
