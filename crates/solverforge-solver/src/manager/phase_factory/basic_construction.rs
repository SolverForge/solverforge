//! Basic construction phase for assigning values to planning variables.
//!
//! Provides first-fit construction for basic variables (e.g., assigning employees to shifts).

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

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
/// use solverforge_solver::manager::BasicConstructionPhaseBuilder;
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
/// let builder = BasicConstructionPhaseBuilder::<Schedule, usize>::new(
///     get_employee,
///     set_employee,
///     value_count,
///     entity_count,
///     "employee_idx",
///     0,
/// );
///
/// // Create a concrete phase:
/// let phase: BasicConstructionPhase<Schedule, usize> = builder.create_phase();
/// ```
pub struct BasicConstructionPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + 'static,
{
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    value_count: fn(&S) -> usize,
    entity_count: fn(&S) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<(S, V)>,
}

impl<S, V> BasicConstructionPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + 'static,
{
    /// Creates a new basic construction phase builder.
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

    /// Creates a basic construction phase.
    pub fn create_phase(&self) -> BasicConstructionPhase<S, V> {
        BasicConstructionPhase {
            getter: self.getter,
            setter: self.setter,
            value_count: self.value_count,
            entity_count: self.entity_count,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, V, D> SolverPhaseFactory<S, D, BasicConstructionPhase<S, V>>
    for BasicConstructionPhaseBuilder<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + From<usize> + 'static,
    D: ScoreDirector<S>,
{
    fn create_phase(&self) -> BasicConstructionPhase<S, V> {
        BasicConstructionPhaseBuilder::create_phase(self)
    }
}

/// Basic construction phase that assigns values to uninitialized entities.
pub struct BasicConstructionPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + 'static,
{
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    value_count: fn(&S) -> usize,
    entity_count: fn(&S) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<(S, V)>,
}

impl<S, V> std::fmt::Debug for BasicConstructionPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicConstructionPhase")
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, D> Phase<S, D> for BasicConstructionPhase<S, V>
where
    S: PlanningSolution,
    V: Copy + PartialEq + Send + Sync + From<usize> + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_values = (self.value_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_values == 0 || n_entities == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let mut value_idx = 0usize;
        for entity_idx in 0..n_entities {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let current =
                (self.getter)(phase_scope.score_director().working_solution(), entity_idx);
            if current.is_some() {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            {
                let sd = step_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
                (self.setter)(
                    sd.working_solution_mut(),
                    entity_idx,
                    Some(V::from(value_idx)),
                );
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
