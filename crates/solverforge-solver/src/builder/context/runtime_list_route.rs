//! Route and savings adapters for [`RuntimeListSlot`](super::RuntimeListSlot).
//!
//! These adapters intentionally keep route and savings semantics independent.
//! A model may bind the same host callback to both bundles, but the runtime
//! never infers that equality or substitutes one bundle for the other.

use std::fmt;

use solverforge_core::domain::DynamicListVariableSlot;

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::list_access::{
    ListAccessCapability, ListAccessError, RouteAccess, RouteSequenceAccess, SavingsAccess,
};
use super::{ListVariableSlot, RuntimeListSlot};

fn static_error<S, V, DM, IDM>(
    slot: &ListVariableSlot<S, V, DM, IDM>,
    capability: ListAccessCapability,
) -> ListAccessError {
    ListAccessError {
        capability,
        entity_type_name: slot.entity_type_name,
        variable_name: slot.variable_name,
    }
}

impl<S, V, DM, IDM> RouteSequenceAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_route_sequence(&self) -> Result<(), ListAccessError> {
        match self {
            Self::Static {
                slot,
                route_bindings,
                ..
            } => route_bindings
                .supports_replace()
                .then_some(())
                .ok_or_else(|| static_error(slot, ListAccessCapability::Replace)),
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as RouteSequenceAccess<S>>::validate_route_sequence(
                    slot,
                )
            }
        }
    }

    fn route_values(&self, solution: &S, entity: usize) -> Result<Vec<usize>, ListAccessError> {
        match self {
            Self::Static {
                slot,
                route_bindings,
                ..
            } => route_bindings
                .read_policy
                .is_available()
                .then(|| (route_bindings.route_read)(solution, entity))
                .ok_or_else(|| static_error(slot, ListAccessCapability::Route)),
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as RouteSequenceAccess<S>>::route_values(
                    slot, solution, entity,
                )
            }
        }
    }

    fn replace_route(
        &self,
        solution: &mut S,
        entity: usize,
        route: Vec<usize>,
    ) -> Result<(), ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_route_sequence()?;
                (route_bindings.route_replace)(solution, entity, route);
                Ok(())
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as RouteSequenceAccess<S>>::replace_route(
                    slot, solution, entity, route,
                )
            }
        }
    }
}

impl<S, V, DM, IDM> RouteAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_route(&self) -> Result<(), ListAccessError> {
        match self {
            Self::Static {
                slot,
                route_bindings,
                ..
            } => route_bindings
                .supports_route()
                .then_some(())
                .ok_or_else(|| static_error(slot, ListAccessCapability::Route)),
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as RouteAccess<S>>::validate_route(slot)
            }
        }
    }

    fn route_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_route()?;
                Ok((route_bindings.route_depot)(solution, entity))
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as RouteAccess<S>>::route_depot(slot, solution, entity)
            }
        }
    }

    fn route_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_route()?;
                Ok((route_bindings.route_distance)(solution, entity, from, to))
            }
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as RouteAccess<S>>::route_distance(
                slot, solution, entity, from, to,
            ),
        }
    }

    fn route_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_route()?;
                Ok((route_bindings.route_feasible)(solution, entity, route))
            }
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as RouteAccess<S>>::route_feasible(
                slot, solution, entity, route,
            ),
        }
    }
}

impl<S, V, DM, IDM> SavingsAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_savings(&self) -> Result<(), ListAccessError> {
        match self {
            Self::Static {
                slot,
                route_bindings,
                ..
            } => {
                self.validate_route_sequence()?;
                route_bindings
                    .supports_savings()
                    .then_some(())
                    .ok_or_else(|| static_error(slot, ListAccessCapability::Savings))
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as SavingsAccess<S>>::validate_savings(slot)
            }
        }
    }

    fn savings_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_savings()?;
                Ok((route_bindings.savings_depot)(solution, entity))
            }
            Self::Dynamic(slot) => <DynamicListVariableSlot<S> as SavingsAccess<S>>::savings_depot(
                slot, solution, entity,
            ),
        }
    }

    fn savings_metric_class(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_savings()?;
                Ok((route_bindings.savings_metric_class)(solution, entity))
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as SavingsAccess<S>>::savings_metric_class(
                    slot, solution, entity,
                )
            }
        }
    }

    fn savings_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_savings()?;
                Ok((route_bindings.savings_distance)(
                    solution, entity, from, to,
                ))
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as SavingsAccess<S>>::savings_distance(
                    slot, solution, entity, from, to,
                )
            }
        }
    }

    fn savings_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        match self {
            Self::Static { route_bindings, .. } => {
                self.validate_savings()?;
                Ok((route_bindings.savings_feasible)(solution, entity, route))
            }
            Self::Dynamic(slot) => {
                <DynamicListVariableSlot<S> as SavingsAccess<S>>::savings_feasible(
                    slot, solution, entity, route,
                )
            }
        }
    }
}
