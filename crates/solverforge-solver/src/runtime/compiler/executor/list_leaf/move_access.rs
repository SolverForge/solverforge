//! Compact list access retained by emitted runtime moves.
//!
//! Cursor generation keeps the complete compiled slot, including distance
//! meters and construction metadata. Emitted moves need only mutation,
//! identity, tabu, ownership, and ruin-recreate access, so they retain this
//! meter-free carrier instead of cloning the complete slot per candidate.

use std::fmt;

use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};

use crate::builder::context::list_access::{ListAccess, ListAccessError};
use crate::builder::context::{
    OwnershipPolicy, PrecedencePolicy, RuntimeListElement, RuntimeListSlot,
};
use crate::heuristic::r#move::list_kernel::{
    ListChangeAccess, ListMoveAccess, ListRangeAccess, ListReverseAccess, ListRuinAccess,
    ListSwapAccess, ListWindowAccess,
};
use crate::heuristic::r#move::metadata::{
    encode_option_debug, encode_runtime_dynamic_list_source, NONE_ID,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::precedence_route::{
    build_precedence_route_graph, PrecedenceRouteGraph,
};
use crate::list_placement::OwnerRestriction;

#[derive(Clone)]
pub(crate) enum RuntimeListMoveAccess<S, V> {
    Static(StaticRuntimeListMoveAccess<S, V>),
    Dynamic {
        slot: DynamicListVariableSlot<S>,
        ownership_policy: OwnershipPolicy,
        precedence_policy: PrecedencePolicy,
    },
}

pub(crate) struct StaticRuntimeListMoveAccess<S, V> {
    entity_type_name: &'static str,
    variable_name: &'static str,
    descriptor_index: usize,
    entity_count: fn(&S) -> usize,
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    element_owner: Option<fn(&S, &V) -> Option<usize>>,
    precedence_successors: Option<fn(&S, V, &mut Vec<V>)>,
}

impl<S, V> Copy for StaticRuntimeListMoveAccess<S, V> {}

impl<S, V> Clone for StaticRuntimeListMoveAccess<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> RuntimeListMoveAccess<S, V> {
    pub(super) fn from_slot<DM, IDM>(slot: &RuntimeListSlot<S, V, DM, IDM>) -> Self
    where
        S: Clone + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
        DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
        IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    {
        match slot {
            RuntimeListSlot::Static { slot, .. } => Self::Static(StaticRuntimeListMoveAccess {
                entity_type_name: slot.entity_type_name,
                variable_name: slot.variable_name,
                descriptor_index: slot.descriptor_index,
                entity_count: slot.entity_count,
                element_count: slot.element_count,
                index_to_element: slot.index_to_element,
                list_len: slot.list_len,
                list_get: slot.list_get,
                list_remove: slot.list_remove,
                list_insert: slot.list_insert,
                list_set: slot.list_set,
                list_reverse: slot.list_reverse,
                sublist_remove: slot.sublist_remove,
                sublist_insert: slot.sublist_insert,
                element_owner: slot.element_owner_fn,
                precedence_successors: slot.precedence_successors_fn,
            }),
            RuntimeListSlot::Dynamic(dynamic) => Self::Dynamic {
                slot: (**dynamic).clone(),
                ownership_policy: slot.ownership_policy(),
                precedence_policy: slot.precedence_policy(),
            },
        }
    }

    fn entity_count(&self, solution: &S) -> usize
    where
        S: Clone + Send + Sync + 'static,
    {
        match self {
            Self::Static(access) => (access.entity_count)(solution),
            Self::Dynamic { slot, .. } => ListAccess::entity_count(slot, solution),
        }
    }

    fn ownership_policy(&self) -> OwnershipPolicy {
        match self {
            Self::Static(access) => {
                if access.element_owner.is_some() {
                    OwnershipPolicy::ExplicitStaticProvider
                } else {
                    OwnershipPolicy::DeclaredUnrestricted
                }
            }
            Self::Dynamic {
                ownership_policy, ..
            } => *ownership_policy,
        }
    }

    fn precedence_policy(&self) -> PrecedencePolicy {
        match self {
            Self::Static(access) => {
                if access.precedence_successors.is_some() {
                    PrecedencePolicy::SuccessorsOnly
                } else {
                    PrecedencePolicy::Absent
                }
            }
            Self::Dynamic {
                precedence_policy, ..
            } => *precedence_policy,
        }
    }
}

impl<S, V> fmt::Debug for RuntimeListMoveAccess<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(access) => formatter
                .debug_struct("RuntimeListMoveAccess::Static")
                .field("entity_type_name", &access.entity_type_name)
                .field("variable_name", &access.variable_name)
                .field("descriptor_index", &access.descriptor_index)
                .finish(),
            Self::Dynamic { slot, .. } => formatter
                .debug_tuple("RuntimeListMoveAccess::Dynamic")
                .field(slot)
                .finish(),
        }
    }
}

fn static_element_ref<V>(element: &RuntimeListElement<V>) -> &V {
    match element {
        RuntimeListElement::Static(value) => value,
        RuntimeListElement::Dynamic(_) => {
            panic!("runtime list move element does not belong to its selected slot")
        }
    }
}

fn static_element<V>(element: RuntimeListElement<V>) -> V {
    match element {
        RuntimeListElement::Static(value) => value,
        RuntimeListElement::Dynamic(_) => {
            panic!("runtime list move element does not belong to its selected slot")
        }
    }
}

fn dynamic_element(element: &RuntimeListElement<impl Sized>) -> usize {
    match element {
        RuntimeListElement::Dynamic(value) => *value,
        RuntimeListElement::Static(_) => {
            panic!("runtime list move element does not belong to its selected slot")
        }
    }
}

impl<S, V> ListMoveAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    type Element = RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Static(access) => access.descriptor_index,
            Self::Dynamic { slot, .. } => ListAccess::descriptor_index(slot),
        }
    }

    fn variable_name(&self) -> &'static str {
        match self {
            Self::Static(access) => access.variable_name,
            Self::Dynamic { slot, .. } => ListAccess::variable_name(slot),
        }
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        match self {
            Self::Static(access) => (access.list_len)(solution, entity),
            Self::Dynamic { slot, .. } => ListAccess::list_len(slot, solution, entity),
        }
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        match self {
            Self::Static(access) => {
                (access.list_get)(solution, entity, position).map(RuntimeListElement::Static)
            }
            Self::Dynamic { slot, .. } => ListAccess::list_get(slot, solution, entity, position)
                .map(RuntimeListElement::Dynamic),
        }
    }

    fn tabu_value_id(&self, _solution: &S, value: Option<&Self::Element>) -> u64 {
        let Some(value) = value else {
            return NONE_ID;
        };
        match self {
            Self::Static(_) => encode_option_debug(Some(static_element_ref(value))),
            Self::Dynamic { .. } => encode_runtime_dynamic_list_source(dynamic_element(value)),
        }
    }
}

impl<S, V> ListChangeAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        match self {
            Self::Static(access) => {
                (access.list_remove)(solution, entity, position).map(RuntimeListElement::Static)
            }
            Self::Dynamic { slot, .. } => ListAccess::list_remove(slot, solution, entity, position)
                .map(RuntimeListElement::Dynamic),
        }
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        match self {
            Self::Static(access) => {
                (access.list_insert)(solution, entity, position, static_element(value));
            }
            Self::Dynamic { slot, .. } => {
                ListAccess::list_insert(slot, solution, entity, position, dynamic_element(&value));
            }
        }
    }
}

impl<S, V> ListSwapAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static(access) => {
                (access.list_set)(solution, entity, position, static_element(value));
                Ok(())
            }
            Self::Dynamic { slot, .. } => {
                ListAccess::list_set(slot, solution, entity, position, dynamic_element(&value))
            }
        }
    }
}

impl<S, V> ListRangeAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    type Element = RuntimeListElement<V>;

    fn descriptor_index(&self) -> usize {
        ListMoveAccess::descriptor_index(self)
    }

    fn variable_name(&self) -> &'static str {
        ListMoveAccess::variable_name(self)
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        ListMoveAccess::list_len(self, solution, entity)
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        ListMoveAccess::list_get(self, solution, entity, position)
    }

    fn tabu_value_id(&self, solution: &S, value: Option<&Self::Element>) -> u64 {
        ListMoveAccess::tabu_value_id(self, solution, value)
    }
}

impl<S, V> ListReverseAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static(access) => {
                (access.list_reverse)(solution, entity, start, end);
                Ok(())
            }
            Self::Dynamic { slot, .. } => {
                ListAccess::list_reverse(slot, solution, entity, start, end)
            }
        }
    }
}

impl<S, V> ListWindowAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        match self {
            Self::Static(access) => Ok((access.sublist_remove)(solution, entity, start, end)
                .into_iter()
                .map(RuntimeListElement::Static)
                .collect()),
            Self::Dynamic { slot, .. } => Ok(ListAccess::sublist_remove(
                slot, solution, entity, start, end,
            )?
            .into_iter()
            .map(RuntimeListElement::Dynamic)
            .collect()),
        }
    }

    fn sublist_insert(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        values: Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static(access) => {
                (access.sublist_insert)(
                    solution,
                    entity,
                    position,
                    values.into_iter().map(static_element).collect(),
                );
                Ok(())
            }
            Self::Dynamic { slot, .. } => ListAccess::sublist_insert(
                slot,
                solution,
                entity,
                position,
                values.iter().map(dynamic_element).collect(),
            ),
        }
    }
}

impl<S, V> ListRuinAccess<S> for RuntimeListMoveAccess<S, V>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    fn entity_count(&self, solution: &S) -> usize {
        self.entity_count(solution)
    }

    fn list_remove(&self, solution: &mut S, entity: usize, position: usize) -> Self::Element {
        ListChangeAccess::list_remove(self, solution, entity, position)
            .expect("runtime list ruin source position must be valid")
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        ListChangeAccess::list_insert(self, solution, entity, position, value);
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
        let owner = match self {
            Self::Static(access) => (access.element_owner.expect("explicit owner"))(
                solution,
                static_element_ref(element),
            ),
            Self::Dynamic { slot, .. } => {
                ListAccess::element_owner(slot, solution, &dynamic_element(element))
                    .expect("runtime list ruin owner access requires validated metadata")
            }
        };
        match owner {
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
        let elements = match self {
            Self::Static(access) => (0..(access.element_count)(solution))
                .map(|index| RuntimeListElement::Static((access.index_to_element)(solution, index)))
                .collect(),
            Self::Dynamic { slot, .. } => (0..ListAccess::element_count(slot, solution))
                .map(|index| {
                    ListAccess::index_to_element(slot, solution, index)
                        .map(RuntimeListElement::Dynamic)
                })
                .collect::<Option<Vec<_>>>()?,
        };
        let graph = build_precedence_route_graph(
            solution,
            &elements,
            |solution, element, successors| match self {
                Self::Static(access) => {
                    let mut values = Vec::new();
                    (access.precedence_successors.expect("validated successors"))(
                        solution,
                        static_element(element),
                        &mut values,
                    );
                    successors.extend(values.into_iter().map(RuntimeListElement::Static));
                }
                Self::Dynamic { slot, .. } => {
                    let mut values = Vec::new();
                    ListAccess::extend_precedence_successors(
                        slot,
                        solution,
                        dynamic_element(&element),
                        &mut values,
                    )
                    .expect("runtime list ruin precedence requires validated successors");
                    successors.extend(values.into_iter().map(RuntimeListElement::Dynamic));
                }
            },
            |solution| self.entity_count(solution),
            |solution, entity| ListMoveAccess::list_len(self, solution, entity),
            |solution, entity, position| ListMoveAccess::list_get(self, solution, entity, position),
        );
        Some((elements, graph))
    }
}
