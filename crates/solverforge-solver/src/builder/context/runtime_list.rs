//! One physical list-slot carrier for the compiled runtime graph.
//!
//! Static and dynamic list variables retain their native access mechanisms,
//! but every runtime kernel sees the same [`RuntimeListSlot`] and
//! [`RuntimeListElement`] protocol.  This is deliberately not a second
//! selector implementation: future construction and neighborhood cursors are
//! generic over `ListAccess` and receive this carrier for both source kinds.

use solverforge_core::domain::DynamicListVariableSlot;
use std::fmt;

use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::runtime_list_metadata_policy::StaticListMetadataBindings;
use super::runtime_list_route_policy::{RuntimeDynamicListSlot, StaticRouteBindings};
use super::{
    list_access::{ListAccess, ListAccessCapability, ListAccessError},
    ListVariableSlot,
};

/// Element payload carried by a runtime list move.
///
/// The tag is an internal invariant established by the selected slot.  It
/// prevents a mixed static/dynamic model from accidentally treating a native
/// element as a dynamic index (or vice versa) while still allowing one list
/// kernel to operate over both kinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimeListElement<V> {
    Static(V),
    Dynamic(usize),
}

/// The declaration-resolved list slot consumed by the compiled runtime graph.
///
/// A static slot keeps its real value type and distance meters.  A dynamic
/// slot keeps the bridge-owned direct access/metadata objects.  The outer enum
/// is only physical dispatch; candidate order, construction, move legality,
/// mutation, and notifications are owned by the shared runtime kernels.
#[derive(Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "runtime list slots stay value-owned without per-slot heap indirection"
)]
pub(crate) enum RuntimeListSlot<S, V, DM, IDM> {
    Static {
        slot: ListVariableSlot<S, V, DM, IDM>,
        /// Descriptor-local variable coordinate resolved exactly once from
        /// the authoritative solution descriptor.
        variable_index: usize,
        /// Resolved typed route/savings sources. Every successful static
        /// kernel call uses its non-null function pointer directly.
        route_bindings: StaticRouteBindings<S>,
        /// Resolved typed optional metadata. Its policies preserve the public
        /// meaning of absent ownership, order, and precedence hooks.
        metadata_bindings: StaticListMetadataBindings<S, V>,
    },
    Dynamic(RuntimeDynamicListSlot<S>),
}

fn element_mismatch() -> ! {
    panic!("runtime list move element does not belong to its selected list slot")
}

fn static_element<V>(element: RuntimeListElement<V>) -> V {
    match element {
        RuntimeListElement::Static(value) => value,
        RuntimeListElement::Dynamic(_) => element_mismatch(),
    }
}

fn dynamic_element<V>(element: RuntimeListElement<V>) -> usize {
    match element {
        RuntimeListElement::Dynamic(value) => value,
        RuntimeListElement::Static(_) => element_mismatch(),
    }
}

fn missing_metadata(
    entity_type_name: &'static str,
    variable_name: &'static str,
    capability: ListAccessCapability,
) -> ListAccessError {
    ListAccessError {
        capability,
        entity_type_name,
        variable_name,
    }
}

impl<S, V, DM, IDM> ListAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Element = RuntimeListElement<V>;

    fn entity_type_name(&self) -> &'static str {
        match self {
            Self::Static { slot, .. } => slot.entity_type_name(),
            Self::Dynamic(slot) => slot.entity_type_name(),
        }
    }

    fn variable_name(&self) -> &'static str {
        match self {
            Self::Static { slot, .. } => slot.variable_name(),
            Self::Dynamic(slot) => slot.variable_name(),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Static { slot, .. } => slot.descriptor_index(),
            Self::Dynamic(slot) => slot.descriptor_index(),
        }
    }

    fn entity_count(&self, solution: &S) -> usize {
        match self {
            Self::Static { slot, .. } => slot.entity_count(solution),
            Self::Dynamic(slot) => slot.entity_count(solution),
        }
    }

    fn element_count(&self, solution: &S) -> usize {
        match self {
            Self::Static { slot, .. } => slot.element_count(solution),
            Self::Dynamic(slot) => slot.element_count(solution),
        }
    }

    fn index_to_element(&self, solution: &S, element_index: usize) -> Option<Self::Element> {
        match self {
            Self::Static { slot, .. } => slot
                .index_to_element(solution, element_index)
                .map(RuntimeListElement::Static),
            Self::Dynamic(slot) => slot
                .index_to_element(solution, element_index)
                .map(RuntimeListElement::Dynamic),
        }
    }

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        match self {
            Self::Static { slot, .. } => match element {
                RuntimeListElement::Static(value) => slot.element_source_key(solution, value),
                RuntimeListElement::Dynamic(_) => element_mismatch(),
            },
            Self::Dynamic(_) => match element {
                RuntimeListElement::Dynamic(value) => *value,
                RuntimeListElement::Static(_) => element_mismatch(),
            },
        }
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        match self {
            Self::Static { slot, .. } => slot
                .assigned_elements(solution)
                .into_iter()
                .map(RuntimeListElement::Static)
                .collect(),
            Self::Dynamic(slot) => slot
                .assigned_elements(solution)
                .into_iter()
                .map(RuntimeListElement::Dynamic)
                .collect(),
        }
    }

    fn list_len(&self, solution: &S, entity: usize) -> usize {
        match self {
            Self::Static { slot, .. } => slot.list_len(solution, entity),
            Self::Dynamic(slot) => slot.list_len(solution, entity),
        }
    }

    fn list_get(&self, solution: &S, entity: usize, position: usize) -> Option<Self::Element> {
        match self {
            Self::Static { slot, .. } => slot
                .list_get(solution, entity, position)
                .map(RuntimeListElement::Static),
            Self::Dynamic(slot) => slot
                .list_get(solution, entity, position)
                .map(RuntimeListElement::Dynamic),
        }
    }

    fn list_insert(&self, solution: &mut S, entity: usize, position: usize, value: Self::Element) {
        match self {
            Self::Static { slot, .. } => {
                slot.list_insert(solution, entity, position, static_element(value));
            }
            Self::Dynamic(slot) => {
                slot.list_insert(solution, entity, position, dynamic_element(value));
            }
        }
    }

    fn list_remove(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
    ) -> Option<Self::Element> {
        match self {
            Self::Static { slot, .. } => slot
                .list_remove(solution, entity, position)
                .map(RuntimeListElement::Static),
            Self::Dynamic(slot) => slot
                .list_remove(solution, entity, position)
                .map(RuntimeListElement::Dynamic),
        }
    }

    fn list_set(
        &self,
        solution: &mut S,
        entity: usize,
        position: usize,
        value: Self::Element,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static { slot, .. } => {
                slot.list_set(solution, entity, position, static_element(value))
            }
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as ListAccess<S>>::list_set(
                slot,
                solution,
                entity,
                position,
                dynamic_element(value),
            ),
        }
    }

    fn list_reverse(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static { slot, .. } => slot.list_reverse(solution, entity, start, end),
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as ListAccess<S>>::list_reverse(
                slot, solution, entity, start, end,
            ),
        }
    }

    fn sublist_remove(
        &self,
        solution: &mut S,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Result<Vec<Self::Element>, ListAccessError> {
        match self {
            Self::Static { slot, .. } => Ok(slot
                .sublist_remove(solution, entity, start, end)?
                .into_iter()
                .map(RuntimeListElement::Static)
                .collect()),
            Self::Dynamic(slot) => Ok(
                <DynamicListVariableSlot<S> as ListAccess<S>>::sublist_remove(
                    slot, solution, entity, start, end,
                )?
                .into_iter()
                .map(RuntimeListElement::Dynamic)
                .collect(),
            ),
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
            Self::Static { slot, .. } => slot.sublist_insert(
                solution,
                entity,
                position,
                values.into_iter().map(static_element).collect(),
            ),
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as ListAccess<S>>::sublist_insert(
                slot,
                solution,
                entity,
                position,
                values.into_iter().map(dynamic_element).collect(),
            ),
        }
    }

    fn element_owner(
        &self,
        solution: &S,
        element: &Self::Element,
    ) -> Result<Option<usize>, ListAccessError> {
        match self {
            Self::Static {
                metadata_bindings, ..
            } => match element {
                RuntimeListElement::Static(value) => {
                    Ok((metadata_bindings.element_owner)(solution, value))
                }
                RuntimeListElement::Dynamic(_) => element_mismatch(),
            },
            Self::Dynamic(slot) => match element {
                RuntimeListElement::Dynamic(value) => {
                    if slot.ownership_policy().is_explicit() {
                        slot.element_owner(solution, value)
                    } else {
                        Ok(None)
                    }
                }
                RuntimeListElement::Static(_) => element_mismatch(),
            },
        }
    }

    fn construction_order_key(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<i64, ListAccessError> {
        match self {
            Self::Static {
                metadata_bindings, ..
            } => Ok((metadata_bindings.construction_order)(
                solution,
                static_element(element),
            )),
            Self::Dynamic(slot) => {
                if slot.construction_order_policy().is_explicit() {
                    slot.construction_order_key(solution, dynamic_element(element))
                } else {
                    Ok(0)
                }
            }
        }
    }

    fn extend_precedence_successors(
        &self,
        solution: &S,
        element: Self::Element,
        successors: &mut Vec<Self::Element>,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static {
                slot,
                metadata_bindings,
                ..
            } => {
                if !metadata_bindings.precedence_policy.has_successors() {
                    return Err(missing_metadata(
                        slot.entity_type_name,
                        slot.variable_name,
                        ListAccessCapability::Precedence,
                    ));
                }
                let mut values = Vec::new();
                (metadata_bindings.precedence_successors)(
                    solution,
                    static_element(element),
                    &mut values,
                );
                successors.extend(values.into_iter().map(RuntimeListElement::Static));
                Ok(())
            }
            Self::Dynamic(slot) => {
                if !slot.precedence_policy().has_successors() {
                    return Err(missing_metadata(
                        slot.entity_type_name,
                        slot.variable_name,
                        ListAccessCapability::Precedence,
                    ));
                }
                let mut values = Vec::new();
                slot.extend_precedence_successors(solution, dynamic_element(element), &mut values)?;
                successors.extend(values.into_iter().map(RuntimeListElement::Dynamic));
                Ok(())
            }
        }
    }

    fn precedence_duration(
        &self,
        solution: &S,
        element: Self::Element,
    ) -> Result<usize, ListAccessError> {
        match self {
            Self::Static {
                slot,
                metadata_bindings,
                ..
            } => {
                if !metadata_bindings.precedence_policy.is_explicit() {
                    return Err(missing_metadata(
                        slot.entity_type_name,
                        slot.variable_name,
                        ListAccessCapability::Precedence,
                    ));
                }
                Ok((metadata_bindings.precedence_duration)(
                    solution,
                    static_element(element),
                ))
            }
            Self::Dynamic(slot) => {
                if slot.precedence_policy().is_explicit() {
                    slot.precedence_duration(solution, dynamic_element(element))
                } else {
                    Err(missing_metadata(
                        slot.entity_type_name,
                        slot.variable_name,
                        ListAccessCapability::Precedence,
                    ))
                }
            }
        }
    }

    fn cross_position_distance(
        &self,
        solution: &S,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        match self {
            Self::Static { slot, .. } => slot.cross_position_distance(
                solution,
                from_entity,
                from_position,
                to_entity,
                to_position,
            ),
            Self::Dynamic(slot) => slot.cross_position_distance(
                solution,
                from_entity,
                from_position,
                to_entity,
                to_position,
            ),
        }
    }

    fn intra_position_distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Result<f64, ListAccessError> {
        match self {
            Self::Static { slot, .. } => Ok(slot.intra_distance_meter.distance(
                solution,
                entity,
                from_position,
                entity,
                to_position,
            )),
            Self::Dynamic(slot) => {
                slot.intra_position_distance(solution, entity, from_position, to_position)
            }
        }
    }
}

/// The one position-metric adapter for compiled list slots.
///
/// Native list declarations retain their established intra metric as a
/// cross-entity meter and are invoked with the same source and destination
/// entity. Dynamic slots never use that generic meter: they delegate to their
/// explicitly bound dynamic metadata. Compilation validates the corresponding
/// capability before a K-opt consumer can call this adapter.
impl<S, V, DM, IDM> ListPositionDistanceMeter<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> f64 {
        ListAccess::intra_position_distance(self, solution, entity, from_position, to_position)
            .expect("compiled list position distance requires a validated slot capability")
    }
}
