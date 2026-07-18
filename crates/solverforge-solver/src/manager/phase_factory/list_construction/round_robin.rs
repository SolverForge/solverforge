use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListSourceAccess,
    RuntimeListElement, RuntimeListSlot,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_placement::OwnerRestriction;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};
use crate::PhaseFactory;

mod kernel;

pub(crate) use kernel::{run_round_robin, RoundRobinAccess};

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
///     |_plan, element| *element,
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
    element_source_key: fn(&S, &E) -> usize,
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
        element_source_key: fn(&S, &E) -> usize,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            assign_element,
            index_to_element,
            element_source_key,
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
            element_source_key: self.element_source_key,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, D> PhaseFactory<S, D> for ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
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
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(&S, usize) -> E,
    element_source_key: fn(&S, &E) -> usize,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
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

impl<S, E> ListSourceAccess<S> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    type Element = E;

    fn element_count(&self, solution: &S) -> usize {
        (self.element_count)(solution)
    }

    fn index_to_element(&self, solution: &S, source_index: usize) -> Option<Self::Element> {
        (source_index < (self.element_count)(solution))
            .then(|| (self.index_to_element)(solution, source_index))
    }

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        (self.element_source_key)(solution, element)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        (self.get_assigned)(solution)
    }
}

impl<S, E> RoundRobinAccess<S> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    type Element = E;

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        let _ = (solution, element);
        0
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        let _ = (solution, entity_count, element);
        OwnerRestriction::Unrestricted
    }

    fn append_element(&self, solution: &mut S, entity_index: usize, element: Self::Element) {
        (self.assign_element)(solution, entity_index, element);
    }
}

impl<S, V, DM, IDM> RoundRobinAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        ListAccess::construction_order_key(self, solution, element.clone())
            .expect("compiled round-robin construction order must be validated before execution")
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        match ListAccess::element_owner(self, solution, element)
            .expect("compiled round-robin ownership policy must be callable")
        {
            None => OwnerRestriction::Unrestricted,
            Some(owner_idx) if owner_idx < entity_count => OwnerRestriction::Fixed(owner_idx),
            Some(_) => OwnerRestriction::Invalid,
        }
    }

    fn append_element(&self, solution: &mut S, entity_index: usize, element: Self::Element) {
        let insert_position = ListAccess::list_len(self, solution, entity_index);
        ListAccess::list_insert(self, solution, entity_index, insert_position, element);
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        let binding = bind_runtime_list_source(self, solver_scope.working_solution())
            .unwrap_or_else(|error| {
                panic!(
                    "ListConstructionPhase source binding failed before phase execution: {error:?}"
                )
            });
        let source_index = binding.into_source_index();
        let unassigned = unassigned_from_current_assignment(
            self,
            &source_index,
            solver_scope.working_solution(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "ListConstructionPhase assignment refresh failed before phase execution: {error:?}"
            )
        });
        let all_assigned = unassigned.is_empty();
        run_round_robin(
            self,
            source_index.source_count(),
            all_assigned,
            &unassigned,
            StepControlPolicy::ObserveConfigLimits,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}

#[cfg(test)]
#[path = "round_robin/tests.rs"]
mod tests;
