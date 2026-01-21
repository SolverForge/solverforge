//! Inverse variable supply for O(1) "who points to this value?" lookups.
//!
//! # Zero-Erasure Design
//!
//! - **Index-based**: Stores `value -> entity_index` mappings, not cloned entities
//! - **Owned**: No `Arc`, `RwLock`, or interior mutability - uses `&mut self`
//! - **Generic**: Full type information preserved, no `dyn Any`

use std::collections::HashMap;
use std::hash::Hash;

/// Index-based inverse variable supply.
///
/// For a chained variable where `entity.previous = value`, this supply answers:
/// "Given `value`, which entity index has `entities[idx].previous == value`?"
///
/// Returns entity indices - caller accesses actual entity via `solution.entities[idx]`.
///
/// # Example
///
/// ```
/// use solverforge_core::domain::supply::InverseSupply;
///
/// let mut supply: InverseSupply<i32> = InverseSupply::new();
///
/// // Entity at index 0 points to value 42
/// supply.insert(42, 0);
///
/// // Entity at index 1 points to value 99
/// supply.insert(99, 1);
///
/// assert_eq!(supply.get(&42), Some(0));
/// assert_eq!(supply.get(&99), Some(1));
/// assert_eq!(supply.get(&100), None);
/// ```
#[derive(Debug)]
pub struct InverseSupply<V>
where
    V: Eq + Hash,
{
    /// Mapping from value to the entity index pointing to it.
    inverse_map: HashMap<V, usize>,
}

impl<V> Default for InverseSupply<V>
where
    V: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V> InverseSupply<V>
where
    V: Eq + Hash,
{
    /// Creates a new empty inverse supply.
    pub fn new() -> Self {
        Self {
            inverse_map: HashMap::new(),
        }
    }

    /// Creates a new inverse supply with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inverse_map: HashMap::with_capacity(capacity),
        }
    }

    /// Gets the entity index that points to the given value.
    ///
    /// Returns `None` if no entity currently points to this value.
    #[inline]
    pub fn get(&self, value: &V) -> Option<usize> {
        self.inverse_map.get(value).copied()
    }

    /// Registers that an entity index now points to a value.
    ///
    /// Returns the previous entity index if this value was already mapped.
    #[inline]
    pub fn insert(&mut self, value: V, entity_idx: usize) -> Option<usize> {
        self.inverse_map.insert(value, entity_idx)
    }

    /// Removes the mapping for a value.
    ///
    /// Returns the entity index that was mapped, if any.
    #[inline]
    pub fn remove(&mut self, value: &V) -> Option<usize> {
        self.inverse_map.remove(value)
    }

    /// Updates the mapping: removes old value, inserts new.
    ///
    /// Use this when an entity changes which value it points to.
    #[inline]
    pub fn update(&mut self, old_value: Option<&V>, new_value: V, entity_idx: usize) {
        if let Some(old) = old_value {
            self.remove(old);
        }
        self.insert(new_value, entity_idx);
    }

    /// Clears all mappings.
    #[inline]
    pub fn clear(&mut self) {
        self.inverse_map.clear();
    }

    /// Returns the number of tracked mappings.
    #[inline]
    pub fn len(&self) -> usize {
        self.inverse_map.len()
    }

    /// Returns true if no mappings exist.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inverse_map.is_empty()
    }

    /// Returns an iterator over all (value, entity_index) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&V, &usize)> {
        self.inverse_map.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.insert(2, 1);

        assert_eq!(supply.get(&1), Some(0));
        assert_eq!(supply.get(&2), Some(1));
        assert_eq!(supply.get(&3), None);
    }

    #[test]
    fn test_remove() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        assert_eq!(supply.len(), 1);

        let removed = supply.remove(&1);
        assert_eq!(removed, Some(0));
        assert_eq!(supply.len(), 0);
        assert_eq!(supply.get(&1), None);
    }

    #[test]
    fn test_update() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.update(Some(&1), 2, 0);

        assert_eq!(supply.get(&1), None);
        assert_eq!(supply.get(&2), Some(0));
    }

    #[test]
    fn test_clear() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        supply.insert(2, 1);
        supply.insert(3, 2);

        assert_eq!(supply.len(), 3);

        supply.clear();

        assert!(supply.is_empty());
    }

    #[test]
    fn test_overwrite() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(1, 0);
        let old = supply.insert(1, 5);

        assert_eq!(old, Some(0));
        assert_eq!(supply.get(&1), Some(5));
        assert_eq!(supply.len(), 1);
    }

    #[test]
    fn test_iter() {
        let mut supply: InverseSupply<i32> = InverseSupply::new();

        supply.insert(10, 0);
        supply.insert(20, 1);
        supply.insert(30, 2);

        let mut pairs: Vec<_> = supply.iter().map(|(&v, &i)| (v, i)).collect();
        pairs.sort();

        assert_eq!(pairs, vec![(10, 0), (20, 1), (30, 2)]);
    }
}
