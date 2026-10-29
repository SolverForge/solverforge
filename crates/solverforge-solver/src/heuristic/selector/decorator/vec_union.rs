/* Canonical vector-backed union scheduling for selector composition. */

use std::marker::PhantomData;

use solverforge_config::{UnionSelectionOrder, UnionWeighting};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveStreamContext, ResourceMoveCursor,
};

/// Applies the canonical union child-context rule without retaining a frozen
/// `VecUnionSelector` declaration.
///
/// Stateful composition executions consume that declaration so they can own
/// one stream-state tree for the whole solve. They call this helper instead
/// of recreating union seeding or ordering behavior at the phase boundary.
pub(crate) fn union_child_context(
    _selection_order: UnionSelectionOrder,
    context: MoveStreamContext,
) -> MoveStreamContext {
    context
}

pub(crate) fn resolve_union_weights(
    weighting: UnionWeighting,
    fixed: &[u64],
    child_sizes: &[usize],
) -> Vec<u64> {
    match weighting {
        UnionWeighting::Equal => {
            assert!(
                fixed.is_empty(),
                "equal union weighting does not accept explicit weights"
            );
            vec![1; child_sizes.len()]
        }
        UnionWeighting::Fixed => {
            assert_eq!(
                fixed.len(),
                child_sizes.len(),
                "fixed union weight count must match child count"
            );
            assert!(
                fixed.iter().all(|weight| *weight > 0),
                "fixed union weights must all be positive"
            );
            fixed.to_vec()
        }
        UnionWeighting::CandidateCount => {
            assert!(
                fixed.is_empty(),
                "candidate_count union weighting does not accept explicit weights"
            );
            child_sizes
                .iter()
                .map(|size| u64::try_from(*size).unwrap_or(u64::MAX))
                .collect()
        }
    }
}

mod scheduler;
pub(crate) use scheduler::UnionScheduler;

pub(crate) struct VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    cursors: Vec<C>,
    scheduler: UnionScheduler,
    discovered: Vec<(usize, CandidateId)>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    pub(crate) fn new(
        cursors: Vec<C>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            scheduler: UnionScheduler::new(cursors.len(), selection_order, context, weights),
            cursors,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }

    fn push_discovered(&mut self, cursor_index: usize, child_index: CandidateId) -> CandidateId {
        let global_id = CandidateId::new(self.discovered.len());
        self.discovered.push((cursor_index, child_index));
        self.cursors[cursor_index]
            .candidate(child_index)
            .expect("vec union candidate must remain valid");
        global_id
    }
}

impl<S, M, C> MoveCursor<S, M> for VecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: MoveCursor<S, M>,
{
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate(solution)
        })?;
        Some(self.push_discovered(cursor_index, child_index))
    }

    fn next_candidate_with_director<D: Director<S>>(
        &mut self,
        director: &mut D,
    ) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate_with_director(director)
        })?;
        Some(self.push_discovered(cursor_index, child_index))
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index.index())?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].take_candidate(child_index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].apply_owned_candidate(child_index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(&(cursor_index, child_index)) = self.discovered.get(index.index()) else {
            return false;
        };
        self.cursors[cursor_index].release_candidate(child_index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.discovered
            .get(index.index())
            .map(|(cursor_index, _)| *cursor_index)
    }
}

/// Resource-aware facade over the canonical [`UnionScheduler`].
///
/// This deliberately shares no ordering implementation with a second union
/// cursor. It differs from [`VecUnionMoveCursor`] only in how the selected
/// child is asked for its next candidate: a caller-owned solve resource is
/// lent at that exact reachable pull boundary.
pub(crate) struct ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
{
    cursors: Vec<C>,
    scheduler: UnionScheduler,
    discovered: Vec<(usize, CandidateId)>,
    _phantom: PhantomData<(fn() -> S, fn() -> M)>,
}

impl<S, M, C> ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(crate) fn new(
        cursors: Vec<C>,
        selection_order: UnionSelectionOrder,
        context: MoveStreamContext,
        weights: Vec<u64>,
    ) -> Self {
        Self {
            scheduler: UnionScheduler::new(cursors.len(), selection_order, context, weights),
            cursors,
            discovered: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<S, M, C> ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
{
    pub(crate) fn child_cursors_mut(&mut self) -> &mut [C] {
        &mut self.cursors
    }
}

impl<S, M, C, Resources> ResourceMoveCursor<S, M, Resources> for ResourceVecUnionMoveCursor<S, M, C>
where
    S: PlanningSolution,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    fn next_candidate_with_resources<D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId> {
        let cursor_count = self.cursors.len();
        let (cursor_index, child_index) = self.scheduler.next_child(cursor_count, |index| {
            self.cursors[index].next_candidate_with_resources(director, resources)
        })?;
        let global_id = CandidateId::new(self.discovered.len());
        self.discovered.push((cursor_index, child_index));
        self.cursors[cursor_index]
            .candidate(child_index)
            .expect("resource union candidate must remain valid");
        Some(global_id)
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        let (cursor_index, child_index) = *self.discovered.get(index.index())?;
        self.cursors[cursor_index].candidate(child_index)
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].take_candidate(child_index)
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        let (cursor_index, child_index) = self.discovered[index.index()];
        self.cursors[cursor_index].apply_owned_candidate(child_index, score_director);
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        let Some(&(cursor_index, child_index)) = self.discovered.get(index.index()) else {
            return false;
        };
        self.cursors[cursor_index].release_candidate(child_index)
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        self.discovered
            .get(index.index())
            .map(|(cursor_index, _)| *cursor_index)
    }
}

#[cfg(test)]
#[path = "vec_union/tests.rs"]
mod tests;
