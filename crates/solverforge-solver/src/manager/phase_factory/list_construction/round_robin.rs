use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::ListVariableContext;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};
use crate::PhaseFactory;

enum AssignmentMode<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    Append(fn(&mut S, usize, E)),
    InsertAtEnd {
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
    },
}

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
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, visits: Vec<()>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let builder = ListConstructionPhaseBuilder::<Plan, usize>::new(
///     |plan| plan.visits.len(),
///     |plan| plan.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |plan| plan.vehicles.len(),
///     |plan, entity_idx, element| { plan.vehicles[entity_idx].visits.push(element); },
///     |_plan, idx| idx,
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
    index_to_element: fn(&S, usize) -> E,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
}

impl<S, E> ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, E),
        index_to_element: fn(&S, usize) -> E,
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
            assignment_mode: AssignmentMode::Append(self.assign_element),
            index_to_element: self.index_to_element,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, D> PhaseFactory<S, D> for ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: Director<S>,
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
    assignment_mode: AssignmentMode<S, E>,
    index_to_element: fn(&S, usize) -> E,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
}

impl<S, E> ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    pub(crate) fn from_variable_context<DM, IDM>(ctx: &ListVariableContext<S, E, DM, IDM>) -> Self {
        Self {
            element_count: ctx.element_count,
            get_assigned: ctx.assigned_elements,
            entity_count: ctx.entity_count,
            assignment_mode: AssignmentMode::InsertAtEnd {
                list_len: ctx.list_len,
                list_insert: ctx.list_insert,
            },
            index_to_element: ctx.index_to_element,
            descriptor_index: ctx.descriptor_index,
            _marker: PhantomData,
        }
    }
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

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: Director<S>,
    BestCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<E> = (self.get_assigned)(phase_scope.score_director().working_solution());

        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        let mut entity_idx = 0;
        for elem_idx in 0..n_elements {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let element =
                (self.index_to_element)(phase_scope.score_director().working_solution(), elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            step_scope.apply_committed_change(|sd| {
                sd.before_variable_changed(self.descriptor_index, entity_idx);
                match self.assignment_mode {
                    AssignmentMode::Append(assign_element) => {
                        assign_element(sd.working_solution_mut(), entity_idx, element);
                    }
                    AssignmentMode::InsertAtEnd {
                        list_len,
                        list_insert,
                    } => {
                        let insert_pos = list_len(sd.working_solution(), entity_idx);
                        list_insert(sd.working_solution_mut(), entity_idx, insert_pos, element);
                    }
                }
                sd.after_variable_changed(self.descriptor_index, entity_idx);
            });

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();

            entity_idx = (entity_idx + 1) % n_entities;
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}
