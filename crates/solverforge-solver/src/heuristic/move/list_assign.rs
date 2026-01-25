//! ListAssignMove - assigns an unassigned element to an entity's list.
//!
//! Used during construction phase to assign elements to entities.
//! Simply appends the element to the entity's list.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::{ScoreDirector, ShadowVariableSupport};

use super::traits::Move;

/// A move that assigns an unassigned element to an entity's list.
///
/// Used during construction heuristic to build an initial solution.
/// Appends the element to the end of the entity's list.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type (typically `usize` index)
pub struct ListAssignMove<S, V> {
    /// The element to assign (typically an index into an element collection)
    element: V,
    /// The entity to assign to (index into entity collection)
    entity_index: usize,
    /// Function to assign element to entity (appends to list)
    assign_fn: fn(&mut S, usize, V),
    /// Function to get list length for an entity
    list_len_fn: fn(&S, usize) -> usize,
    /// Function to remove element at position from entity
    remove_fn: fn(&mut S, usize, usize) -> V,
    /// Variable name for logging
    variable_name: &'static str,
    /// Descriptor index for the entity type
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, V: Clone> Clone for ListAssignMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            element: self.element.clone(),
            entity_index: self.entity_index,
            assign_fn: self.assign_fn,
            list_len_fn: self.list_len_fn,
            remove_fn: self.remove_fn,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Copy> Copy for ListAssignMove<S, V> {}

impl<S, V: Debug> Debug for ListAssignMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListAssignMove")
            .field("element", &self.element)
            .field("entity_index", &self.entity_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListAssignMove<S, V> {
    /// Creates a new list assign move.
    pub fn new(
        element: V,
        entity_index: usize,
        assign_fn: fn(&mut S, usize, V),
        list_len_fn: fn(&S, usize) -> usize,
        remove_fn: fn(&mut S, usize, usize) -> V,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element,
            entity_index,
            assign_fn,
            list_len_fn,
            remove_fn,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the element being assigned.
    pub fn element(&self) -> &V {
        &self.element
    }

    /// Returns the target entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }
}

impl<S, V> Move<S> for ListAssignMove<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<C>(&self, _score_director: &ScoreDirector<S, C>) -> bool
    where
        C: ConstraintSet<S, S::Score>,
    {
        // Assignment moves are always doable during construction
        true
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: ConstraintSet<S, S::Score>,
    {
        // Get insertion position (end of list) BEFORE modification
        let insert_pos = (self.list_len_fn)(score_director.working_solution(), self.entity_index);

        // Notify before change
        score_director.before_variable_changed(self.descriptor_index, self.entity_index);

        // Assign element to entity (appends to list)
        (self.assign_fn)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.element.clone(),
        );

        // Notify after change
        score_director.after_variable_changed(self.descriptor_index, self.entity_index);

        // Register undo - remove the element we just added
        let remove_fn = self.remove_fn;
        let entity_idx = self.entity_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            let _ = remove_fn(s, entity_idx, insert_pos);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}
