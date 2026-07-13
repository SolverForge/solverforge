//! Public facade for canonical source-indexed regret insertion.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListSourceAccess,
    RuntimeListSourceIndex,
};
use crate::list_placement::OwnerRestriction;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

mod kernel;

use kernel::RegretAccess;

pub(crate) use kernel::run_regret;

/// List construction phase using the canonical regret-insertion kernel.
///
/// The explicit source key is part of the public contract: it identifies a
/// declared element independently of its payload representation, so assigned
/// values, precedence successors, and trace coordinates never rely on
/// equality/hash recovery.
pub struct ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    list_remove: fn(&mut S, usize, usize) -> E,
    index_to_element: fn(&S, usize) -> E,
    element_source_key: fn(&S, &E) -> usize,
    element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    element_order_key: Option<fn(&S, E) -> i64>,
    precedence_duration_fn: Option<fn(&S, E) -> usize>,
    precedence_successors_fn: Option<fn(&S, E, &mut Vec<E>)>,
    descriptor_index: usize,
    _marker: PhantomData<fn() -> (S, E)>,
}

impl<S, E> std::fmt::Debug for ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRegretInsertionPhase").finish()
    }
}

impl<S, E> ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    /// Creates a regret-insertion phase over an explicitly keyed source.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        list_remove: fn(&mut S, usize, usize) -> E,
        index_to_element: fn(&S, usize) -> E,
        element_source_key: fn(&S, &E) -> usize,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            element_source_key,
            element_owner_fn: None,
            element_order_key: None,
            precedence_duration_fn: None,
            precedence_successors_fn: None,
            descriptor_index,
            _marker: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }

    pub fn with_element_order_key(mut self, element_order_key: Option<fn(&S, E) -> i64>) -> Self {
        self.element_order_key = element_order_key;
        self
    }

    pub fn with_precedence_hooks(
        mut self,
        duration_fn: Option<fn(&S, E) -> usize>,
        successors_fn: Option<fn(&S, E, &mut Vec<E>)>,
    ) -> Self {
        self.precedence_duration_fn = duration_fn;
        self.precedence_successors_fn = successors_fn;
        self
    }
}

impl<S, E> ListSourceAccess<S> for ListRegretInsertionPhase<S, E>
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

impl<S, E> RegretAccess<S> for ListRegretInsertionPhase<S, E>
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

    fn list_len(&self, solution: &S, entity_index: usize) -> usize {
        (self.list_len)(solution, entity_index)
    }

    fn insert_element(
        &self,
        solution: &mut S,
        entity_index: usize,
        position: usize,
        element: Self::Element,
    ) {
        (self.list_insert)(solution, entity_index, position, element);
    }

    fn remove_element(&self, solution: &mut S, entity_index: usize, position: usize) {
        let _ = (self.list_remove)(solution, entity_index, position);
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        crate::list_placement::owner_restriction(
            self.element_owner_fn,
            solution,
            entity_count,
            element,
        )
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        self.element_order_key
            .map_or(0, |order_key| order_key(solution, *element))
    }

    fn precedence_duration(&self, solution: &S, element: &Self::Element) -> Option<usize> {
        self.precedence_duration_fn
            .map(|duration| duration(solution, *element))
    }

    fn extend_precedence_successor_source_indices(
        &self,
        solution: &S,
        element: &Self::Element,
        source_index: &RuntimeListSourceIndex<Self::Element>,
        successors: &mut Vec<usize>,
    ) -> bool {
        let Some(successor_fn) = self.precedence_successors_fn else {
            return false;
        };
        let mut values = Vec::new();
        successor_fn(solution, *element, &mut values);
        successors.extend(values.into_iter().filter_map(|successor| {
            source_index.source_index_for_key((self.element_source_key)(solution, &successor))
        }));
        true
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        let binding = bind_runtime_list_source(self, solver_scope.working_solution()).unwrap_or_else(
            |error| {
                panic!(
                    "ListRegretInsertionPhase source binding failed before phase execution: {error:?}"
                            )
            },
                            );
        let source_index = binding.into_source_index();
        let unassigned = unassigned_from_current_assignment(
            self,
            &source_index,
            solver_scope.working_solution(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "ListRegretInsertionPhase assignment refresh failed before phase execution: {error:?}"
            )
                    });
        run_regret(
            self,
            &source_index,
            &unassigned,
            StepControlPolicy::CompleteMandatoryConstruction,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListRegretInsertion"
    }
}

#[cfg(test)]
#[path = "regret/tests.rs"]
mod tests;
