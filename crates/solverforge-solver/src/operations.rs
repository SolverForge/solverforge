//! Variable operations trait for unified solver entry point.
//!
//! This module provides the [`VariableOperations`] trait that enables a single
//! `run_solver()` function to work with both basic and list planning variables
//! through monomorphism (generics + traits, no `dyn`, no runtime polymorphism).

use std::hash::Hash;

/// Operations for manipulating planning variables during solving.
///
/// This trait is automatically implemented by the `#[planning_solution]` macro
/// based on the variable configuration attributes. It provides a unified interface
/// for both basic variables (single value assignment) and list variables (ordered
/// sequences of elements).
///
/// # Type Parameter
///
/// The `Element` associated type represents what gets assigned:
/// - For basic variables: a value index (`usize`) into the value range
/// - For list variables: an element index (`usize`) identifying which element
///
/// # Construction vs Local Search
///
/// The trait methods support two phases:
///
/// **Construction phase** uses:
/// - [`element_count`](Self::element_count) - total elements to assign
/// - [`entity_count`](Self::entity_count) - total entities available
/// - [`assigned_elements`](Self::assigned_elements) - already-assigned elements
/// - [`assign`](Self::assign) - assign an element to an entity
///
/// **Local search phase** additionally uses:
/// - [`list_len`](Self::list_len) - current list length at entity
/// - [`remove`](Self::remove) - remove element from position
/// - [`insert`](Self::insert) - insert element at position
///
/// # Basic vs List Behavior
///
/// For basic variables, the list operations have trivial implementations:
/// - `list_len` returns 1 (each entity holds exactly one value)
/// - `remove` at position 0 returns the current value
/// - `insert` at position 0 sets the new value
///
/// For list variables, these operations manipulate the actual list.
pub trait VariableOperations: Sized + Send + 'static {
    /// The element type being assigned.
    ///
    /// For both basic and list variables, this is `usize`:
    /// - Basic: index into the value range
    /// - List: index identifying the element
    type Element: Copy + Eq + Hash + Send + Sync + 'static;

    /// Returns the total number of elements to assign during construction.
    ///
    /// For basic variables: number of values in the range.
    /// For list variables: number of elements to distribute.
    fn element_count(&self) -> usize;

    /// Returns the total number of entities that can receive assignments.
    fn entity_count(&self) -> usize;

    /// Returns indices of elements already assigned to entities.
    ///
    /// Used during construction to determine which elements still need assignment.
    fn assigned_elements(&self) -> Vec<Self::Element>;

    /// Assigns an element to an entity.
    ///
    /// For basic variables: sets the entity's variable to the element value.
    /// For list variables: appends the element to the entity's list.
    fn assign(&mut self, entity_idx: usize, elem: Self::Element);

    /// Returns the list length at the given entity.
    ///
    /// For basic variables: always returns 1.
    /// For list variables: returns the current list length.
    fn list_len(&self, entity_idx: usize) -> usize;

    /// Removes and returns the element at the given position.
    ///
    /// For basic variables: position must be 0, returns current value.
    /// For list variables: removes from the list at that position.
    fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element;

    /// Inserts an element at the given position.
    ///
    /// For basic variables: position must be 0, sets new value.
    /// For list variables: inserts into the list at that position.
    fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element);

    /// Gets the element at the given position without removing it.
    ///
    /// For basic variables: position must be 0, returns current value.
    /// For list variables: returns the element at that position.
    fn get(&self, entity_idx: usize, pos: usize) -> Self::Element;

    /// Returns all possible values for basic variables.
    ///
    /// For basic variables: returns the value range.
    /// For list variables: returns empty vec (not applicable).
    fn value_range(&self) -> Vec<Self::Element> {
        Vec::new()
    }

    /// Removes a contiguous range of elements from an entity's list.
    ///
    /// For list variables: removes elements from `start` to `end` (exclusive).
    /// Returns the removed elements as a Vec.
    ///
    /// Default implementation removes elements one by one.
    fn remove_sublist(&mut self, entity_idx: usize, start: usize, end: usize) -> Vec<Self::Element> {
        let mut removed = Vec::with_capacity(end - start);
        for _ in start..end {
            removed.push(self.remove(entity_idx, start));
        }
        removed
    }

    /// Inserts multiple elements at a position in an entity's list.
    ///
    /// For list variables: inserts all elements starting at `pos`.
    ///
    /// Default implementation inserts elements one by one.
    fn insert_sublist(&mut self, entity_idx: usize, pos: usize, elements: Vec<Self::Element>) {
        for (i, elem) in elements.into_iter().enumerate() {
            self.insert(entity_idx, pos + i, elem);
        }
    }

    /// Returns the descriptor index for this variable type.
    ///
    /// Used internally for score director coordination.
    fn descriptor_index() -> usize;

    /// Returns the variable name as defined in the solution struct.
    fn variable_name() -> &'static str;

    /// Returns whether this is a list variable.
    ///
    /// `true` for list variables, `false` for basic variables.
    fn is_list_variable() -> bool;
}
