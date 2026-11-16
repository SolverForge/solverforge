//! Frozen ownership, construction-order, and precedence bindings.

use std::fmt;

use super::runtime_list_policy::{
    ConstructionOrderPolicy, ElementGroupPolicy, OwnershipPolicy, PrecedencePolicy,
};
use super::ListVariableSlot;

fn declared_unrestricted<S, V>(_: &S, _: &V) -> Option<usize> {
    None
}

fn declared_natural_element_order<S, V>(_: &S, _: V) -> i64 {
    0
}

fn unavailable_precedence_duration<S, V>(_: &S, _: V) -> usize {
    panic!("compiled precedence duration was invoked without explicit precedence")
}

fn unavailable_precedence_successors<S, V>(_: &S, _: V, _: &mut Vec<V>) {
    panic!("compiled precedence successors were invoked without precedence")
}

/// Non-null typed metadata callables with frozen semantic policies.
///
/// Natural ordering is represented by a constant key plus the shared
/// source-index tie-break. Owner absence is represented by a direct function
/// returning `None`. A precedence sentinel is unreachable after selected
/// family validation.
#[derive(Clone, Copy)]
pub(crate) struct StaticListMetadataBindings<S, V> {
    pub(super) ownership_policy: OwnershipPolicy,
    pub(super) construction_order_policy: ConstructionOrderPolicy,
    pub(super) precedence_policy: PrecedencePolicy,
    pub(super) element_group_policy: ElementGroupPolicy,
    pub(super) element_owner: fn(&S, &V) -> Option<usize>,
    pub(super) construction_order: fn(&S, V) -> i64,
    pub(super) precedence_duration: fn(&S, V) -> usize,
    /// Precedence successors under a precedence policy with successors, the
    /// element group hook under an explicit group policy; the two exclude
    /// each other.
    pub(super) element_relation: fn(&S, V, &mut Vec<V>),
}

impl<S, V> fmt::Debug for StaticListMetadataBindings<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticListMetadataBindings")
            .field("ownership_policy", &self.ownership_policy.trace_label())
            .field(
                "construction_order_policy",
                &self.construction_order_policy.trace_label(),
            )
            .field("precedence_policy", &self.precedence_policy.trace_label())
            .field(
                "element_group_policy",
                &self.element_group_policy.trace_label(),
            )
            .finish()
    }
}

impl<S, V> StaticListMetadataBindings<S, V> {
    pub(super) fn from_slot<DM, IDM>(slot: &ListVariableSlot<S, V, DM, IDM>) -> Self {
        let (ownership_policy, element_owner): (OwnershipPolicy, fn(&S, &V) -> Option<usize>) =
            match slot.element_owner_fn {
                Some(element_owner) => (OwnershipPolicy::ExplicitStaticProvider, element_owner),
                None => (
                    OwnershipPolicy::DeclaredUnrestricted,
                    declared_unrestricted::<S, V>,
                ),
            };
        let (construction_order_policy, construction_order): (
            ConstructionOrderPolicy,
            fn(&S, V) -> i64,
        ) = match slot.construction_element_order_key {
            Some(construction_order) => (
                ConstructionOrderPolicy::ExplicitStaticProvider,
                construction_order,
            ),
            None => (
                ConstructionOrderPolicy::DeclaredNaturalElementOrder,
                declared_natural_element_order::<S, V>,
            ),
        };
        let (precedence_policy, precedence_duration, precedence_successors): (
            PrecedencePolicy,
            fn(&S, V) -> usize,
            fn(&S, V, &mut Vec<V>),
        ) = match (slot.precedence_duration_fn, slot.precedence_successors_fn()) {
            (Some(duration), Some(successors)) => {
                (PrecedencePolicy::Explicit, duration, successors)
            }
            (None, Some(successors)) => (
                PrecedencePolicy::SuccessorsOnly,
                unavailable_precedence_duration::<S, V>,
                successors,
            ),
            (Some(_), None) | (None, None) => (
                PrecedencePolicy::Absent,
                unavailable_precedence_duration::<S, V>,
                unavailable_precedence_successors::<S, V>,
            ),
        };
        let (element_group_policy, element_relation): (ElementGroupPolicy, fn(&S, V, &mut Vec<V>)) =
            match slot.element_group_fn() {
                Some(element_group) => (ElementGroupPolicy::ExplicitStaticProvider, element_group),
                None => (ElementGroupPolicy::Absent, precedence_successors),
            };
        Self {
            ownership_policy,
            construction_order_policy,
            precedence_policy,
            element_group_policy,
            element_owner,
            construction_order,
            precedence_duration,
            element_relation,
        }
    }
}
