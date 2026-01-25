//! Problem fact descriptor.

use std::any::TypeId;
use std::fmt;

/// Describes a problem fact type at runtime.
///
/// This is metadata-only - fact access is done through generated methods
/// on the solution type itself (zero-erasure architecture).
pub struct ProblemFactDescriptor {
    /// Name of the problem fact type.
    pub type_name: &'static str,
    /// TypeId of the problem fact type.
    pub type_id: TypeId,
    /// Field name in the solution.
    pub solution_field: &'static str,
    /// Whether this is a collection of facts.
    pub is_collection: bool,
    /// The ID field name, if any (for value range provider lookups).
    pub id_field: Option<&'static str>,
}

impl ProblemFactDescriptor {
    /// Creates a new ProblemFactDescriptor.
    pub fn new(type_name: &'static str, type_id: TypeId, solution_field: &'static str) -> Self {
        ProblemFactDescriptor {
            type_name,
            type_id,
            solution_field,
            is_collection: true,
            id_field: None,
        }
    }

    /// Sets whether this is a single fact (not a collection).
    pub fn single(mut self) -> Self {
        self.is_collection = false;
        self
    }

    /// Sets the ID field.
    pub fn with_id_field(mut self, field: &'static str) -> Self {
        self.id_field = Some(field);
        self
    }
}

impl Clone for ProblemFactDescriptor {
    fn clone(&self) -> Self {
        Self {
            type_name: self.type_name,
            type_id: self.type_id,
            solution_field: self.solution_field,
            is_collection: self.is_collection,
            id_field: self.id_field,
        }
    }
}

impl fmt::Debug for ProblemFactDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProblemFactDescriptor")
            .field("type_name", &self.type_name)
            .field("solution_field", &self.solution_field)
            .field("is_collection", &self.is_collection)
            .finish()
    }
}
