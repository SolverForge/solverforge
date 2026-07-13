//! Move projection adapters for the canonical scalar recipe cursor.
//!
//! These adapters only reify an already-selected owned runtime move into an
//! established public move payload. They never inspect a model, invoke a
//! source, rank candidates, or open another scalar cursor.

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_core::score::Score;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::r#move::{DynamicScalarChangeMove, DynamicScalarSwapMove, Move};
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};

use super::{RuntimeScalarMove, RuntimeScalarNeighborhoodCursor, RuntimeScalarRecipe};

/// Candidate-store boundary for a typed move projection. The inner canonical cursor
/// transfers each recipe by ownership, so a selected public candidate remains
/// owned by exactly one outer store.
pub struct RuntimeScalarFacadeCursor<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    inner: RuntimeScalarNeighborhoodCursor<S>,
    store: CandidateStore<S, M>,
    emit: fn(RuntimeScalarMove<S>) -> M,
}

impl<S, M> RuntimeScalarFacadeCursor<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    pub(crate) fn new(
        inner: RuntimeScalarNeighborhoodCursor<S>,
        emit: fn(RuntimeScalarMove<S>) -> M,
    ) -> Self {
        Self {
            inner,
            store: CandidateStore::new(),
            emit,
        }
    }

    fn next_move(&mut self) -> Option<M> {
        self.inner.next_owned_candidate().map(self.emit)
    }
}

impl<S, M> MoveCursor<S, M> for RuntimeScalarFacadeCursor<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.next_move().map(|mov| self.store.push(mov))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.store.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<M> {
        self.next_move()
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S, M> Iterator for RuntimeScalarFacadeCursor<S, M>
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

pub(crate) fn emit_dynamic_scalar_change_move<S>(
    runtime_move: RuntimeScalarMove<S>,
) -> DynamicScalarChangeMove<S>
where
    S: PlanningSolution,
{
    match runtime_move.into_recipe() {
        RuntimeScalarRecipe::Change {
            slot: RuntimeScalarSlot::Dynamic(slot),
            entity_index,
            to_value,
        } => DynamicScalarChangeMove::new(slot, entity_index, to_value),
        _ => panic!("dynamic scalar change facade must only reify change recipes"),
    }
}

pub(crate) fn emit_dynamic_scalar_swap_move<S>(
    runtime_move: RuntimeScalarMove<S>,
) -> DynamicScalarSwapMove<S>
where
    S: PlanningSolution,
{
    match runtime_move.into_recipe() {
        RuntimeScalarRecipe::Swap {
            slot: RuntimeScalarSlot::Dynamic(slot),
            left_entity_index,
            right_entity_index,
        } => DynamicScalarSwapMove::new(slot, left_entity_index, right_entity_index),
        _ => panic!("dynamic scalar swap facade must only reify swap recipes"),
    }
}

pub(crate) fn dynamic_slot<S>(slot: DynamicScalarVariableSlot<S>) -> RuntimeScalarSlot<S> {
    RuntimeScalarSlot::Dynamic(slot)
}
