use std::fmt::Debug;

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::metadata::{
    encode_option_debug, encode_usize, ordered_coordinate_pair, scoped_move_identity,
    MoveTabuScope, TABU_OP_SWAP,
};
use super::{Move, MoveTabuSignature};

/// Exchanges two values in one descriptor-resolved dynamic scalar variable.
pub struct DynamicScalarSwapMove<S> {
    slot: DynamicScalarVariableSlot<S>,
    left_entity_index: usize,
    right_entity_index: usize,
    indices: [usize; 2],
}

impl<S> Clone for DynamicScalarSwapMove<S> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            left_entity_index: self.left_entity_index,
            right_entity_index: self.right_entity_index,
            indices: self.indices,
        }
    }
}

impl<S> Debug for DynamicScalarSwapMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicScalarSwapMove")
            .field("left_entity_index", &self.left_entity_index)
            .field("right_entity_index", &self.right_entity_index)
            .field("descriptor_index", &self.slot.descriptor_index())
            .field("variable", &self.slot.variable)
            .field("variable_name", &self.slot.variable_name)
            .finish()
    }
}

impl<S> DynamicScalarSwapMove<S> {
    pub fn new(
        slot: DynamicScalarVariableSlot<S>,
        left_entity_index: usize,
        right_entity_index: usize,
    ) -> Self {
        Self {
            slot,
            left_entity_index,
            right_entity_index,
            indices: [left_entity_index, right_entity_index],
        }
    }

    pub fn left_entity_index(&self) -> usize {
        self.left_entity_index
    }

    pub fn right_entity_index(&self) -> usize {
        self.right_entity_index
    }
}

impl<S> Move<S> for DynamicScalarSwapMove<S>
where
    S: PlanningSolution,
{
    type Undo = (Option<usize>, Option<usize>);

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.left_entity_index == self.right_entity_index {
            return false;
        }

        let solution = score_director.working_solution();
        let left_value = self.slot.current_value(solution, self.left_entity_index);
        let right_value = self.slot.current_value(solution, self.right_entity_index);
        left_value != right_value
            && self
                .slot
                .value_is_legal(solution, self.left_entity_index, right_value)
            && self
                .slot
                .value_is_legal(solution, self.right_entity_index, left_value)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let left_value = self
            .slot
            .current_value(score_director.working_solution(), self.left_entity_index);
        let right_value = self
            .slot
            .current_value(score_director.working_solution(), self.right_entity_index);
        let descriptor_index = self.slot.descriptor_index();

        score_director.before_variable_changed(descriptor_index, self.left_entity_index);
        score_director.before_variable_changed(descriptor_index, self.right_entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.left_entity_index,
            right_value,
        );
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.right_entity_index,
            left_value,
        );
        score_director.after_variable_changed(descriptor_index, self.left_entity_index);
        score_director.after_variable_changed(descriptor_index, self.right_entity_index);

        (left_value, right_value)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        let descriptor_index = self.slot.descriptor_index();
        score_director.before_variable_changed(descriptor_index, self.left_entity_index);
        score_director.before_variable_changed(descriptor_index, self.right_entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.left_entity_index,
            undo.0,
        );
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.right_entity_index,
            undo.1,
        );
        score_director.after_variable_changed(descriptor_index, self.left_entity_index);
        score_director.after_variable_changed(descriptor_index, self.right_entity_index);
    }

    fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
    }

    fn variable_name(&self) -> &str {
        self.slot.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "dynamic_scalar_swap"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let solution = score_director.working_solution();
        let left_value = self.slot.current_value(solution, self.left_entity_index);
        let right_value = self.slot.current_value(solution, self.right_entity_index);
        let left_id = encode_option_debug(left_value.as_ref());
        let right_id = encode_option_debug(right_value.as_ref());
        let left_entity_id = encode_usize(self.left_entity_index);
        let right_entity_id = encode_usize(self.right_entity_index);
        let scope = MoveTabuScope::new(self.slot.descriptor_index(), self.slot.variable_name);
        let entity_pair = ordered_coordinate_pair((left_entity_id, 0), (right_entity_id, 0));
        let move_id = scoped_move_identity(
            scope,
            TABU_OP_SWAP,
            entity_pair.into_iter().map(|(entity_id, _)| entity_id),
        );

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens([
                scope.entity_token(left_entity_id),
                scope.entity_token(right_entity_id),
            ])
            .with_destination_value_tokens([
                scope.value_token(right_id),
                scope.value_token(left_id),
            ])
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        Some(CandidateTraceIdentity::logical_move(
            self.slot.descriptor_index(),
            self.slot.variable_name,
            "scalar_swap",
            [self.left_entity_index, self.right_entity_index],
        ))
    }
}
