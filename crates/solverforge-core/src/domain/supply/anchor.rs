//! Anchor variable supply for O(1) "what's the chain root?" lookups.
//!
//! # Zero-Erasure Design
//!
//! - **Index-based**: Stores `entity_idx -> anchor_idx` mappings
//! - **Owned**: No `Arc`, `RwLock`, or interior mutability - uses `&mut self`
//! - **Generic**: Full type information preserved, no `dyn Any`

use std::collections::HashMap;

/// Index-based anchor variable supply.
///
/// For chained variables, entities form chains rooted at anchors.
/// This supply answers: "Given an entity index, what anchor index is at the chain root?"
///
/// Both entities and anchors are referenced by index into their respective collections.
///
/// # Example
///
/// ```
/// use solverforge_core::domain::supply::AnchorSupply;
///
/// let mut supply = AnchorSupply::new();
///
/// // Entities 0, 1, 2 all belong to anchor 0
/// supply.set(0, 0);
/// supply.set(1, 0);
/// supply.set(2, 0);
///
/// // Entity 3 belongs to anchor 1
/// supply.set(3, 1);
///
/// assert_eq!(supply.get(0), Some(0));
/// assert_eq!(supply.get(1), Some(0));
/// assert_eq!(supply.get(3), Some(1));
/// ```
#[derive(Debug, Default)]
pub struct AnchorSupply {
    /// Mapping from entity index to anchor index.
    anchor_map: HashMap<usize, usize>,
}

impl AnchorSupply {
    /// Creates a new empty anchor supply.
    pub fn new() -> Self {
        Self {
            anchor_map: HashMap::new(),
        }
    }

    /// Creates a new anchor supply with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            anchor_map: HashMap::with_capacity(capacity),
        }
    }

    /// Gets the anchor index for an entity index.
    ///
    /// Returns `None` if the entity is not in any chain.
    #[inline]
    pub fn get(&self, entity_idx: usize) -> Option<usize> {
        self.anchor_map.get(&entity_idx).copied()
    }

    /// Sets the anchor index for an entity index.
    ///
    /// Returns the previous anchor index if one existed.
    #[inline]
    pub fn set(&mut self, entity_idx: usize, anchor_idx: usize) -> Option<usize> {
        self.anchor_map.insert(entity_idx, anchor_idx)
    }

    /// Removes the anchor mapping for an entity.
    ///
    /// Returns the anchor index that was mapped, if any.
    #[inline]
    pub fn remove(&mut self, entity_idx: usize) -> Option<usize> {
        self.anchor_map.remove(&entity_idx)
    }

    /// Updates anchors for multiple entities at once.
    ///
    /// Use when re-rooting a chain segment to a new anchor.
    #[inline]
    pub fn cascade(&mut self, entity_indices: impl IntoIterator<Item = usize>, anchor_idx: usize) {
        for entity_idx in entity_indices {
            self.anchor_map.insert(entity_idx, anchor_idx);
        }
    }

    /// Clears all mappings.
    #[inline]
    pub fn clear(&mut self) {
        self.anchor_map.clear();
    }

    /// Returns the number of tracked entities.
    #[inline]
    pub fn len(&self) -> usize {
        self.anchor_map.len()
    }

    /// Returns true if no entities are tracked.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.anchor_map.is_empty()
    }

    /// Returns an iterator over all (entity_idx, anchor_idx) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &usize)> {
        self.anchor_map.iter()
    }

    /// Returns all entity indices that share the given anchor.
    pub fn entities_for_anchor(&self, anchor_idx: usize) -> Vec<usize> {
        self.anchor_map
            .iter()
            .filter(|(_, &a)| a == anchor_idx)
            .map(|(&e, _)| e)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        supply.set(1, 10);
        supply.set(2, 20);

        assert_eq!(supply.get(0), Some(10));
        assert_eq!(supply.get(1), Some(10));
        assert_eq!(supply.get(2), Some(20));
        assert_eq!(supply.get(99), None);
    }

    #[test]
    fn test_remove() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(0);
        assert_eq!(removed, Some(10));
        assert!(supply.is_empty());
    }

    #[test]
    fn test_cascade() {
        let mut supply = AnchorSupply::new();

        supply.cascade([0, 1, 2, 3], 5);

        assert_eq!(supply.get(0), Some(5));
        assert_eq!(supply.get(1), Some(5));
        assert_eq!(supply.get(2), Some(5));
        assert_eq!(supply.get(3), Some(5));
        assert_eq!(supply.len(), 4);
    }

    #[test]
    fn test_update_anchor() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        assert_eq!(supply.get(0), Some(10));

        supply.set(0, 20);
        assert_eq!(supply.get(0), Some(20));
        assert_eq!(supply.len(), 1);
    }

    #[test]
    fn test_entities_for_anchor() {
        let mut supply = AnchorSupply::new();

        supply.set(0, 10);
        supply.set(1, 10);
        supply.set(2, 20);
        supply.set(3, 10);

        let mut entities = supply.entities_for_anchor(10);
        entities.sort();

        assert_eq!(entities, vec![0, 1, 3]);
    }

    #[test]
    fn test_clear() {
        let mut supply = AnchorSupply::new();

        supply.cascade(0..10, 0);
        assert_eq!(supply.len(), 10);

        supply.clear();
        assert!(supply.is_empty());
    }
}
