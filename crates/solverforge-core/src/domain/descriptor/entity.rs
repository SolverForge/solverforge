//! Entity descriptor.

use std::any::TypeId;
use std::fmt;

use super::VariableDescriptor;

/// Describes a planning entity type at runtime.
///
/// This is metadata-only - entity access is done through generated methods
/// on the solution type itself (zero-erasure architecture).
pub struct EntityDescriptor {
    /// Name of the entity type.
    pub type_name: &'static str,
    /// TypeId of the entity type.
    pub type_id: TypeId,
    /// Field name in the solution (for entity collections).
    pub solution_field: &'static str,
    /// Whether this is a collection of entities.
    pub is_collection: bool,
    /// Variable descriptors for this entity (metadata only).
    pub variable_descriptors: Vec<VariableDescriptor>,
    /// The ID field name, if any.
    pub id_field: Option<&'static str>,
    /// The pinning field name, if any.
    pub pin_field: Option<&'static str>,
}

impl EntityDescriptor {
    /// Creates a new EntityDescriptor.
    pub fn new(type_name: &'static str, type_id: TypeId, solution_field: &'static str) -> Self {
        EntityDescriptor {
            type_name,
            type_id,
            solution_field,
            is_collection: true,
            variable_descriptors: Vec::new(),
            id_field: None,
            pin_field: None,
        }
    }

    /// Adds a variable descriptor (metadata only).
    pub fn with_variable(mut self, descriptor: VariableDescriptor) -> Self {
        self.variable_descriptors.push(descriptor);
        self
    }

    /// Sets the ID field.
    pub fn with_id_field(mut self, field: &'static str) -> Self {
        self.id_field = Some(field);
        self
    }

    /// Sets the pin field.
    pub fn with_pin_field(mut self, field: &'static str) -> Self {
        self.pin_field = Some(field);
        self
    }

    /// Returns genuine (non-shadow) variable descriptors.
    pub fn genuine_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_genuine())
    }

    /// Returns shadow variable descriptors.
    pub fn shadow_variable_descriptors(&self) -> impl Iterator<Item = &VariableDescriptor> {
        self.variable_descriptors
            .iter()
            .filter(|v| v.variable_type.is_shadow())
    }

    /// Finds a variable descriptor by name.
    pub fn find_variable(&self, name: &str) -> Option<&VariableDescriptor> {
        self.variable_descriptors.iter().find(|v| v.name == name)
    }

    /// Returns true if this entity has any genuine variables.
    pub fn has_genuine_variables(&self) -> bool {
        self.variable_descriptors
            .iter()
            .any(|v| v.variable_type.is_genuine())
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
