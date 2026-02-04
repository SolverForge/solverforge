//! List variable state supply for tracking element positions.
//!
//! # Zero-Erasure Design
//!
//! - **Index-based**: Elements identified by index, entity owners by index
//! - **Owned**: No `Arc`, `RwLock`, or interior mutability - uses `&mut self`
//! - **No Clone**: All lookups return indices or `Copy` types

use std::collections::HashMap;
use std::hash::Hash;

/// Position of an element within a list variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementPosition {
    /// The entity index that owns the list containing this element.
    pub entity_idx: usize,
    /// The index of this element within the entity's list.
    pub list_idx: usize,
}

/// Index-based list variable state supply.
///
/// Tracks which entity owns each element and at what position.
/// All values are indices - no cloning of actual domain objects.
///
/// # Type Parameters
///
/// - `E`: Element identifier type (must be `Eq + Hash`, typically `usize` or a small `Copy` type)
///
/// # Example
///
/// ```
/// use solverforge_core::domain::supply::ListStateSupply;
///
/// // Using usize as element identifier
/// let mut supply: ListStateSupply<usize> = ListStateSupply::new();
///
/// // Element 0 is at position 0 in entity 0's list
/// supply.assign(0, 0, 0);
///
/// // Element 1 is at position 1 in entity 0's list
/// supply.assign(1, 0, 1);
///
/// // Element 2 is at position 0 in entity 1's list
/// supply.assign(2, 1, 0);
///
/// assert_eq!(supply.get_position(&0), Some(solverforge_core::domain::supply::ElementPosition { entity_idx: 0, list_idx: 0 }));
/// assert_eq!(supply.get_entity(&1), Some(0));
/// assert_eq!(supply.get_list_index(&2), Some(0));
/// ```
#[derive(Debug)]
pub struct ListStateSupply<E>
where
    E: Eq + Hash,
{
    /// Map from element to its position.
    position_map: HashMap<E, ElementPosition>,
    /// Count of unassigned elements.
    unassigned_count: usize,
}

impl<E> Default for ListStateSupply<E>
where
    E: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E> ListStateSupply<E>
where
    E: Eq + Hash,
{
    /// Creates a new empty list state supply.
    pub fn new() -> Self {
        Self {
            position_map: HashMap::new(),
            unassigned_count: 0,
        }
    }

    /// Creates a new supply with initial unassigned count.
    pub fn with_unassigned(count: usize) -> Self {
        Self {
            position_map: HashMap::new(),
            unassigned_count: count,
        }
    }

    /// Initializes/resets the supply with a new unassigned count.
    pub fn initialize(&mut self, unassigned_count: usize) {
        self.position_map.clear();
        self.unassigned_count = unassigned_count;
    }

    /// Assigns an element to a position in an entity's list.
    ///
    /// Decrements the unassigned count.
    #[inline]
    pub fn assign(&mut self, element: E, entity_idx: usize, list_idx: usize) {
        let pos = ElementPosition {
            entity_idx,
            list_idx,
        };
        let old = self.position_map.insert(element, pos);
        if old.is_none() && self.unassigned_count > 0 {
            self.unassigned_count -= 1;
        }
    }

    /// Unassigns an element (removes from any list).
    ///
    /// Increments the unassigned count.
    #[inline]
    pub fn unassign(&mut self, element: &E) -> Option<ElementPosition> {
        let old = self.position_map.remove(element);
        if old.is_some() {
            self.unassigned_count += 1;
        }
        old
    }

    /// Updates an element's position.
    ///
    /// Returns true if the position changed.
    #[inline]
    pub fn update(&mut self, element: &E, entity_idx: usize, list_idx: usize) -> bool
    where
        E: Clone,
    {
        let new_pos = ElementPosition {
            entity_idx,
            list_idx,
        };
        if let Some(pos) = self.position_map.get_mut(element) {
            if *pos != new_pos {
                *pos = new_pos;
                true
            } else {
                false
            }
        } else {
            // Element wasn't tracked - assign it
            self.position_map.insert(element.clone(), new_pos);
            if self.unassigned_count > 0 {
                self.unassigned_count -= 1;
            }
            true
        }
    }

    /// Gets the full position of an element.
    #[inline]
    pub fn get_position(&self, element: &E) -> Option<ElementPosition> {
        self.position_map.get(element).copied()
    }

    /// Gets the entity index that owns this element.
    #[inline]
    pub fn get_entity(&self, element: &E) -> Option<usize> {
        self.position_map.get(element).map(|p| p.entity_idx)
    }

    /// Gets the list index of this element within its entity's list.
    #[inline]
    pub fn get_list_index(&self, element: &E) -> Option<usize> {
        self.position_map.get(element).map(|p| p.list_idx)
    }

    /// Returns true if the element is assigned to a list.
    #[inline]
    pub fn is_assigned(&self, element: &E) -> bool {
        self.position_map.contains_key(element)
    }

    /// Returns the count of unassigned elements.
    #[inline]
    pub fn unassigned_count(&self) -> usize {
        self.unassigned_count
    }

    /// Returns the count of assigned elements.
    #[inline]
    pub fn assigned_count(&self) -> usize {
        self.position_map.len()
    }

    /// Clears all position data.
    #[inline]
    pub fn clear(&mut self) {
        let assigned = self.position_map.len();
        self.position_map.clear();
        self.unassigned_count += assigned;
    }

    /// Returns an iterator over all (element, position) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&E, &ElementPosition)> {
        self.position_map.iter()
    }

    /// Returns all elements assigned to a specific entity.
    pub fn elements_for_entity(&self, entity_idx: usize) -> Vec<&E> {
        self.position_map
            .iter()
            .filter(|(_, pos)| pos.entity_idx == entity_idx)
            .map(|(e, _)| e)
            .collect()
    }
}
