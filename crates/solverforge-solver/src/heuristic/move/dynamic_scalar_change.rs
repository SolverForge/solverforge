use std::fmt::Debug;

use smallvec::smallvec;
use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use super::metadata::{encode_option_debug, encode_usize, hash_str, MoveTabuScope};
use super::{Move, MoveTabuSignature};

pub struct DynamicScalarChangeMove<S> {
    slot: DynamicScalarVariableSlot<S>,
    entity_index: usize,
    to_value: Option<usize>,
}

impl<S> Clone for DynamicScalarChangeMove<S> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            entity_index: self.entity_index,
            to_value: self.to_value,
        }
    }
}

impl<S> Debug for DynamicScalarChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicScalarChangeMove")
            .field("entity_index", &self.entity_index)
            .field("descriptor_index", &self.slot.descriptor_index())
            .field("variable", &self.slot.variable)
            .field("variable_name", &self.slot.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S> DynamicScalarChangeMove<S> {
    pub fn new(
        slot: DynamicScalarVariableSlot<S>,
        entity_index: usize,
        to_value: Option<usize>,
    ) -> Self {
        Self {
            slot,
            entity_index,
            to_value,
        }
    }

    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub fn to_value(&self) -> Option<usize> {
        self.to_value
    }
}

impl<S> Move<S> for DynamicScalarChangeMove<S>
where
    S: PlanningSolution,
{
    type Undo = Option<usize>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if !self.slot.value_is_legal(
            score_director.working_solution(),
            self.entity_index,
            self.to_value,
        ) {
            return false;
        }
        self.slot
            .current_value(score_director.working_solution(), self.entity_index)
            != self.to_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let old_value = self
            .slot
            .current_value(score_director.working_solution(), self.entity_index);
        let descriptor_index = self.slot.descriptor_index();

        score_director.before_variable_changed(descriptor_index, self.entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.entity_index,
            self.to_value,
        );
        score_director.after_variable_changed(descriptor_index, self.entity_index);

        old_value
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        let descriptor_index = self.slot.descriptor_index();
        score_director.before_variable_changed(descriptor_index, self.entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.entity_index,
            undo,
        );
        score_director.after_variable_changed(descriptor_index, self.entity_index);
    }

    fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.slot.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "dynamic_scalar_change"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let current = self
            .slot
            .current_value(score_director.working_solution(), self.entity_index);
        let from_id = encode_option_debug(current.as_ref());
        let to_id = encode_option_debug(self.to_value.as_ref());
        let entity_id = encode_usize(self.entity_index);
        let variable_id = hash_str(self.slot.variable_name);
        let scope = MoveTabuScope::new(self.slot.descriptor_index(), self.slot.variable_name);

        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable_id,
                entity_id,
                from_id,
                to_id
            ],
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable_id,
                entity_id,
                to_id,
                from_id
            ],
        )
        .with_entity_tokens([scope.entity_token(entity_id)])
        .with_destination_value_tokens([scope.value_token(to_id)])
    }
}
