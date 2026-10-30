//! Entity descriptor.

use std::any::{Any, TypeId};
use std::fmt;

use super::{EntityClassId, VariableDescriptor};
use crate::domain::entity_ref::{EntityExtractor, EntityRef};

/// Describes a planning entity type at runtime.
pub struct EntityDescriptor {
    /// Name of the entity type.
    pub type_name: &'static str,
    /// TypeId of the entity type.
    pub type_id: TypeId,
    /// Logical entity class ID for dynamic binding models.
    pub logical_id: Option<EntityClassId>,
    /// Field name in the solution (for entity collections).
    pub solution_field: &'static str,
    /// Whether this is a collection of entities.
    pub is_collection: bool,
    /// Variable descriptors for this entity (metadata only).
    pub variable_descriptors: Vec<VariableDescriptor>,
    /// Extractor for getting entities from a solution.
    pub extractor: Option<Box<dyn EntityExtractor>>,
    /// The ID field name, if any.
    pub id_field: Option<&'static str>,
    /// The pinning field name, if any.
    pub pin_field: Option<&'static str>,
    // Typed access to the entity's pin state, when pinning is configured.
    pin_predicate: Option<fn(&dyn Any) -> bool>,
}

impl EntityDescriptor {
    /// Creates a descriptor for a collection field `solution_field` holding
    /// entities of type `type_name`, with no variables, extractor, ID or
    /// pinning.
    pub fn new(type_name: &'static str, type_id: TypeId, solution_field: &'static str) -> Self {
        EntityDescriptor {
            type_name,
            type_id,
            logical_id: None,
            solution_field,
            is_collection: true,
            variable_descriptors: Vec::new(),
            extractor: None,
            id_field: None,
            pin_field: None,
            pin_predicate: None,
        }
    }

    /// Sets the extractor used to read entities out of a solution.
    pub fn with_extractor(mut self, extractor: Box<dyn EntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// Sets the logical entity class ID used by dynamic bindings.
    pub fn with_logical_id(mut self, logical_id: EntityClassId) -> Self {
        self.logical_id = Some(logical_id);
        self
    }

    /// Appends a variable descriptor.
    pub fn with_variable(mut self, descriptor: VariableDescriptor) -> Self {
        self.variable_descriptors.push(descriptor);
        self
    }

    /// Names the entity's planning ID field.
    pub fn with_id_field(mut self, field: &'static str) -> Self {
        self.id_field = Some(field);
        self
    }

    /// Names the entity's pinning field.
    pub fn with_pin_field(mut self, field: &'static str) -> Self {
        self.pin_field = Some(field);
        self
    }

    /// Supplies the entity-level pin predicate for a manually assembled descriptor.
    pub fn with_pin_predicate(mut self, predicate: fn(&dyn Any) -> bool) -> Self {
        self.pin_predicate = Some(predicate);
        self
    }

    /// Returns whether this entity is pinned. Descriptors without a predicate are unrestricted.
    pub fn is_pinned(&self, solution: &dyn Any, index: usize) -> bool {
        self.pin_predicate.is_some_and(|predicate| {
            predicate(
                self.get_entity(solution, index)
                    .expect("pinned entity lookup must match its descriptor"),
            )
        })
    }

    /// Returns `true` when a pin predicate is configured.
    pub fn has_pin_predicate(&self) -> bool {
        self.pin_predicate.is_some()
    }

    /// Iterates over the genuine (scalar and list) variables.
    pub fn genuine_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_genuine())
    }

    /// Iterates over the shadow variables.
    pub fn shadow_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_shadow())
    }

    /// Finds a variable by field name.
    pub fn find_variable(&self, name: &str) -> Option<&VariableDescriptor> {
        self.variable_descriptors.iter().find(|v| v.name == name)
    }

    /// Returns `true` when the entity declares at least one genuine variable.
    pub fn has_genuine_variables(&self) -> bool {
        self.variable_descriptors
            .iter()
            .any(|v| v.variable_type.is_genuine())
    }

    /// Returns `true` when an extractor is configured.
    pub fn has_extractor(&self) -> bool {
        self.extractor.is_some()
    }

    /// Number of entities of this type in `solution`; `None` without an
    /// extractor or when `solution` is not the expected type.
    pub fn entity_count(&self, solution: &dyn Any) -> Option<usize> {
        self.extractor.as_ref()?.count(solution)
    }

    /// Borrows the entity at `index`; `None` without an extractor, on a
    /// solution type mismatch, or when `index` is out of range.
    pub fn get_entity<'a>(&self, solution: &'a dyn Any, index: usize) -> Option<&'a dyn Any> {
        self.extractor.as_ref()?.get(solution, index)
    }

    /// Mutably borrows the entity at `index`; `None` without an extractor, on a
    /// solution type mismatch, or when `index` is out of range.
    pub fn get_entity_mut<'a>(
        &self,
        solution: &'a mut dyn Any,
        index: usize,
    ) -> Option<&'a mut dyn Any> {
        self.extractor.as_ref()?.get_mut(solution, index)
    }

    /// References to every entity of this type in `solution`; empty without an
    /// extractor or on a solution type mismatch.
    pub fn entity_refs(&self, solution: &dyn Any) -> Vec<EntityRef> {
        self.extractor
            .as_ref()
            .map(|e| e.entity_refs(solution))
            .unwrap_or_default()
    }

    /// Calls `f(index, entity)` for every entity; returns `None` without an
    /// extractor or on a solution type mismatch.
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

    /// Calls `f(index, entity)` with mutable access for every entity; returns
    /// `None` without an extractor or on a solution type mismatch.
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
            logical_id: self.logical_id,
            solution_field: self.solution_field,
            is_collection: self.is_collection,
            variable_descriptors: self.variable_descriptors.clone(),
            extractor: self.extractor.clone(),
            id_field: self.id_field,
            pin_field: self.pin_field,
            pin_predicate: self.pin_predicate,
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
