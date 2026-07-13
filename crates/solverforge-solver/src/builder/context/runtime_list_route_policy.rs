//! Frozen route/savings bindings for the shared runtime list carrier.

use std::fmt;
use std::ops::Deref;

use solverforge_core::domain::DynamicListVariableSlot;

use super::runtime_list_policy::{
    ConstructionOrderPolicy, OwnershipPolicy, PrecedencePolicy, RouteFeasibilityPolicy,
    RouteReadPolicy, RouteReplacePolicy, SavingsMetricClassPolicy,
};
use super::ListVariableSlot;

fn unavailable_route_read<S>(_: &S, _: usize) -> Vec<usize> {
    panic!("compiled route read was invoked without an available route-read policy")
}

fn unavailable_route_replace<S>(_: &mut S, _: usize, _: Vec<usize>) {
    panic!("compiled route replacement was invoked without an available replacement policy")
}

fn unavailable_route_depot<S>(_: &S, _: usize) -> usize {
    panic!("compiled route depot was invoked without an available route bundle")
}

fn unavailable_route_distance<S>(_: &S, _: usize, _: usize, _: usize) -> i64 {
    panic!("compiled route distance was invoked without an available route bundle")
}

fn declared_always_feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
    true
}

fn unavailable_savings_depot<S>(_: &S, _: usize) -> usize {
    panic!("compiled savings depot was invoked without an available savings bundle")
}

fn declared_entity_identity<S>(_: &S, entity: usize) -> usize {
    entity
}

fn unavailable_savings_distance<S>(_: &S, _: usize, _: usize, _: usize) -> i64 {
    panic!("compiled savings distance was invoked without an available savings bundle")
}

fn unavailable_savings_feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
    panic!("compiled savings feasibility was invoked without an available savings bundle")
}

/// Non-null typed route/savings callables plus their declared source policy.
///
/// Every field is callable, including unavailable values. Capability
/// validation prevents an unavailable callable from reaching execution. This
/// makes the successful static route/K-opt/Clarke-Wright path direct function
/// dispatch with no `Option` or policy branch per candidate.
#[derive(Clone, Copy)]
pub(crate) struct StaticRouteBindings<S> {
    pub(super) read_policy: RouteReadPolicy,
    pub(super) replace_policy: RouteReplacePolicy,
    pub(super) feasibility_policy: RouteFeasibilityPolicy,
    pub(super) metric_class_policy: SavingsMetricClassPolicy,
    pub(super) route_read: fn(&S, usize) -> Vec<usize>,
    pub(super) route_replace: fn(&mut S, usize, Vec<usize>),
    pub(super) route_depot: fn(&S, usize) -> usize,
    pub(super) route_distance: fn(&S, usize, usize, usize) -> i64,
    pub(super) route_feasible: fn(&S, usize, &[usize]) -> bool,
    pub(super) savings_depot: fn(&S, usize) -> usize,
    pub(super) savings_metric_class: fn(&S, usize) -> usize,
    pub(super) savings_distance: fn(&S, usize, usize, usize) -> i64,
    pub(super) savings_feasible: fn(&S, usize, &[usize]) -> bool,
    route_depot_declared: bool,
    route_distance_declared: bool,
    savings_depot_declared: bool,
    savings_distance_declared: bool,
    savings_feasible_declared: bool,
}

impl<S> fmt::Debug for StaticRouteBindings<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticRouteBindings")
            .field("read_policy", &self.read_policy.trace_label())
            .field("replace_policy", &self.replace_policy.trace_label())
            .field("feasibility_policy", &self.feasibility_policy.trace_label())
            .field(
                "metric_class_policy",
                &self.metric_class_policy.trace_label(),
            )
            .field("route_depot_declared", &self.route_depot_declared)
            .field("route_distance_declared", &self.route_distance_declared)
            .field("savings_depot_declared", &self.savings_depot_declared)
            .field("savings_distance_declared", &self.savings_distance_declared)
            .field("savings_feasible_declared", &self.savings_feasible_declared)
            .finish()
    }
}

impl<S> StaticRouteBindings<S> {
    pub(super) fn from_slot<V, DM, IDM>(slot: &ListVariableSlot<S, V, DM, IDM>) -> Self {
        let (read_policy, route_read): (RouteReadPolicy, fn(&S, usize) -> Vec<usize>) =
            match slot.route_get_fn {
                Some(route_read) => (RouteReadPolicy::ExplicitStaticProvider, route_read),
                None => (RouteReadPolicy::Unavailable, unavailable_route_read::<S>),
            };
        let (replace_policy, route_replace): (RouteReplacePolicy, fn(&mut S, usize, Vec<usize>)) =
            match slot.route_set_fn {
                Some(route_replace) => (RouteReplacePolicy::ExplicitStaticProvider, route_replace),
                None => (
                    RouteReplacePolicy::Unavailable,
                    unavailable_route_replace::<S>,
                ),
            };
        let (feasibility_policy, route_feasible): (
            RouteFeasibilityPolicy,
            fn(&S, usize, &[usize]) -> bool,
        ) = match slot.route_feasible_fn {
            Some(route_feasible) => (
                RouteFeasibilityPolicy::ExplicitStaticProvider,
                route_feasible,
            ),
            None => (
                RouteFeasibilityPolicy::DeclaredAlwaysFeasible,
                declared_always_feasible::<S>,
            ),
        };
        let (metric_class_policy, savings_metric_class): (
            SavingsMetricClassPolicy,
            fn(&S, usize) -> usize,
        ) = match slot.savings_metric_class_fn {
            Some(metric_class) => (
                SavingsMetricClassPolicy::ExplicitStaticProvider,
                metric_class,
            ),
            None => (
                SavingsMetricClassPolicy::DeclaredEntityIdentity,
                declared_entity_identity::<S>,
            ),
        };
        Self {
            read_policy,
            replace_policy,
            feasibility_policy,
            metric_class_policy,
            route_read,
            route_replace,
            route_depot: slot.route_depot_fn.unwrap_or(unavailable_route_depot::<S>),
            route_distance: slot
                .route_distance_fn
                .unwrap_or(unavailable_route_distance::<S>),
            route_feasible,
            savings_depot: slot
                .savings_depot_fn
                .unwrap_or(unavailable_savings_depot::<S>),
            savings_metric_class,
            savings_distance: slot
                .savings_distance_fn
                .unwrap_or(unavailable_savings_distance::<S>),
            savings_feasible: slot
                .savings_feasible_fn
                .unwrap_or(unavailable_savings_feasible::<S>),
            route_depot_declared: slot.route_depot_fn.is_some(),
            route_distance_declared: slot.route_distance_fn.is_some(),
            savings_depot_declared: slot.savings_depot_fn.is_some(),
            savings_distance_declared: slot.savings_distance_fn.is_some(),
            savings_feasible_declared: slot.savings_feasible_fn.is_some(),
        }
    }

    pub(super) const fn supports_replace(&self) -> bool {
        self.replace_policy.is_available()
    }

    pub(super) const fn supports_route(&self) -> bool {
        self.read_policy.is_available()
            && self.replace_policy.is_available()
            && self.route_depot_declared
            && self.route_distance_declared
            && self.feasibility_policy.is_available()
    }

    pub(super) const fn supports_savings(&self) -> bool {
        self.replace_policy.is_available()
            && self.savings_depot_declared
            && self.savings_distance_declared
            && self.savings_feasible_declared
            && self.metric_class_policy.is_available()
    }
}

/// Dynamic payload plus route/savings policies derived from immutable schema.
///
/// `Deref` preserves direct dynamic slot calls in generic kernels. Policy is
/// frozen for compile-time validation and provenance; it does not add a trait
/// object or a nullable-source decision to candidate generation.
#[derive(Clone)]
pub(crate) struct RuntimeDynamicListSlot<S> {
    slot: DynamicListVariableSlot<S>,
    read_policy: RouteReadPolicy,
    replace_policy: RouteReplacePolicy,
    feasibility_policy: RouteFeasibilityPolicy,
    metric_class_policy: SavingsMetricClassPolicy,
    ownership_policy: OwnershipPolicy,
    construction_order_policy: ConstructionOrderPolicy,
    precedence_policy: PrecedencePolicy,
}

impl<S> RuntimeDynamicListSlot<S> {
    pub(super) fn new(slot: DynamicListVariableSlot<S>) -> Self {
        let access = slot.access_capabilities();
        let metadata = slot.metadata_capabilities().unwrap_or_default();
        Self {
            slot,
            read_policy: RouteReadPolicy::DynamicBaseAccess,
            replace_policy: if access.replace {
                RouteReplacePolicy::DynamicBaseAccess
            } else {
                RouteReplacePolicy::Unavailable
            },
            feasibility_policy: if metadata.route {
                RouteFeasibilityPolicy::ExplicitDynamicProvider
            } else {
                RouteFeasibilityPolicy::Unavailable
            },
            metric_class_policy: if metadata.savings {
                SavingsMetricClassPolicy::ExplicitDynamicProvider
            } else {
                SavingsMetricClassPolicy::Unavailable
            },
            ownership_policy: if metadata.element_owner {
                OwnershipPolicy::ExplicitDynamicProvider
            } else {
                OwnershipPolicy::DeclaredUnrestricted
            },
            construction_order_policy: if metadata.construction_order_key {
                ConstructionOrderPolicy::ExplicitDynamicProvider
            } else {
                ConstructionOrderPolicy::DeclaredNaturalElementOrder
            },
            precedence_policy: match (metadata.precedence_duration, metadata.precedence_successors)
            {
                (true, true) => PrecedencePolicy::Explicit,
                (false, true) => PrecedencePolicy::SuccessorsOnly,
                // Duration without ordered successors has no construction or
                // selector behavior, so it is deliberately
                // absent rather than a partial precedence fallback.
                (true, false) | (false, false) => PrecedencePolicy::Absent,
            },
        }
    }

    pub(super) const fn route_read_policy(&self) -> RouteReadPolicy {
        self.read_policy
    }

    pub(super) const fn route_replace_policy(&self) -> RouteReplacePolicy {
        self.replace_policy
    }

    pub(super) const fn route_feasibility_policy(&self) -> RouteFeasibilityPolicy {
        self.feasibility_policy
    }

    pub(super) const fn savings_metric_class_policy(&self) -> SavingsMetricClassPolicy {
        self.metric_class_policy
    }

    pub(super) const fn ownership_policy(&self) -> OwnershipPolicy {
        self.ownership_policy
    }

    pub(super) const fn construction_order_policy(&self) -> ConstructionOrderPolicy {
        self.construction_order_policy
    }

    pub(super) const fn precedence_policy(&self) -> PrecedencePolicy {
        self.precedence_policy
    }
}

impl<S> fmt::Debug for RuntimeDynamicListSlot<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeDynamicListSlot")
            .field("slot", &self.slot)
            .field("read_policy", &self.read_policy.trace_label())
            .field("replace_policy", &self.replace_policy.trace_label())
            .field("feasibility_policy", &self.feasibility_policy.trace_label())
            .field(
                "metric_class_policy",
                &self.metric_class_policy.trace_label(),
            )
            .field("ownership_policy", &self.ownership_policy.trace_label())
            .field(
                "construction_order_policy",
                &self.construction_order_policy.trace_label(),
            )
            .field("precedence_policy", &self.precedence_policy.trace_label())
            .finish()
    }
}

impl<S> Deref for RuntimeDynamicListSlot<S> {
    type Target = DynamicListVariableSlot<S>;

    fn deref(&self) -> &Self::Target {
        &self.slot
    }
}
