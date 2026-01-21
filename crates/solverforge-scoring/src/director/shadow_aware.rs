//! Shadow-aware score director for solutions with shadow variables.
//!
//! Provides [`ShadowVariableSupport`] trait and [`ShadowAwareScoreDirector`]
//! that integrates shadow variable updates into the change notification protocol.

use std::any::Any;
use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::ScoreDirector;

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
    /// `fn(&S, usize) -> usize` signature required by `TypedScoreDirector`.
    fn entity_count(solution: &Self, descriptor_index: usize) -> usize;
}

/// A score director that integrates O(1) shadow variable updates.
///
/// Wraps an inner score director and calls [`ShadowVariableSupport::update_element_shadow`]
/// and [`ShadowVariableSupport::retract_element_shadow`] for true O(1) incremental updates.
///
/// # Type Parameters
///
/// - `S`: Solution type (must implement [`ShadowVariableSupport`])
/// - `D`: Inner score director type (zero-erasure, no trait objects)
pub struct ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    inner: D,
    _phantom: PhantomData<S>,
}

impl<S, D> ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    /// Creates a new shadow-aware score director wrapping the given inner director.
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the inner score director.
    pub fn inner(&self) -> &D {
        &self.inner
    }

    /// Returns a mutable reference to the inner score director.
    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.inner
    }

    /// Consumes self and returns the inner score director.
    pub fn into_inner(self) -> D {
        self.inner
    }
}

use crate::api::constraint_set::ConstraintSet;
use crate::director::typed::TypedScoreDirector;
use solverforge_core::score::Score;

impl<S, C> ShadowAwareScoreDirector<S, TypedScoreDirector<S, C>>
where
    S: ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    /// Returns constraint match totals for score analysis.
    ///
    /// Returns a vector of (name, weight, score, match_count) tuples.
    pub fn constraint_match_totals(&self) -> Vec<(String, S::Score, S::Score, usize)> {
        self.inner.constraint_match_totals()
    }
}

impl<S, D> ScoreDirector<S> for ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    fn working_solution(&self) -> &S {
        self.inner.working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.inner.working_solution_mut()
    }

    fn calculate_score(&mut self) -> S::Score {
        self.inner.calculate_score()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner.solution_descriptor()
    }

    fn clone_working_solution(&self) -> S {
        self.inner.clone_working_solution()
    }

    fn before_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        // Forward to inner for basic variable changes (no shadow updates needed)
        self.inner
            .before_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn after_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        // Forward to inner for basic variable changes (no shadow updates needed)
        self.inner
            .after_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn before_list_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        position: usize,
        element_idx: usize,
        variable_name: &str,
    ) {
        // O(1): Retract shadow for ONE element before removal
        self.inner.working_solution_mut().retract_element_shadow(
            entity_index,
            position,
            element_idx,
        );

        // Retract element from constraints
        if let Some(elem_descriptor) = S::element_descriptor_index() {
            self.inner.before_list_variable_changed(
                elem_descriptor,
                entity_index,
                position,
                element_idx,
                "shadow",
            );
        }

        // Retract entity from constraints
        self.inner.before_list_variable_changed(
            descriptor_index,
            entity_index,
            position,
            element_idx,
            variable_name,
        );
    }

    fn after_list_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        position: usize,
        element_idx: usize,
        variable_name: &str,
    ) {
        // O(1): Update shadow for ONE element after insertion
        self.inner.working_solution_mut().update_element_shadow(
            entity_index,
            position,
            element_idx,
        );

        // Insert element into constraints
        if let Some(elem_descriptor) = S::element_descriptor_index() {
            self.inner.after_list_variable_changed(
                elem_descriptor,
                entity_index,
                position,
                element_idx,
                "shadow",
            );
        }

        // Insert entity into constraints
        self.inner.after_list_variable_changed(
            descriptor_index,
            entity_index,
            position,
            element_idx,
            variable_name,
        );
    }

    fn trigger_variable_listeners(&mut self) {
        self.inner.trigger_variable_listeners();
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner.entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner.total_entity_count()
    }

    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any> {
        self.inner.get_entity(descriptor_index, entity_index)
    }

    fn is_incremental(&self) -> bool {
        self.inner.is_incremental()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.inner.register_undo(undo);
    }
}

impl<S, D> std::fmt::Debug for ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport + std::fmt::Debug,
    D: ScoreDirector<S> + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShadowAwareScoreDirector")
            .field("inner", &self.inner)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::typed::TypedScoreDirector;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        elements: Vec<i32>,
        vehicle_idx: Vec<Option<usize>>,
        prev_idx: Vec<Option<usize>>,
        next_idx: Vec<Option<usize>>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl ShadowVariableSupport for TestSolution {
        fn update_element_shadow(
            &mut self,
            entity_idx: usize,
            position: usize,
            element_idx: usize,
        ) {
            // O(1): Update one element's shadows
            self.vehicle_idx[element_idx] = Some(entity_idx);
            self.prev_idx[element_idx] = if position > 0 {
                Some(position - 1)
            } else {
                None
            };
            self.next_idx[element_idx] = if position < self.elements.len() - 1 {
                Some(position + 1)
            } else {
                None
            };
        }

        fn retract_element_shadow(
            &mut self,
            entity_idx: usize,
            position: usize,
            element_idx: usize,
        ) {
            // Use parameters
            let _ = (entity_idx, position);
            // O(1): Clear one element's shadows
            self.vehicle_idx[element_idx] = None;
            self.prev_idx[element_idx] = None;
            self.next_idx[element_idx] = None;
        }
    }

    #[test]
    fn shadow_update_on_list_variable_change() {
        let solution = TestSolution {
            elements: vec![10, 20, 30],
            vehicle_idx: vec![None, None, None],
            prev_idx: vec![None, None, None],
            next_idx: vec![None, None, None],
            score: None,
        };

        let inner = TypedScoreDirector::new(solution, ());
        let mut director = ShadowAwareScoreDirector::new(inner);

        // Initialize
        director.calculate_score();

        // Insert element 1 at position 0 of entity 0
        director.after_list_variable_changed(0, 0, 0, 1, "visits");

        // Shadow should be updated
        assert_eq!(director.working_solution().vehicle_idx[1], Some(0));
    }

    #[test]
    fn inner_access() {
        let solution = TestSolution {
            elements: vec![1, 2, 3],
            vehicle_idx: vec![None, None, None],
            prev_idx: vec![None, None, None],
            next_idx: vec![None, None, None],
            score: None,
        };

        let inner = TypedScoreDirector::new(solution, ());
        let director = ShadowAwareScoreDirector::new(inner);

        assert!(!director.inner().is_initialized());
    }

    #[test]
    fn into_inner_consumes() {
        let solution = TestSolution {
            elements: vec![1],
            vehicle_idx: vec![None],
            prev_idx: vec![None],
            next_idx: vec![None],
            score: None,
        };

        let inner = TypedScoreDirector::new(solution, ());
        let director = ShadowAwareScoreDirector::new(inner);

        let recovered = director.into_inner();
        assert_eq!(recovered.working_solution().elements.len(), 1);
    }
}
