//! Basic construction phase for assigning values to planning variables.
//!
//! Provides first-fit construction for basic variables (e.g., assigning employees to shifts).

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

use super::super::SolverPhaseFactory;

/// Builder for creating basic variable construction phases.
///
/// This builder creates phases that assign values to uninitialized basic
/// planning variables (e.g., `Option<usize>`). Uses round-robin assignment.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `V` - The value type (e.g., `usize`)
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{BasicConstructionPhaseBuilder, SolverPhaseFactory};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Shift { employee_idx: Option<usize> }
///
/// #[derive(Clone)]
/// struct Schedule { shifts: Vec<Shift>, employees: Vec<()>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_employee(s: &Schedule, idx: usize) -> Option<usize> {
///     s.shifts.get(idx).and_then(|shift| shift.employee_idx)
/// }
///
/// fn set_employee(s: &mut Schedule, idx: usize, v: Option<usize>) {
///     if let Some(shift) = s.shifts.get_mut(idx) {
///         shift.employee_idx = v;
///     }
/// }
///
/// fn value_count(s: &Schedule) -> usize { s.employees.len() }
/// fn entity_count(s: &Schedule) -> usize { s.shifts.len() }
///
/// let factory = BasicConstructionPhaseBuilder::<Schedule, usize>::new(
///     get_employee,
///     set_employee,
///     value_count,
///     entity_count,
///     "employee_idx",
///     0,
/// );
/// ```
pub struct BasicConstructionPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    /// Typed getter for the planning variable
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter for the planning variable
    setter: fn(&mut S, usize, Option<V>),
    /// Returns the number of valid values (0..value_count)
    value_count: fn(&S) -> usize,
    /// Returns the number of entities
    entity_count: fn(&S) -> usize,
    /// Variable name for change notification
    variable_name: &'static str,
    /// Descriptor index for entity collection
    descriptor_index: usize,
    _marker: PhantomData<(S, V)>,
}

impl<S, V> BasicConstructionPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    /// Creates a new basic construction phase builder.
    ///
    /// # Arguments
    ///
    /// * `getter` - Function to get the variable value for an entity
    /// * `setter` - Function to set the variable value for an entity
    /// * `value_count` - Function returning the number of valid values
    /// * `entity_count` - Function returning the number of entities
    /// * `variable_name` - Name of the variable for change notification
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        value_count: fn(&S) -> usize,
        entity_count: fn(&S) -> usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            getter,
            setter,
            value_count,
            entity_count,
            variable_name,
            descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S> SolverPhaseFactory<S> for BasicConstructionPhaseBuilder<S, usize>
where
    S: PlanningSolution + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        Box::new(BasicConstructionPhase {
            getter: self.getter,
            setter: self.setter,
            value_count: self.value_count,
            entity_count: self.entity_count,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        })
    }
}

/// Basic construction phase that assigns values to uninitialized entities.
struct BasicConstructionPhase<S>
where
    S: PlanningSolution,
{
    getter: fn(&S, usize) -> Option<usize>,
    setter: fn(&mut S, usize, Option<usize>),
    value_count: fn(&S) -> usize,
    entity_count: fn(&S) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<S>,
}

impl<S> std::fmt::Debug for BasicConstructionPhase<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicConstructionPhase")
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> Phase<S> for BasicConstructionPhase<S>
where
    S: PlanningSolution,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_values = (self.value_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_values == 0 || n_entities == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        // Round-robin assignment: for each uninitialized entity, assign first available value
        let mut value_idx = 0usize;
        for entity_idx in 0..n_entities {
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            // Skip if already assigned
            let current =
                (self.getter)(phase_scope.score_director().working_solution(), entity_idx);
            if current.is_some() {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            // Assign value and notify score director
            {
                let sd = step_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
                (self.setter)(sd.working_solution_mut(), entity_idx, Some(value_idx));
                sd.after_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
            }

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();

            value_idx = (value_idx + 1) % n_values;
        }

        phase_scope.update_best_solution();
        tracing::info!(
            best_score = ?phase_scope.solver_scope().best_score(),
            "BasicConstruction complete"
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "BasicConstruction"
    }
}
