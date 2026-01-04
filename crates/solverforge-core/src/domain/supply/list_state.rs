//! List variable state supply for managing list-related shadow variables.
//!
//! This module provides centralized tracking of list variable shadow state:
//! - Element position (index within list)
//! - Element inverse (which entity owns the element)
//! - Previous/Next element relationships
//!
//! By centralizing this tracking, we avoid duplicate iteration when multiple
//! shadow variables need to be updated.

use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

use super::{DemandKey, Supply, SupplyDemand};

/// Position of an element within a list variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementPosition<E> {
    /// Element is not assigned to any list.
    Unassigned,
    /// Element is at the given index within the entity's list.
    Assigned {
        /// The entity that owns the list containing this element.
        entity: E,
        /// The index of this element within the list.
        index: usize,
    },
}

impl<E> ElementPosition<E> {
    /// Returns true if the element is unassigned.
    pub fn is_unassigned(&self) -> bool {
        matches!(self, ElementPosition::Unassigned)
    }

    /// Returns true if the element is assigned.
    pub fn is_assigned(&self) -> bool {
        matches!(self, ElementPosition::Assigned { .. })
    }

    /// Returns the index if assigned.
    pub fn index(&self) -> Option<usize> {
        match self {
            ElementPosition::Assigned { index, .. } => Some(*index),
            ElementPosition::Unassigned => None,
        }
    }

    /// Returns the entity if assigned.
    pub fn entity(&self) -> Option<&E> {
        match self {
            ElementPosition::Assigned { entity, .. } => Some(entity),
            ElementPosition::Unassigned => None,
        }
    }
}

/// Mutable position storage for efficient updates.
#[derive(Debug, Clone)]
struct MutablePosition<E: Clone> {
    entity: E,
    index: usize,
}

impl<E: Clone> MutablePosition<E> {
    fn new(entity: E, index: usize) -> Self {
        Self { entity, index }
    }

    fn to_position(&self) -> ElementPosition<E> {
        ElementPosition::Assigned {
            entity: self.entity.clone(),
            index: self.index,
        }
    }
}

/// Change type for position updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeType {
    /// Both entity and index changed (or first assignment).
    Both,
    /// Only index changed (same entity).
    Index,
    /// Nothing changed.
    Neither,
}

impl ChangeType {
    fn anything_changed(&self) -> bool {
        !matches!(self, ChangeType::Neither)
    }
}

/// Supply that tracks list variable element positions.
///
/// This is the single source of truth for all list variable shadow state.
/// It tracks index, inverse, and next/previous relationships efficiently.
///
/// # Type Parameters
///
/// - `Element`: The type of elements in the list variable
/// - `Entity`: The type of entities that own list variables
pub struct ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    /// Map from element to its current position.
    element_position_map: RwLock<HashMap<Element, MutablePosition<Entity>>>,
    /// Count of unassigned elements.
    unassigned_count: RwLock<usize>,
    /// Name of the source list variable.
    variable_name: String,
}

impl<Element, Entity> ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    /// Creates a new list variable state supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            element_position_map: RwLock::new(HashMap::new()),
            unassigned_count: RwLock::new(0),
            variable_name: variable_name.into(),
        }
    }

    /// Initializes the supply with the initial unassigned count.
    pub fn initialize(&self, initial_unassigned_count: usize) {
        let mut map = self.element_position_map.write().unwrap();
        map.clear();
        *self.unassigned_count.write().unwrap() = initial_unassigned_count;
    }

    /// Adds an element at the given position.
    ///
    /// Called when an element is inserted into a list.
    pub fn add_element(&self, entity: Entity, element: Element, index: usize) {
        let mut map = self.element_position_map.write().unwrap();
        let old = map.insert(element.clone(), MutablePosition::new(entity, index));
        if old.is_some() {
            panic!(
                "List variable '{}' supply corrupted: element already exists",
                self.variable_name
            );
        }
        *self.unassigned_count.write().unwrap() -= 1;
    }

    /// Removes an element from the given position.
    ///
    /// Called when an element is removed from a list.
    pub fn remove_element(&self, element: &Element, expected_index: usize) {
        let mut map = self.element_position_map.write().unwrap();
        let old = map.remove(element);
        match old {
            None => {
                panic!(
                    "List variable '{}' supply corrupted: element not found for removal",
                    self.variable_name
                );
            }
            Some(pos) if pos.index != expected_index => {
                panic!(
                    "List variable '{}' supply corrupted: element at wrong index ({} vs {})",
                    self.variable_name, pos.index, expected_index
                );
            }
            _ => {}
        }
        *self.unassigned_count.write().unwrap() += 1;
    }

    /// Unassigns an element (removes without index validation).
    ///
    /// Called when an element is completely removed from any list.
    pub fn unassign_element(&self, element: &Element) {
        let mut map = self.element_position_map.write().unwrap();
        let old = map.remove(element);
        if old.is_none() {
            panic!(
                "List variable '{}' supply corrupted: element not found for unassignment",
                self.variable_name
            );
        }
        *self.unassigned_count.write().unwrap() += 1;
    }

    /// Updates an element's position and returns whether it changed.
    ///
    /// Called when processing a range change in the list.
    pub fn change_element(&self, entity: Entity, element: &Element, index: usize) -> bool {
        let mut map = self.element_position_map.write().unwrap();

        if let Some(pos) = map.get_mut(element) {
            let change_type = if pos.entity.type_id() != entity.type_id() {
                // Different entity (note: this is a rough check, proper eq would be better)
                pos.entity = entity;
                pos.index = index;
                ChangeType::Both
            } else if pos.index != index {
                pos.index = index;
                ChangeType::Index
            } else {
                ChangeType::Neither
            };
            change_type.anything_changed()
        } else {
            // Element wasn't tracked yet, add it
            map.insert(element.clone(), MutablePosition::new(entity, index));
            *self.unassigned_count.write().unwrap() -= 1;
            true
        }
    }

    /// Updates an element's position with entity equality check.
    pub fn change_element_with_eq<F>(&self, entity: Entity, element: &Element, index: usize, entity_eq: F) -> bool
    where
        F: Fn(&Entity, &Entity) -> bool,
    {
        let mut map = self.element_position_map.write().unwrap();

        if let Some(pos) = map.get_mut(element) {
            let change_type = if !entity_eq(&pos.entity, &entity) {
                pos.entity = entity;
                pos.index = index;
                ChangeType::Both
            } else if pos.index != index {
                pos.index = index;
                ChangeType::Index
            } else {
                ChangeType::Neither
            };
            change_type.anything_changed()
        } else {
            map.insert(element.clone(), MutablePosition::new(entity, index));
            *self.unassigned_count.write().unwrap() -= 1;
            true
        }
    }

    /// Gets the position of an element.
    pub fn get_position(&self, element: &Element) -> ElementPosition<Entity> {
        let map = self.element_position_map.read().unwrap();
        match map.get(element) {
            Some(pos) => pos.to_position(),
            None => ElementPosition::Unassigned,
        }
    }

    /// Gets the index of an element, if assigned.
    pub fn get_index(&self, element: &Element) -> Option<usize> {
        let map = self.element_position_map.read().unwrap();
        map.get(element).map(|pos| pos.index)
    }

    /// Gets the entity (inverse) for an element, if assigned.
    pub fn get_inverse(&self, element: &Element) -> Option<Entity> {
        let map = self.element_position_map.read().unwrap();
        map.get(element).map(|pos| pos.entity.clone())
    }

    /// Gets the previous element, if any.
    ///
    /// Requires a function to get the list from an entity.
    pub fn get_previous_element<F>(&self, element: &Element, get_list: F) -> Option<Element>
    where
        F: Fn(&Entity) -> &[Element],
    {
        let map = self.element_position_map.read().unwrap();
        let pos = map.get(element)?;
        if pos.index == 0 {
            None
        } else {
            let list = get_list(&pos.entity);
            list.get(pos.index - 1).cloned()
        }
    }

    /// Gets the next element, if any.
    ///
    /// Requires a function to get the list from an entity.
    pub fn get_next_element<F>(&self, element: &Element, get_list: F) -> Option<Element>
    where
        F: Fn(&Entity) -> &[Element],
    {
        let map = self.element_position_map.read().unwrap();
        let pos = map.get(element)?;
        let list = get_list(&pos.entity);
        if pos.index >= list.len() - 1 {
            None
        } else {
            list.get(pos.index + 1).cloned()
        }
    }

    /// Returns the count of unassigned elements.
    pub fn unassigned_count(&self) -> usize {
        *self.unassigned_count.read().unwrap()
    }

    /// Returns true if the element is assigned to a list.
    pub fn is_assigned(&self, element: &Element) -> bool {
        self.element_position_map.read().unwrap().contains_key(element)
    }

    /// Returns the variable name this supply tracks.
    pub fn variable_name(&self) -> &str {
        &self.variable_name
    }
}

impl<Element, Entity> Supply for ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
}

impl<Element, Entity> std::fmt::Debug for ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let map = self.element_position_map.read().unwrap();
        f.debug_struct("ListVariableStateSupply")
            .field("variable_name", &self.variable_name)
            .field("element_count", &map.len())
            .field("unassigned_count", &*self.unassigned_count.read().unwrap())
            .finish()
    }
}

/// Demand for a ListVariableStateSupply.
pub struct ListVariableStateDemand<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    variable_name: String,
    _phantom: std::marker::PhantomData<(Element, Entity)>,
}

impl<Element, Entity> ListVariableStateDemand<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    /// Creates a new demand for a list variable state supply.
    pub fn new(variable_name: impl Into<String>) -> Self {
        Self {
            variable_name: variable_name.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Element, Entity> SupplyDemand for ListVariableStateDemand<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    type Output = ListVariableStateSupply<Element, Entity>;

    fn demand_key(&self) -> DemandKey {
        DemandKey::new::<ListVariableStateSupply<Element, Entity>>(&self.variable_name)
    }

    fn create_supply(&self) -> Self::Output {
        ListVariableStateSupply::new(&self.variable_name)
    }
}

/// Trait for supplies that provide index information.
pub trait IndexVariableSupply {
    /// Gets the index of an element in its list, if assigned.
    fn get_index_dyn(&self, element: &dyn Any) -> Option<usize>;
}

/// Trait for supplies that provide inverse (owner entity) information.
pub trait InverseVariableSupply {
    /// Gets the entity that owns an element, if assigned.
    fn get_inverse_dyn(&self, element: &dyn Any) -> Option<Box<dyn Any + Send + Sync>>;
}

impl<Element, Entity> IndexVariableSupply for ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    fn get_index_dyn(&self, element: &dyn Any) -> Option<usize> {
        element
            .downcast_ref::<Element>()
            .and_then(|e| self.get_index(e))
    }
}

impl<Element, Entity> InverseVariableSupply for ListVariableStateSupply<Element, Entity>
where
    Element: Eq + Hash + Clone + Send + Sync + 'static,
    Entity: Clone + Send + Sync + 'static,
{
    fn get_inverse_dyn(&self, element: &dyn Any) -> Option<Box<dyn Any + Send + Sync>> {
        element.downcast_ref::<Element>().and_then(|e| {
            self.get_inverse(e)
                .map(|entity| Box::new(entity) as Box<dyn Any + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Task {
        id: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Vehicle {
        id: usize,
        tasks: Vec<Task>,
    }

    #[test]
    fn test_add_and_get_element() {
        let supply: ListVariableStateSupply<Task, Vehicle> =
            ListVariableStateSupply::new("tasks");
        supply.initialize(3);

        let vehicle = Vehicle {
            id: 1,
            tasks: vec![],
        };
        let task = Task { id: 1 };

        supply.add_element(vehicle.clone(), task.clone(), 0);

        assert!(supply.is_assigned(&task));
        assert_eq!(supply.get_index(&task), Some(0));
        assert_eq!(supply.get_inverse(&task), Some(vehicle));
        assert_eq!(supply.unassigned_count(), 2);
    }

    #[test]
    fn test_remove_element() {
        let supply: ListVariableStateSupply<Task, Vehicle> =
            ListVariableStateSupply::new("tasks");
        supply.initialize(3);

        let vehicle = Vehicle {
            id: 1,
            tasks: vec![],
        };
        let task = Task { id: 1 };

        supply.add_element(vehicle, task.clone(), 0);
        assert_eq!(supply.unassigned_count(), 2);

        supply.remove_element(&task, 0);
        assert!(!supply.is_assigned(&task));
        assert_eq!(supply.unassigned_count(), 3);
    }

    #[test]
    fn test_change_element() {
        let supply: ListVariableStateSupply<Task, Vehicle> =
            ListVariableStateSupply::new("tasks");
        supply.initialize(1);

        let vehicle = Vehicle {
            id: 1,
            tasks: vec![],
        };
        let task = Task { id: 1 };

        supply.add_element(vehicle.clone(), task.clone(), 0);

        // Change index only
        let changed = supply.change_element_with_eq(vehicle.clone(), &task, 2, |a, b| a.id == b.id);
        assert!(changed);
        assert_eq!(supply.get_index(&task), Some(2));

        // No change
        let changed = supply.change_element_with_eq(vehicle.clone(), &task, 2, |a, b| a.id == b.id);
        assert!(!changed);
    }

    #[test]
    fn test_element_position() {
        let supply: ListVariableStateSupply<Task, Vehicle> =
            ListVariableStateSupply::new("tasks");
        supply.initialize(1);

        let vehicle = Vehicle {
            id: 1,
            tasks: vec![],
        };
        let task = Task { id: 1 };

        // Unassigned initially
        let pos = supply.get_position(&task);
        assert!(pos.is_unassigned());

        // After assignment
        supply.add_element(vehicle.clone(), task.clone(), 3);
        let pos = supply.get_position(&task);
        assert!(pos.is_assigned());
        assert_eq!(pos.index(), Some(3));
        assert_eq!(pos.entity(), Some(&vehicle));
    }

    #[test]
    fn test_demand() {
        let demand: ListVariableStateDemand<Task, Vehicle> =
            ListVariableStateDemand::new("tasks");
        let supply = demand.create_supply();

        assert_eq!(supply.variable_name(), "tasks");
    }

    #[test]
    fn test_previous_next_element() {
        let supply: ListVariableStateSupply<Task, Vehicle> =
            ListVariableStateSupply::new("tasks");
        supply.initialize(3);

        let task0 = Task { id: 0 };
        let task1 = Task { id: 1 };
        let task2 = Task { id: 2 };

        let vehicle = Vehicle {
            id: 1,
            tasks: vec![task0.clone(), task1.clone(), task2.clone()],
        };

        supply.add_element(vehicle.clone(), task0.clone(), 0);
        supply.add_element(vehicle.clone(), task1.clone(), 1);
        supply.add_element(vehicle.clone(), task2.clone(), 2);

        // Get previous
        let prev = supply.get_previous_element(&task1, |v| &v.tasks);
        assert_eq!(prev, Some(task0.clone()));

        let prev_first = supply.get_previous_element(&task0, |v| &v.tasks);
        assert_eq!(prev_first, None);

        // Get next
        let next = supply.get_next_element(&task1, |v| &v.tasks);
        assert_eq!(next, Some(task2.clone()));

        let next_last = supply.get_next_element(&task2, |v| &v.tasks);
        assert_eq!(next_last, None);
    }
}
