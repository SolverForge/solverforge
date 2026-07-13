use std::fmt::Debug;

use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use super::list_kernel::{
    change_candidate_trace_identity, change_do_move, change_is_doable, change_tabu_signature,
    change_undo_move, ChangeCoordinates, ChangeValueTransfer,
};
use super::{Move, MoveTabuSignature};

pub struct DynamicListChangeMove<S> {
    slot: DynamicListVariableSlot<S>,
    source_entity_index: usize,
    source_position: usize,
    dest_entity_index: usize,
    dest_position: usize,
    indices: [usize; 2],
}

impl<S> Clone for DynamicListChangeMove<S> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            source_entity_index: self.source_entity_index,
            source_position: self.source_position,
            dest_entity_index: self.dest_entity_index,
            dest_position: self.dest_position,
            indices: self.indices,
        }
    }
}

impl<S> Debug for DynamicListChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicListChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_position", &self.source_position)
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.slot.variable_name)
            .finish()
    }
}

impl<S> DynamicListChangeMove<S> {
    pub fn new(
        slot: DynamicListVariableSlot<S>,
        source_entity_index: usize,
        source_position: usize,
        dest_entity_index: usize,
        dest_position: usize,
    ) -> Self {
        Self {
            slot,
            source_entity_index,
            source_position,
            dest_entity_index,
            dest_position,
            indices: [source_entity_index, dest_entity_index],
        }
    }

    fn is_intra_list(&self) -> bool {
        self.source_entity_index == self.dest_entity_index
    }

    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    pub fn source_position(&self) -> usize {
        self.source_position
    }

    pub fn dest_entity_index(&self) -> usize {
        self.dest_entity_index
    }

    pub fn dest_position(&self) -> usize {
        self.dest_position
    }

    fn coordinates(&self) -> ChangeCoordinates {
        ChangeCoordinates {
            source_entity: self.source_entity_index,
            source_position: self.source_position,
            destination_entity: self.dest_entity_index,
            destination_position: self.dest_position,
        }
    }
}

impl<S> Move<S> for DynamicListChangeMove<S>
where
    S: PlanningSolution,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        change_is_doable(&self.slot, self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        change_do_move(
            &self.slot,
            self.coordinates(),
            ChangeValueTransfer::MoveIntoInsert,
            score_director,
        );
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        change_undo_move(&self.slot, self.coordinates(), score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        if self.is_intra_list() {
            &self.indices[0..1]
        } else {
            &self.indices
        }
    }

    fn variable_name(&self) -> &str {
        self.slot.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "dynamic_list_change"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        change_tabu_signature(&self.slot, self.coordinates(), score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(change_candidate_trace_identity(
            &self.slot,
            self.coordinates(),
        ))
    }
}
