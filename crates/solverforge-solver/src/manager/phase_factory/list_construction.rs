//! List construction phase for assigning list elements to entities.
//!
//! Provides round-robin construction for list variables (e.g., assigning visits to vehicles).

use std::marker::PhantomData;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use tracing::info;

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

use super::super::PhaseFactory;

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
/// use solverforge_solver::{ListConstructionPhase, ListConstructionPhaseBuilder};
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
/// let builder = ListConstructionPhaseBuilder::<Plan, usize>::new(
///     |plan| plan.visits.len(),
///     |plan| plan.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |plan| plan.vehicles.len(),
///     |plan, entity_idx, element| { plan.vehicles[entity_idx].visits.push(element); },
///     |idx| idx,
///     1,
/// );
///
/// // Create a concrete phase:
/// let phase: ListConstructionPhase<Plan, usize> = builder.create_phase();
/// ```
pub struct ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(usize) -> E,
    descriptor_index: usize,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    /// Creates a new list construction phase builder.
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, E),
        index_to_element: fn(usize) -> E,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            assign_element,
            index_to_element,
            descriptor_index,
            _marker: PhantomData,
        }
    }

    /// Creates the list construction phase.
    pub fn create_phase(&self) -> ListConstructionPhase<S, E> {
        ListConstructionPhase {
            element_count: self.element_count,
            get_assigned: self.get_assigned,
            entity_count: self.entity_count,
            assign_element: self.assign_element,
            index_to_element: self.index_to_element,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, C> PhaseFactory<S, C> for ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    S::Score: Score,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    C: ConstraintSet<S, S::Score>,
{
    type Phase = ListConstructionPhase<S, E>;

    fn create(&self) -> Self::Phase {
        ListConstructionPhaseBuilder::create_phase(self)
    }
}

/// List construction phase that assigns elements round-robin to entities.
pub struct ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(usize) -> E,
    descriptor_index: usize,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> std::fmt::Debug for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListConstructionPhase").finish()
    }
}

impl<S, E, C> Phase<S, C> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    S::Score: Score,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    C: ConstraintSet<S, S::Score>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>) {
        let phase_start = Instant::now();
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        info!(event = "phase_start", phase = "ConstructionHeuristic");

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            let duration = phase_start.elapsed();
            info!(
                event = "phase_end",
                phase = "ConstructionHeuristic",
                duration_ms = duration.as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = %score,
            );
            return;
        }

        let assigned: Vec<E> = (self.get_assigned)(phase_scope.score_director().working_solution());

        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            let score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            let duration = phase_start.elapsed();
            info!(
                event = "phase_end",
                phase = "ConstructionHeuristic",
                duration_ms = duration.as_millis() as u64,
                steps = 0u64,
                speed = 0u64,
                score = %score,
            );
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        let mut entity_idx = 0;
        let mut steps = 0u64;
        let mut last_score = None;
        for elem_idx in 0..n_elements {
            if phase_scope.solver_scope().is_terminate_early() {
                break;
            }

            let element = (self.index_to_element)(elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            {
                let sd = step_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx);
                (self.assign_element)(sd.working_solution_mut(), entity_idx, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx);
            }

            let step_score = step_scope.calculate_score();
            last_score = Some(step_score);
            step_scope.set_step_score(step_score);
            step_scope.complete();

            entity_idx = (entity_idx + 1) % n_entities;
            steps += 1;
        }

        phase_scope.update_best_solution();

        let duration = phase_start.elapsed();
        let speed = if duration.as_secs_f64() > 0.0 {
            (steps as f64 / duration.as_secs_f64()) as u64
        } else {
            0
        };
        let final_score = last_score
            .or_else(|| phase_scope.solver_scope().best_score().copied())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "N/A".to_string());
        info!(
            event = "phase_end",
            phase = "ConstructionHeuristic",
            duration_ms = duration.as_millis() as u64,
            steps = steps,
            speed = speed,
            score = %final_score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}
