//! Entity reference types for identifying entities by index.

/// A reference to a planning entity with its index in the solution.
///
/// This struct provides a way to identify entities during solving
/// without needing to know the concrete entity type.
#[derive(Debug, Clone)]
pub struct EntityRef {
    /// Index of this entity in its collection.
    pub index: usize,
    /// Name of the entity type.
    pub type_name: &'static str,
    /// Name of the collection field in the solution.
    pub collection_field: &'static str,
}

impl EntityRef {
    /// Creates a new entity reference.
    pub fn new(index: usize, type_name: &'static str, collection_field: &'static str) -> Self {
        Self {
            index,
            type_name,
            collection_field,
        }
    }
}
