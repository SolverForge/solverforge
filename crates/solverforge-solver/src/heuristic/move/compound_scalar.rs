use std::fmt::{self, Debug};

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_scoring::Director;

use crate::stats::{CandidateTraceCoordinate, CandidateTraceIdentity};

use super::metadata::{
    encode_option_usize, encode_usize, hash_str, MoveTabuScope, MoveTabuSignature,
};
use super::{Move, MoveAffectedEntity};

pub const COMPOUND_SCALAR_VARIABLE: &str = "compound_scalar";

pub type ScalarEditLegality<S> = fn(&S, usize, usize, Option<usize>) -> bool;

enum CompoundScalarEditAccess<S> {
    Static {
        getter: fn(&S, usize, usize) -> Option<usize>,
        setter: fn(&mut S, usize, usize, Option<usize>),
        value_is_legal: Option<ScalarEditLegality<S>>,
    },
    Dynamic(DynamicScalarVariableSlot<S>),
}

impl<S> Clone for CompoundScalarEditAccess<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Static {
                getter,
                setter,
                value_is_legal,
            } => Self::Static {
                getter: *getter,
                setter: *setter,
                value_is_legal: *value_is_legal,
            },
            Self::Dynamic(slot) => Self::Dynamic(slot.clone()),
        }
    }
}

/// One edit in a grouped scalar move.
///
/// The static form keeps the existing direct function pointers.  The dynamic
/// form carries the already-bound dynamic slot, so move execution never needs
/// a thread-local group name or a schema lookup.
pub struct CompoundScalarEdit<S> {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_index: usize,
    pub variable_name: &'static str,
    pub to_value: Option<usize>,
    access: CompoundScalarEditAccess<S>,
}

impl<S> Clone for CompoundScalarEdit<S> {
    fn clone(&self) -> Self {
        Self {
            descriptor_index: self.descriptor_index,
            entity_index: self.entity_index,
            variable_index: self.variable_index,
            variable_name: self.variable_name,
            to_value: self.to_value,
            access: self.access.clone(),
        }
    }
}

impl<S> CompoundScalarEdit<S> {
    #[allow(clippy::too_many_arguments)]
    pub fn static_edit(
        descriptor_index: usize,
        entity_index: usize,
        variable_index: usize,
        variable_name: &'static str,
        to_value: Option<usize>,
        getter: fn(&S, usize, usize) -> Option<usize>,
        setter: fn(&mut S, usize, usize, Option<usize>),
        value_is_legal: Option<ScalarEditLegality<S>>,
    ) -> Self {
        Self {
            descriptor_index,
            entity_index,
            variable_index,
            variable_name,
            to_value,
            access: CompoundScalarEditAccess::Static {
                getter,
                setter,
                value_is_legal,
            },
        }
    }

    pub fn dynamic_edit(
        descriptor_index: usize,
        entity_index: usize,
        variable_index: usize,
        variable_name: &'static str,
        to_value: Option<usize>,
        slot: DynamicScalarVariableSlot<S>,
    ) -> Self {
        debug_assert_eq!(descriptor_index, slot.descriptor_index());
        debug_assert_eq!(variable_index, slot.descriptor_variable_index());
        debug_assert_eq!(variable_name, slot.variable_name);
        Self {
            descriptor_index,
            entity_index,
            variable_index,
            variable_name,
            to_value,
            access: CompoundScalarEditAccess::Dynamic(slot),
        }
    }

    pub fn with_value_is_legal(mut self, value_is_legal: ScalarEditLegality<S>) -> Self {
        if let CompoundScalarEditAccess::Static {
            value_is_legal: legality,
            ..
        } = &mut self.access
        {
            *legality = Some(value_is_legal);
        }
        self
    }
}

impl<S> CompoundScalarEdit<S> {
    pub(crate) fn current_value(&self, solution: &S) -> Option<usize> {
        match &self.access {
            CompoundScalarEditAccess::Static { getter, .. } => {
                getter(solution, self.entity_index, self.variable_index)
            }
            CompoundScalarEditAccess::Dynamic(slot) => {
                slot.current_value(solution, self.entity_index)
            }
        }
    }

    pub(crate) fn set_value(&self, solution: &mut S, value: Option<usize>) {
        match &self.access {
            CompoundScalarEditAccess::Static { setter, .. } => {
                setter(solution, self.entity_index, self.variable_index, value)
            }
            CompoundScalarEditAccess::Dynamic(slot) => {
                slot.set_value(solution, self.entity_index, value)
            }
        }
    }

    pub(crate) fn value_is_legal(&self, solution: &S) -> bool {
        match &self.access {
            CompoundScalarEditAccess::Static {
                value_is_legal: Some(legality),
                ..
            } => legality(
                solution,
                self.entity_index,
                self.variable_index,
                self.to_value,
            ),
            CompoundScalarEditAccess::Static {
                value_is_legal: None,
                ..
            } => true,
            CompoundScalarEditAccess::Dynamic(slot) => {
                slot.value_is_legal(solution, self.entity_index, self.to_value)
            }
        }
    }
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
    construction_value_order_key: Option<i64>,
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
            construction_value_order_key: None,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
    }

    pub(crate) fn with_construction_value_order_key(mut self, order_key: Option<i64>) -> Self {
        self.construction_value_order_key = order_key;
        self
    }

    pub(crate) fn construction_value_order_key(&self) -> Option<i64> {
        self.construction_value_order_key
    }

    pub fn edits(&self) -> &[CompoundScalarEdit<S>] {
        &self.edits
    }

    pub fn reason(&self) -> &'static str {
        self.reason
    }

    pub(crate) fn is_doable_on(&self, solution: &S) -> bool
    where
        S: PlanningSolution,
    {
        if self.edits.is_empty() {
            return false;
        }

        let mut changes_value = false;
        for edit in &self.edits {
            if !edit.value_is_legal(solution) {
                return false;
            }
            changes_value |= edit.current_value(solution) != edit.to_value;
        }
        changes_value
    }
}

impl<S> Debug for CompoundScalarMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompoundScalarMove")
            .field("reason", &self.reason)
            .field("variable_label", &self.variable_label)
            .field("edits", &self.edits)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .field(
                "construction_value_order_key",
                &self.construction_value_order_key,
            )
            .finish()
    }
}

impl<S> Move<S> for CompoundScalarMove<S>
where
    S: PlanningSolution,
{
    type Undo = Vec<Option<usize>>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.is_doable_on(score_director.working_solution())
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let mut undo = Vec::with_capacity(self.edits.len());
        let affected = unique_affected_entities(&self.edits);
        for edit in &self.edits {
            let old_value = edit.current_value(score_director.working_solution());
            undo.push(old_value);
        }
        for (descriptor_index, entity_index) in &affected {
            score_director.before_variable_changed(*descriptor_index, *entity_index);
        }
        for edit in &self.edits {
            edit.set_value(score_director.working_solution_mut(), edit.to_value);
        }
        for (descriptor_index, entity_index) in affected.iter().rev() {
            score_director.after_variable_changed(*descriptor_index, *entity_index);
        }
        undo
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        let affected = unique_affected_entities(&self.edits);
        for (descriptor_index, entity_index) in &affected {
            score_director.before_variable_changed(*descriptor_index, *entity_index);
        }
        for (edit, old_value) in self.edits.iter().zip(undo) {
            edit.set_value(score_director.working_solution_mut(), old_value);
        }
        for (descriptor_index, entity_index) in affected.iter().rev() {
            score_director.after_variable_changed(*descriptor_index, *entity_index);
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

    fn telemetry_label(&self) -> &'static str {
        self.reason
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
            let current = edit.current_value(score_director.working_solution());
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

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        let children = self.edits.iter().map(|edit| {
            CandidateTraceIdentity::logical_move(
                edit.descriptor_index,
                edit.variable_name,
                "scalar_change",
                vec![
                    CandidateTraceCoordinate::from(edit.entity_index),
                    CandidateTraceCoordinate::from(edit.to_value),
                ],
            )
        });
        Some(CandidateTraceIdentity::composite(self.reason, children))
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

fn unique_affected_entities<S>(edits: &[CompoundScalarEdit<S>]) -> Vec<(usize, usize)> {
    let mut affected = Vec::new();
    for edit in edits {
        let entity = (edit.descriptor_index, edit.entity_index);
        if !affected.contains(&entity) {
            affected.push(entity);
        }
    }
    affected
}
