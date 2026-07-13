use std::fmt;

use solverforge_core::domain::{DynamicScalarVariableSlot, EntityClassId, VariableId};

use crate::heuristic::r#move::CompoundScalarEdit;

use super::super::variable::ScalarVariableSlot;

enum ScalarGroupMemberAccess<S> {
    Static(ScalarVariableSlot<S>),
    Dynamic(DynamicScalarVariableSlot<S>),
}

impl<S> Clone for ScalarGroupMemberAccess<S> {
    fn clone(&self) -> Self {
        match self {
            Self::Static(slot) => Self::Static(*slot),
            Self::Dynamic(slot) => Self::Dynamic(slot.clone()),
        }
    }
}

/// One scalar variable participating in a scalar group.
///
/// Typed members retain direct function pointers. Dynamic members retain the
/// existing dynamic slot adapter; both are consumed through this one semantic
/// surface by grouped construction and local search.
pub struct ScalarGroupMemberBinding<S> {
    pub descriptor_index: usize,
    pub variable_index: usize,
    pub entity_type_name: &'static str,
    pub variable_name: &'static str,
    pub allows_unassigned: bool,
    construction_binding_index: Option<usize>,
    access: ScalarGroupMemberAccess<S>,
}

impl<S> Clone for ScalarGroupMemberBinding<S> {
    fn clone(&self) -> Self {
        Self {
            descriptor_index: self.descriptor_index,
            variable_index: self.variable_index,
            entity_type_name: self.entity_type_name,
            variable_name: self.variable_name,
            allows_unassigned: self.allows_unassigned,
            construction_binding_index: self.construction_binding_index,
            access: self.access.clone(),
        }
    }
}

impl<S> ScalarGroupMemberBinding<S> {
    pub fn from_scalar_slot(slot: ScalarVariableSlot<S>) -> Self {
        Self {
            descriptor_index: slot.descriptor_index,
            variable_index: slot.variable_index,
            entity_type_name: slot.entity_type_name,
            variable_name: slot.variable_name,
            allows_unassigned: slot.allows_unassigned,
            construction_binding_index: None,
            access: ScalarGroupMemberAccess::Static(slot),
        }
    }

    pub fn from_dynamic_slot(slot: DynamicScalarVariableSlot<S>) -> Self {
        Self {
            // Dynamic group targets are canonicalized only by RuntimeModel
            // compilation. This avoids accepting a stale prebound descriptor
            // index before both entity and variable identity are validated.
            descriptor_index: usize::MAX,
            variable_index: usize::MAX,
            entity_type_name: slot.entity_type_name,
            variable_name: slot.variable_name,
            allows_unassigned: slot.allows_unassigned,
            construction_binding_index: None,
            access: ScalarGroupMemberAccess::Dynamic(slot),
        }
    }

    pub(crate) fn is_dynamic(&self) -> bool {
        matches!(self.access, ScalarGroupMemberAccess::Dynamic(_))
    }

    pub(crate) fn dynamic_identity(&self) -> Option<(EntityClassId, VariableId)> {
        match &self.access {
            ScalarGroupMemberAccess::Static(_) => None,
            ScalarGroupMemberAccess::Dynamic(slot) => Some((slot.entity, slot.variable)),
        }
    }

    pub(crate) fn canonicalize_dynamic_slot(
        &mut self,
        slot: DynamicScalarVariableSlot<S>,
    ) -> Result<(), String> {
        let Some((entity, variable)) = self.dynamic_identity() else {
            return Ok(());
        };
        if slot.entity != entity || slot.variable != variable {
            return Err(format!(
                "dynamic scalar group target {}.{} does not match the registered dynamic scalar slot {}.{}",
                self.entity_type_name,
                self.variable_name,
                slot.entity_type_name,
                slot.variable_name
            ));
        }
        // RuntimeModel resolves every registered dynamic slot before it
        // canonicalizes group members, so both descriptor indexes are now
        // available even when the original registration only prebound the
        // entity descriptor index.
        self.descriptor_index = slot.descriptor_index();
        self.variable_index = slot.descriptor_variable_index();
        self.entity_type_name = slot.entity_type_name;
        self.variable_name = slot.variable_name;
        self.allows_unassigned = slot.allows_unassigned;
        self.access = ScalarGroupMemberAccess::Dynamic(slot);
        Ok(())
    }

    pub(crate) fn set_construction_binding_index(&mut self, binding_index: usize) {
        self.construction_binding_index = Some(binding_index);
    }

    pub(crate) fn construction_binding_index(&self) -> usize {
        self.construction_binding_index.unwrap_or_else(|| {
            panic!(
                "scalar group member {}.{} was not bound to a construction slot",
                self.entity_type_name, self.variable_name
            )
        })
    }
}

impl<S> ScalarGroupMemberBinding<S> {
    pub fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => slot.current_value(solution, entity_index),
            ScalarGroupMemberAccess::Dynamic(slot) => slot.current_value(solution, entity_index),
        }
    }

    pub fn set_value(&self, solution: &mut S, entity_index: usize, value: Option<usize>) {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => slot.set_value(solution, entity_index, value),
            ScalarGroupMemberAccess::Dynamic(slot) => slot.set_value(solution, entity_index, value),
        }
    }

    pub fn value_is_legal(
        &self,
        solution: &S,
        entity_index: usize,
        candidate: Option<usize>,
    ) -> bool {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => {
                slot.value_is_legal(solution, entity_index, candidate)
            }
            ScalarGroupMemberAccess::Dynamic(slot) => {
                slot.value_is_legal(solution, entity_index, candidate)
            }
        }
    }

    pub fn entity_count(&self, solution: &S) -> usize {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => (slot.entity_count)(solution),
            ScalarGroupMemberAccess::Dynamic(slot) => slot.entity_count(solution),
        }
    }

    pub fn candidate_values(
        &self,
        solution: &S,
        entity_index: usize,
        value_candidate_limit: Option<usize>,
    ) -> Vec<usize> {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => {
                slot.candidate_values_for_entity(solution, entity_index, value_candidate_limit)
            }
            ScalarGroupMemberAccess::Dynamic(slot) => {
                let values = slot.candidate_values(solution, entity_index);
                match value_candidate_limit {
                    Some(limit) => values.iter().copied().take(limit).collect(),
                    None => values.to_vec(),
                }
            }
        }
    }

    pub(crate) fn compound_edit(
        &self,
        entity_index: usize,
        to_value: Option<usize>,
    ) -> CompoundScalarEdit<S> {
        match &self.access {
            ScalarGroupMemberAccess::Static(slot) => CompoundScalarEdit::static_edit(
                self.descriptor_index,
                entity_index,
                self.variable_index,
                self.variable_name,
                to_value,
                slot.getter,
                slot.setter,
                None,
            ),
            ScalarGroupMemberAccess::Dynamic(slot) => CompoundScalarEdit::dynamic_edit(
                self.descriptor_index,
                entity_index,
                self.variable_index,
                self.variable_name,
                to_value,
                slot.clone(),
            ),
        }
    }
}

impl<S> fmt::Debug for ScalarGroupMemberBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScalarGroupMemberBinding")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .field(
                "construction_binding_index",
                &self.construction_binding_index,
            )
            .field("dynamic", &self.is_dynamic())
            .finish()
    }
}
