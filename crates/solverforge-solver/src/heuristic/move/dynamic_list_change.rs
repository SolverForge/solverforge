use std::fmt::Debug;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken,
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

    fn adjusted_dest(&self) -> usize {
        if self.is_intra_list() && self.dest_position > self.source_position {
            self.dest_position - 1
        } else {
            self.dest_position
        }
    }
}

impl<S> Move<S> for DynamicListChangeMove<S>
where
    S: PlanningSolution,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        let source_len = self.slot.list_len(solution, self.source_entity_index);
        if self.source_position >= source_len {
            return false;
        }

        let dest_len = self.slot.list_len(solution, self.dest_entity_index);
        let max_dest = if self.is_intra_list() {
            source_len
        } else {
            dest_len
        };
        if self.dest_position > max_dest {
            return false;
        }

        if self.is_intra_list() {
            if self.source_position == self.dest_position {
                return false;
            }
            if self.dest_position == self.source_position + 1 {
                return false;
            }
        }

        true
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let descriptor_index = self.slot.descriptor_index();
        score_director.before_variable_changed(descriptor_index, self.source_entity_index);
        if !self.is_intra_list() {
            score_director.before_variable_changed(descriptor_index, self.dest_entity_index);
        }

        let value = self
            .slot
            .list_remove(
                score_director.working_solution_mut(),
                self.source_entity_index,
                self.source_position,
            )
            .expect("source position should be valid");
        self.slot.list_insert(
            score_director.working_solution_mut(),
            self.dest_entity_index,
            self.adjusted_dest(),
            value,
        );

        score_director.after_variable_changed(descriptor_index, self.source_entity_index);
        if !self.is_intra_list() {
            score_director.after_variable_changed(descriptor_index, self.dest_entity_index);
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        let descriptor_index = self.slot.descriptor_index();
        score_director.before_variable_changed(descriptor_index, self.dest_entity_index);
        if !self.is_intra_list() {
            score_director.before_variable_changed(descriptor_index, self.source_entity_index);
        }

        let removed = self
            .slot
            .list_remove(
                score_director.working_solution_mut(),
                self.dest_entity_index,
                self.adjusted_dest(),
            )
            .expect("undo destination position should contain moved element");
        self.slot.list_insert(
            score_director.working_solution_mut(),
            self.source_entity_index,
            self.source_position,
            removed,
        );

        score_director.after_variable_changed(descriptor_index, self.dest_entity_index);
        if !self.is_intra_list() {
            score_director.after_variable_changed(descriptor_index, self.source_entity_index);
        }
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
        let moved_value = self.slot.list_get(
            score_director.working_solution(),
            self.source_entity_index,
            self.source_position,
        );
        let moved_id = encode_option_debug(moved_value.as_ref());
        let source_entity_id = encode_usize(self.source_entity_index);
        let dest_entity_id = encode_usize(self.dest_entity_index);
        let variable_id = hash_str(self.slot.variable_name);
        let scope = MoveTabuScope::new(self.slot.descriptor_index(), self.slot.variable_name);
        let adjusted_dest = self.adjusted_dest();
        let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
            smallvec![scope.entity_token(source_entity_id)];
        if !self.is_intra_list() {
            entity_tokens.push(scope.entity_token(dest_entity_id));
        }

        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable_id,
                source_entity_id,
                encode_usize(self.source_position),
                dest_entity_id,
                encode_usize(adjusted_dest),
                moved_id
            ],
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable_id,
                dest_entity_id,
                encode_usize(adjusted_dest),
                source_entity_id,
                encode_usize(self.source_position),
                moved_id
            ],
        )
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens([scope.value_token(moved_id)])
    }
}
