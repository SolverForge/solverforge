//! Compile-time binding metadata for [`RuntimeListSlot`](super::RuntimeListSlot).
//!
//! Physical list access stays in `runtime_list.rs`; this companion keeps
//! descriptor identity, frozen savings policy, and structural capability
//! validation together so execution has no schema discovery work.

use solverforge_core::domain::DynamicListVariableSlot;

use super::list_access::ListAccessCapability;
use super::runtime_list_metadata_policy::StaticListMetadataBindings;
use super::runtime_list_route_policy::{RuntimeDynamicListSlot, StaticRouteBindings};
use super::{
    ConstructionOrderPolicy, ListVariableSlot, OwnershipPolicy, PrecedencePolicy,
    RouteFeasibilityPolicy, RouteReadPolicy, RouteReplacePolicy, RuntimeListSlot,
    RuntimeScalarSlotId, SavingsMetricClassPolicy,
};

impl<S, V, DM, IDM> RuntimeListSlot<S, V, DM, IDM> {
    /// Binds a typed list slot to all canonical route/savings policies.
    pub(crate) fn from_static(
        slot: ListVariableSlot<S, V, DM, IDM>,
        variable_index: usize,
    ) -> Self {
        let route_bindings = StaticRouteBindings::from_slot(&slot);
        let metadata_bindings = StaticListMetadataBindings::from_slot(&slot);
        Self::Static {
            slot,
            variable_index,
            route_bindings,
            metadata_bindings,
        }
    }

    /// Binds a dynamic list slot to the schema capability that was actually
    /// declared. There is no entity-identity policy for host-language
    /// metadata.
    pub(crate) fn from_dynamic(slot: DynamicListVariableSlot<S>) -> Self {
        Self::Dynamic(RuntimeDynamicListSlot::new(slot))
    }

    pub(crate) fn savings_metric_class_policy(&self) -> SavingsMetricClassPolicy {
        match self {
            Self::Static { route_bindings, .. } => route_bindings.metric_class_policy,
            Self::Dynamic(slot) => slot.savings_metric_class_policy(),
        }
    }

    pub(crate) fn route_read_policy(&self) -> RouteReadPolicy {
        match self {
            Self::Static { route_bindings, .. } => route_bindings.read_policy,
            Self::Dynamic(slot) => slot.route_read_policy(),
        }
    }

    pub(crate) fn route_replace_policy(&self) -> RouteReplacePolicy {
        match self {
            Self::Static { route_bindings, .. } => route_bindings.replace_policy,
            Self::Dynamic(slot) => slot.route_replace_policy(),
        }
    }

    pub(crate) fn route_feasibility_policy(&self) -> RouteFeasibilityPolicy {
        match self {
            Self::Static { route_bindings, .. } => route_bindings.feasibility_policy,
            Self::Dynamic(slot) => slot.route_feasibility_policy(),
        }
    }

    pub(crate) fn ownership_policy(&self) -> OwnershipPolicy {
        match self {
            Self::Static {
                metadata_bindings, ..
            } => metadata_bindings.ownership_policy,
            Self::Dynamic(slot) => slot.ownership_policy(),
        }
    }

    pub(crate) fn construction_order_policy(&self) -> ConstructionOrderPolicy {
        match self {
            Self::Static {
                metadata_bindings, ..
            } => metadata_bindings.construction_order_policy,
            Self::Dynamic(slot) => slot.construction_order_policy(),
        }
    }

    pub(crate) fn precedence_policy(&self) -> PrecedencePolicy {
        match self {
            Self::Static {
                metadata_bindings, ..
            } => metadata_bindings.precedence_policy,
            Self::Dynamic(slot) => slot.precedence_policy(),
        }
    }

    pub(crate) fn identity(&self) -> RuntimeScalarSlotId {
        match self {
            Self::Static {
                slot,
                variable_index,
                ..
            } => RuntimeScalarSlotId {
                descriptor_index: slot.descriptor_index,
                variable_index: *variable_index,
                entity_class: slot.entity_type_name.into(),
                variable_name: slot.variable_name.into(),
                dynamic_identity: None,
            },
            Self::Dynamic(slot) => RuntimeScalarSlotId {
                descriptor_index: slot.descriptor_index(),
                variable_index: slot.descriptor_variable_index(),
                entity_class: slot.entity_type_name.into(),
                variable_name: slot.variable_name.into(),
                dynamic_identity: Some((slot.entity, slot.variable)),
            },
        }
    }

    /// Answers a structural access/metadata requirement without opening a
    /// cursor or touching a solution. The compiler calls this once while
    /// freezing a graph; execution never substitutes a weaker operation.
    pub(crate) fn supports(&self, capability: ListAccessCapability) -> bool {
        match self {
            Self::Static {
                route_bindings,
                metadata_bindings,
                ..
            } => match capability {
                ListAccessCapability::Set
                | ListAccessCapability::Reverse
                | ListAccessCapability::Sublist
                | ListAccessCapability::CrossPositionDistance
                | ListAccessCapability::IntraPositionDistance => true,
                ListAccessCapability::Replace => route_bindings.supports_replace(),
                ListAccessCapability::ElementOwner => {
                    metadata_bindings.ownership_policy.is_explicit()
                }
                ListAccessCapability::ConstructionOrderKey => {
                    metadata_bindings.construction_order_policy.is_explicit()
                }
                ListAccessCapability::Precedence => {
                    metadata_bindings.precedence_policy.is_explicit()
                }
                ListAccessCapability::Route => route_bindings.supports_route(),
                ListAccessCapability::Savings => route_bindings.supports_savings(),
            },
            Self::Dynamic(slot) => {
                let access = slot.access_capabilities();
                let metadata = slot.metadata_capabilities().unwrap_or_default();
                match capability {
                    ListAccessCapability::Set => access.set,
                    ListAccessCapability::Replace => slot.route_replace_policy().is_available(),
                    ListAccessCapability::Reverse => access.reverse,
                    ListAccessCapability::Sublist => access.sublist,
                    ListAccessCapability::ElementOwner => slot.ownership_policy().is_explicit(),
                    ListAccessCapability::ConstructionOrderKey => {
                        slot.construction_order_policy().is_explicit()
                    }
                    ListAccessCapability::Precedence => slot.precedence_policy().is_explicit(),
                    ListAccessCapability::CrossPositionDistance => metadata.cross_position_distance,
                    ListAccessCapability::IntraPositionDistance => metadata.intra_position_distance,
                    ListAccessCapability::Route => {
                        slot.route_read_policy().is_available()
                            && slot.route_replace_policy().is_available()
                            && metadata.route
                            && slot.route_feasibility_policy().is_dynamic_explicit()
                    }
                    ListAccessCapability::Savings => {
                        slot.route_replace_policy().is_available()
                            && metadata.savings
                            && slot.savings_metric_class_policy().is_dynamic_explicit()
                    }
                }
            }
        }
    }
}
