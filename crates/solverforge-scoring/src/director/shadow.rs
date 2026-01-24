//! Shadow variable support traits.
//!
//! Provides [`ShadowVariableSupport`] and [`SolvableSolution`] traits
//! for solutions with shadow variables.

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

/// Trait for solutions that maintain shadow variables with O(1) incremental updates.
///
/// Shadow variables are derived values that depend on planning variables.
/// When a list variable element changes position, only that element's shadow
/// variables are updated - plus fixing neighbor pointers.
///
/// # O(1) Element-Level Updates
///
/// `update_element_shadow(entity_idx, position, element_idx)` does exactly:
/// 1. Set element's inverse relation (e.g., vehicle_idx)
/// 2. Set element's previous pointer
/// 3. Set element's next pointer
/// 4. Fix previous neighbor's next pointer
/// 5. Fix next neighbor's previous pointer
/// 6. Compute element's cascading shadows (e.g., arrival_time)
///
/// Total: 5 pointer updates + 1 compute = O(1)
pub trait ShadowVariableSupport: PlanningSolution {
    /// Updates shadow variables for ONE element at the given position.
    ///
    /// This is O(1): updates the element's shadows plus fixes neighbor pointers.
    /// Called after an element is inserted/moved to a new position.
    ///
    /// # Arguments
    /// - `entity_idx`: The entity (e.g., vehicle) that owns the list
    /// - `position`: The position in the entity's list where the element now sits
    /// - `element_idx`: The global index of the element being updated
    fn update_element_shadow(&mut self, entity_idx: usize, position: usize, element_idx: usize);

    /// Retracts shadow variables for ONE element before removal.
    ///
    /// This is O(1): clears the element's shadows and fixes neighbor pointers.
    /// Called before an element is removed from a position.
    ///
    /// # Arguments
    /// - `entity_idx`: The entity (e.g., vehicle) that owns the list
    /// - `position`: The position in the entity's list where the element currently sits
    /// - `element_idx`: The global index of the element being retracted
    fn retract_element_shadow(&mut self, entity_idx: usize, position: usize, element_idx: usize);

    /// Updates shadow variables for all entities.
    ///
    /// Called during initialization or after bulk solution changes.
    fn update_all_shadows(&mut self) {}

    /// Returns the descriptor index for the element collection.
    ///
    /// For VRP-style problems with `vehicles` (descriptor 0) and `visits`
    /// (descriptor 1), this returns 1 since visits have shadow variables.
    fn element_descriptor_index() -> Option<usize> {
        None
    }
}

/// Trait for solutions that can be solved using the fluent builder API.
///
/// This trait combines all requirements for automatic solver wiring:
/// - `PlanningSolution` for score management
/// - `ShadowVariableSupport` for shadow variable updates
/// - Solution descriptor for entity metadata
/// - Entity count for move selector iteration
///
/// Typically implemented automatically by the `#[planning_solution]` macro.
pub trait SolvableSolution: ShadowVariableSupport {
    /// Returns the solution descriptor for this type.
    ///
    /// The descriptor provides entity metadata for the solver infrastructure.
    fn descriptor() -> SolutionDescriptor;

    /// Returns the entity count for a given descriptor index.
    ///
    /// This is an associated function (not a method) to match the
    /// `fn(&S, usize) -> usize` signature required by `ScoreDirector`.
    fn entity_count(solution: &Self, descriptor_index: usize) -> usize;
}
