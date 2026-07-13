//! Access boundary for shared list ruin-and-recreate mechanics.

use std::fmt;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{list_access::ListAccess, RuntimeListSlot};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::precedence_route::{
    build_precedence_route_graph, PrecedenceRouteGraph, PrecedenceRouteHooks,
};
use crate::list_placement::{owner_restriction, OwnerRestriction};

use super::ListMoveAccess;

/// Physical list operations and optional metadata consumed by ruin/recreate.
///
/// The static facade keeps its function-pointer contract in
/// [`StaticListRuinAccess`]. A future runtime carrier implements this trait
/// directly over `RuntimeListSlot`; the algorithm never needs a static
/// selector fallback or an erased element representation.
pub(crate) trait ListRuinAccess<S>: ListMoveAccess<S> {
    fn entity_count(&self, solution: &S) -> usize;
    fn list_remove(&self, solution: &mut S, entity: usize, position: usize) -> Self::Element;
    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element);
    fn has_owner_binding(&self) -> bool;
    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction;
    fn recreate_precedence_graph(
        &self,
        solution: &S,
    ) -> Option<(Vec<Self::Element>, PrecedenceRouteGraph)>;
}

/// Static function-pointer access retained by the public `ListRuinMove`.
pub(crate) struct StaticListRuinAccess<S, V> {
    pub(crate) entity_count: fn(&S) -> usize,
    pub(crate) list_len: fn(&S, usize) -> usize,
    pub(crate) list_get: fn(&S, usize, usize) -> Option<V>,
    pub(crate) list_remove: fn(&mut S, usize, usize) -> V,
    pub(crate) list_insert: fn(&mut S, usize, usize, V),
    pub(crate) element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    pub(crate) precedence_element_count: Option<fn(&S) -> usize>,
    pub(crate) precedence_index_to_element: Option<fn(&S, usize) -> V>,
    pub(crate) precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    pub(crate) variable_name: &'static str,
    pub(crate) descriptor_index: usize,
}

impl<S, V> Copy for StaticListRuinAccess<S, V> {}

impl<S, V> Clone for StaticListRuinAccess<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> fmt::Debug for StaticListRuinAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StaticListRuinAccess")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .field("has_element_owner", &self.element_owner_fn.is_some())
            .field(
                "has_precedence_successors",
                &self.precedence_successors_fn.is_some(),
            )
            .finish()
    }
}

impl<S, V> ListMoveAccess<S> for StaticListRuinAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    type Element = V;

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn variable_name(&self) -> &'static str {
        self.variable_name
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        (self.list_len)(solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        (self.list_get)(solution, entity, position)
    }
}

impl<S, V> ListRuinAccess<S> for StaticListRuinAccess<S, V>
where
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn list_remove(&self, solution: &mut S, entity: usize, position: usize) -> Self::Element {
        (self.list_remove)(solution, entity, position)
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        (self.list_insert)(solution, entity, position, value);
    }

    fn has_owner_binding(&self) -> bool {
        self.element_owner_fn.is_some()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        owner_restriction(self.element_owner_fn, solution, entity_count, element)
    }

    fn recreate_precedence_graph(
        &self,
        solution: &S,
    ) -> Option<(Vec<Self::Element>, PrecedenceRouteGraph)> {
        let element_count = self.precedence_element_count?;
        let index_to_element = self.precedence_index_to_element?;
        let fixed_successors = self.precedence_successors_fn?;
        let elements = (0..element_count(solution))
            .map(|index| index_to_element(solution, index))
            .collect::<Vec<_>>();
        let hooks = PrecedenceRouteHooks::new(
            fixed_successors,
            self.entity_count,
            self.list_len,
            self.list_get,
        );
        let graph = hooks.build_graph_with_elements(solution, &elements);
        Some((elements, graph))
    }
}

/// Runtime list access reuses the same ruin/recreate mechanics as the static
/// public facade. Ownership is explicit only when the compiled slot declared
/// it; successors-only precedence remains sufficient for recreate pruning.
impl<S, V, DM, IDM> ListRuinAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn list_remove(&self, solution: &mut S, entity: usize, position: usize) -> Self::Element {
        ListAccess::list_remove(self, solution, entity, position)
            .expect("runtime list ruin source position must be valid")
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        ListAccess::list_insert(self, solution, entity, position, value);
    }

    fn has_owner_binding(&self) -> bool {
        self.ownership_policy().is_explicit()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        if !self.has_owner_binding() {
            return OwnerRestriction::Unrestricted;
        }
        match ListAccess::element_owner(self, solution, element)
            .expect("runtime list ruin owner access requires validated explicit metadata")
        {
            Some(owner) if owner < entity_count => OwnerRestriction::Fixed(owner),
            Some(_) => OwnerRestriction::Invalid,
            None => OwnerRestriction::Unrestricted,
        }
    }

    fn recreate_precedence_graph(
        &self,
        solution: &S,
    ) -> Option<(Vec<Self::Element>, PrecedenceRouteGraph)> {
        if !self.precedence_policy().has_successors() {
            return None;
        }
        let elements = (0..ListAccess::element_count(self, solution))
            .map(|index| ListAccess::index_to_element(self, solution, index))
            .collect::<Option<Vec<_>>>()?;
        let graph = build_precedence_route_graph(
            solution,
            &elements,
            |solution, element, successors| {
                ListAccess::extend_precedence_successors(self, solution, element, successors)
                    .expect("runtime list ruin precedence access requires validated successors")
            },
            |solution| ListAccess::entity_count(self, solution),
            |solution, entity| ListAccess::list_len(self, solution, entity),
            |solution, entity, position| ListAccess::list_get(self, solution, entity, position),
        );
        Some((elements, graph))
    }
}
