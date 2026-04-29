use std::fmt::{self, Debug};

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_usize, encode_usize, hash_str, MoveTabuScope, MoveTabuSignature,
};
use super::{Move, MoveAffectedEntity};

pub const COMPOUND_SCALAR_VARIABLE: &str = "compound_scalar";

pub type ScalarEditLegality<S> = fn(&S, usize, usize, Option<usize>) -> bool;

#[derive(Clone)]
pub struct CompoundScalarEdit<S> {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_index: usize,
    pub variable_name: &'static str,
    pub to_value: Option<usize>,
    pub getter: fn(&S, usize, usize) -> Option<usize>,
    pub setter: fn(&mut S, usize, usize, Option<usize>),
    pub value_is_legal: Option<ScalarEditLegality<S>>,
}

impl<S> Debug for CompoundScalarEdit<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompoundScalarEdit")
            .field("descriptor_index", &self.descriptor_index)
            .field("entity_index", &self.entity_index)
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

#[derive(Clone)]
pub struct CompoundScalarMove<S> {
    reason: &'static str,
    variable_label: &'static str,
    edits: Vec<CompoundScalarEdit<S>>,
    entity_indices: Vec<usize>,
    require_hard_improvement: bool,
}

impl<S> CompoundScalarMove<S> {
    pub fn new(reason: &'static str, edits: Vec<CompoundScalarEdit<S>>) -> Self {
        Self::with_label(reason, COMPOUND_SCALAR_VARIABLE, edits)
    }

    pub fn with_label(
        reason: &'static str,
        variable_label: &'static str,
        edits: Vec<CompoundScalarEdit<S>>,
    ) -> Self {
        let mut entity_indices = edits
            .iter()
            .map(|edit| edit.entity_index)
            .collect::<Vec<_>>();
        entity_indices.sort_unstable();
        entity_indices.dedup();
        Self {
            reason,
            variable_label,
            edits,
            entity_indices,
            require_hard_improvement: false,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
    }

    pub fn edits(&self) -> &[CompoundScalarEdit<S>] {
        &self.edits
    }

    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

impl<S> Debug for CompoundScalarMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompoundScalarMove")
            .field("reason", &self.reason)
            .field("variable_label", &self.variable_label)
            .field("edits", &self.edits)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

impl<S> Move<S> for CompoundScalarMove<S>
where
    S: PlanningSolution,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.edits.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let mut changes_value = false;
        for edit in &self.edits {
            if let Some(value_is_legal) = edit.value_is_legal {
                if !value_is_legal(
                    solution,
                    edit.entity_index,
                    edit.variable_index,
                    edit.to_value,
                ) {
                    return false;
                }
            }
            let current = (edit.getter)(solution, edit.entity_index, edit.variable_index);
            changes_value |= current != edit.to_value;
        }

        changes_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        for edit in &self.edits {
            let old_value = (edit.getter)(
                score_director.working_solution(),
                edit.entity_index,
                edit.variable_index,
            );
            score_director.before_variable_changed(edit.descriptor_index, edit.entity_index);
            (edit.setter)(
                score_director.working_solution_mut(),
                edit.entity_index,
                edit.variable_index,
                edit.to_value,
            );
            score_director.after_variable_changed(edit.descriptor_index, edit.entity_index);

            let setter = edit.setter;
            let entity_index = edit.entity_index;
            let variable_index = edit.variable_index;
            score_director.register_undo(Box::new(move |solution: &mut S| {
                setter(solution, entity_index, variable_index, old_value);
            }));
        }
    }

    fn descriptor_index(&self) -> usize {
        self.edits
            .first()
            .map(|edit| edit.descriptor_index)
            .unwrap_or(usize::MAX)
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_label
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard_improvement
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let scope = MoveTabuScope::new(self.descriptor_index(), self.variable_label);
        let mut move_id = smallvec![hash_str(self.reason)];
        let mut undo_move_id = smallvec![hash_str(self.reason)];
        let mut entity_tokens: SmallVec<[_; 8]> = SmallVec::new();
        let mut destination_tokens: SmallVec<[_; 8]> = SmallVec::new();

        for edit in &self.edits {
            let current = (edit.getter)(
                score_director.working_solution(),
                edit.entity_index,
                edit.variable_index,
            );
            let descriptor = encode_usize(edit.descriptor_index);
            let entity = encode_usize(edit.entity_index);
            let variable = hash_str(edit.variable_name);
            let from = encode_option_usize(current);
            let to = encode_option_usize(edit.to_value);
            let edit_scope = MoveTabuScope::new(edit.descriptor_index, edit.variable_name);

            move_id.extend([descriptor, entity, variable, from, to]);
            undo_move_id.extend([descriptor, entity, variable, to, from]);
            entity_tokens.push(edit_scope.entity_token(entity));
            destination_tokens.push(edit_scope.value_token(to));
        }

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens(destination_tokens)
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        for edit in &self.edits {
            visitor(MoveAffectedEntity {
                descriptor_index: edit.descriptor_index,
                entity_index: edit.entity_index,
                variable_name: edit.variable_name,
            });
        }
    }
}
