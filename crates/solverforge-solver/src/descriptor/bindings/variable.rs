use std::fmt::{self, Debug};
use std::ops::Deref;

use solverforge_core::domain::ValueRangeType;

use crate::phase::construction::ConstructionSlotId;

#[derive(Clone)]
pub(crate) struct VariableBinding {
    pub(crate) binding_index: usize,
    pub(crate) descriptor_index: usize,
    pub(crate) variable_index: usize,
    pub(crate) entity_type_name: &'static str,
    pub(crate) variable_name: &'static str,
    pub(crate) allows_unassigned: bool,
    pub(crate) range_type: ValueRangeType,
}

impl Debug for VariableBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableBinding")
            .field("binding_index", &self.binding_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("allows_unassigned", &self.allows_unassigned)
            .field("range_type", &self.range_type)
            .finish()
    }
}

impl VariableBinding {
    pub(crate) fn slot_id(&self, entity_index: usize) -> ConstructionSlotId {
        ConstructionSlotId::new(self.binding_index, entity_index)
    }
}

pub(crate) struct ResolvedVariableBinding<S> {
    binding: VariableBinding,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S> Clone for ResolvedVariableBinding<S> {
    fn clone(&self) -> Self {
        Self {
            binding: self.binding.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S> Debug for ResolvedVariableBinding<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedVariableBinding")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> Deref for ResolvedVariableBinding<S> {
    type Target = VariableBinding;

    fn deref(&self) -> &Self::Target {
        &self.binding
    }
}

impl<S> ResolvedVariableBinding<S> {
    pub(crate) fn new(binding: VariableBinding) -> Self {
        Self {
            binding,
            _phantom: std::marker::PhantomData,
        }
    }
}
