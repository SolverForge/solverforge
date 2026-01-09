//! List construction phase for assigning list elements to entities.
//!
//! Provides round-robin construction for list variables (e.g., assigning visits to vehicles).

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

use super::super::SolverPhaseFactory;

/// Builder for creating list construction phases.
///
/// This builder creates phases that assign unassigned list elements to entities
/// using a round-robin strategy. Ideal for VRP-style problems where visits
/// need to be distributed across vehicles.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `E` - The element type (e.g., visit index)
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{ListConstructionPhaseBuilder, SolverPhaseFactory};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, visits: Vec<()>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let factory = ListConstructionPhaseBuilder::<Plan, usize>::new(
///     |plan| plan.visits.len(),
///     |plan| plan.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |plan| plan.vehicles.len(),
///     |plan, entity_idx, element| { plan.vehicles[entity_idx].visits.push(element); },
///     |idx| idx,
///     "visits",
///     1,
/// );
/// ```
pub struct ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Clone + Send + Sync + 'static,
{
    /// Returns total number of elements to assign
    element_count: fn(&S) -> usize,
    /// Returns currently assigned elements
    get_assigned: fn(&S) -> Vec<E>,
    /// Returns number of entities to assign to
    entity_count: fn(&S) -> usize,
    /// Assigns an element to an entity
    assign_element: fn(&mut S, usize, E),
    /// Converts element index to element type
    index_to_element: fn(usize) -> E,
    /// Variable name for change notification
    variable_name: &'static str,
    /// Descriptor index for entity collection
    descriptor_index: usize,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Clone + Send + Sync + 'static,
{
    /// Creates a new list construction phase builder.
    ///
    /// # Arguments
    ///
    /// * `element_count` - Function returning total elements to assign
    /// * `get_assigned` - Function returning already-assigned elements
    /// * `entity_count` - Function returning number of entities
    /// * `assign_element` - Function to assign an element to an entity
    /// * `index_to_element` - Function to convert index to element type
    /// * `variable_name` - Name of the list variable for change notification
    /// * `descriptor_index` - Entity descriptor index for the list owner
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, E),
        index_to_element: fn(usize) -> E,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            assign_element,
            index_to_element,
            variable_name,
            descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E> SolverPhaseFactory<S> for ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution + 'static,
    E: Clone + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        Box::new(ListConstructionPhase {
            element_count: self.element_count,
            get_assigned: self.get_assigned,
            entity_count: self.entity_count,
            assign_element: self.assign_element,
            index_to_element: self.index_to_element,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        })
    }
}

/// List construction phase that assigns elements round-robin to entities.
struct ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Clone + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(usize) -> E,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> std::fmt::Debug for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListConstructionPhase").finish()
    }
}

impl<S, E> Phase<S> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Clone + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            // Nothing to assign - just calculate initial score and update best
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        // Get already-assigned elements
        let assigned: Vec<E> = (self.get_assigned)(phase_scope.score_director().working_solution());

        // If all elements already assigned, skip construction
        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        // Build set of assigned elements for O(1) lookup
        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        // Round-robin assignment
        let mut entity_idx = 0;
        for elem_idx in 0..n_elements {
            // Check early termination
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            let element = (self.index_to_element)(elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Assign element to entity and notify score director
            {
                let sd = step_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
                (self.assign_element)(sd.working_solution_mut(), entity_idx, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
            }

            // Calculate score after assignment
            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();

            entity_idx = (entity_idx + 1) % n_entities;
        }

        // Update best solution at end of construction
        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}
