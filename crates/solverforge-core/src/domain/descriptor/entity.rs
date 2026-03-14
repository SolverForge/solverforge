// Entity descriptor.

use std::any::{Any, TypeId};
use std::fmt;

use super::VariableDescriptor;
use crate::domain::entity_ref::{EntityExtractor, EntityRef};

/// Describes a planning entity type at runtime.
pub struct EntityDescriptor {
    // Name of the entity type.
    pub type_name: &'static str,
    // TypeId of the entity type.
    pub type_id: TypeId,
    // Field name in the solution (for entity collections).
    pub solution_field: &'static str,
    // Whether this is a collection of entities.
    pub is_collection: bool,
    // Variable descriptors for this entity (metadata only).
    pub variable_descriptors: Vec<VariableDescriptor>,
    // Extractor for getting entities from a solution.
    pub extractor: Option<Box<dyn EntityExtractor>>,
    // The ID field name, if any.
    pub id_field: Option<&'static str>,
    // The pinning field name, if any.
    pub pin_field: Option<&'static str>,
}

impl EntityDescriptor {
    pub fn new(type_name: &'static str, type_id: TypeId, solution_field: &'static str) -> Self {
        EntityDescriptor {
            type_name,
            type_id,
            solution_field,
            is_collection: true,
            variable_descriptors: Vec::new(),
            extractor: None,
            id_field: None,
            pin_field: None,
        }
    }

    pub fn with_extractor(mut self, extractor: Box<dyn EntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    pub fn with_variable(mut self, descriptor: VariableDescriptor) -> Self {
        self.variable_descriptors.push(descriptor);
        self
    }

    pub fn with_id_field(mut self, field: &'static str) -> Self {
        self.id_field = Some(field);
        self
    }

    pub fn with_pin_field(mut self, field: &'static str) -> Self {
        self.pin_field = Some(field);
        self
    }

    pub fn genuine_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_genuine())
    }

    pub fn shadow_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_shadow())
    }

    pub fn find_variable(&self, name: &str) -> Option<&VariableDescriptor> {
        self.variable_descriptors.iter().find(|v| v.name == name)
    }

    pub fn has_genuine_variables(&self) -> bool {
        self.variable_descriptors
            .iter()
            .any(|v| v.variable_type.is_genuine())
    }

    pub fn has_extractor(&self) -> bool {
        self.extractor.is_some()
    }

    pub fn entity_count(&self, solution: &dyn Any) -> Option<usize> {
        self.extractor.as_ref()?.count(solution)
    }

    pub fn get_entity<'a>(&self, solution: &'a dyn Any, index: usize) -> Option<&'a dyn Any> {
        self.extractor.as_ref()?.get(solution, index)
    }

    pub fn get_entity_mut<'a>(
        &self,
        solution: &'a mut dyn Any,
        index: usize,
    ) -> Option<&'a mut dyn Any> {
        self.extractor.as_ref()?.get_mut(solution, index)
    }

    pub fn entity_refs(&self, solution: &dyn Any) -> Vec<EntityRef> {
        self.extractor
            .as_ref()
            .map(|e| e.entity_refs(solution))
            .unwrap_or_default()
    }

    pub fn for_each_entity<F>(&self, solution: &dyn Any, mut f: F) -> Option<()>
    where
        F: FnMut(usize, &dyn Any),
    {
        let extractor = self.extractor.as_ref()?;
        let count = extractor.count(solution)?;
        for i in 0..count {
            if let Some(entity) = extractor.get(solution, i) {
                f(i, entity);
            }
        }
        Some(())
    }

    pub fn for_each_entity_mut<F>(&self, solution: &mut dyn Any, mut f: F) -> Option<()>
    where
        F: FnMut(usize, &mut dyn Any),
    {
        let extractor = self.extractor.as_ref()?;
        let count = extractor.count(solution)?;
        for i in 0..count {
            if let Some(entity) = extractor.get_mut(solution, i) {
                f(i, entity);
            }
        }
        Some(())
    }
}

impl Clone for EntityDescriptor {
    fn clone(&self) -> Self {
        Self {
            type_name: self.type_name,
            type_id: self.type_id,
            solution_field: self.solution_field,
            is_collection: self.is_collection,
            variable_descriptors: self.variable_descriptors.clone(),
            extractor: self.extractor.clone(),
            id_field: self.id_field,
            pin_field: self.pin_field,
        }
    }
}

impl fmt::Debug for EntityDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntityDescriptor")
            .field("type_name", &self.type_name)
            .field("solution_field", &self.solution_field)
            .field("variables", &self.variable_descriptors.len())
            .finish()
    }
}
