use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId, VariableId,
};

use crate::builder::context::RuntimeListElement;
use crate::heuristic::r#move::list_kernel::ListRuinAccess;
use crate::list_placement::OwnerRestriction;

use super::support::{descriptor, dynamic_slot, initial_plan, ListPlan, Slot};

#[derive(Debug)]
struct SuccessorsOnlyAccess;

impl DynamicListAccess<ListPlan> for SuccessorsOnlyAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &ListPlan) -> usize {
        plan.routes.len()
    }

    fn element_count(&self, plan: &ListPlan) -> usize {
        plan.elements.len()
    }

    fn element(&self, plan: &ListPlan, index: usize) -> Option<usize> {
        plan.elements.get(index).copied()
    }

    fn assigned_elements(&self, plan: &ListPlan) -> Vec<usize> {
        plan.routes.iter().flatten().copied().collect()
    }

    fn len(&self, plan: &ListPlan, entity: usize) -> usize {
        plan.routes[entity].len()
    }

    fn get(&self, plan: &ListPlan, entity: usize, position: usize) -> Option<usize> {
        plan.routes.get(entity)?.get(position).copied()
    }

    fn insert(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) {
        plan.routes[entity].insert(position, value);
    }

    fn remove(&self, plan: &mut ListPlan, entity: usize, position: usize) -> Option<usize> {
        (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            set: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn set(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) -> bool {
        plan.routes[entity][position] = value;
        true
    }
}

#[derive(Debug)]
struct SuccessorsOnlyMetadata;

impl DynamicListMetadata<ListPlan> for SuccessorsOnlyMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            element_owner: true,
            precedence_successors: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, plan: &ListPlan, element: usize) -> Option<usize> {
        Some(element % plan.routes.len())
    }

    fn construction_order_key(&self, _: &ListPlan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn extend_precedence_successors(
        &self,
        _: &ListPlan,
        element: usize,
        successors: &mut Vec<usize>,
    ) -> bool {
        if element % 4 != 3 {
            successors.push(element + 1);
        }
        true
    }

    fn cross_position_distance(
        &self,
        _: &ListPlan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        None
    }

    fn intra_position_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<f64> {
        None
    }

    fn route_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_metric_class(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn savings_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }
}

fn successors_only_slot() -> Slot {
    let slot = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(SuccessorsOnlyAccess),
        Arc::new(SuccessorsOnlyMetadata),
    )
    .expect("successor-only test metadata matches its slot")
    .resolved_against(&descriptor())
    .expect("successor-only test slot resolves against descriptor");
    Slot::from_dynamic(slot)
}

#[test]
fn runtime_list_ruin_access_keeps_absent_owner_unrestricted() {
    let plan = initial_plan();
    let slot = dynamic_slot();
    assert!(!ListRuinAccess::has_owner_binding(&slot));
    assert_eq!(
        ListRuinAccess::owner_restriction(
            &slot,
            &plan,
            plan.routes.len(),
            &RuntimeListElement::Dynamic(0),
        ),
        OwnerRestriction::Unrestricted,
    );
}

#[test]
fn runtime_list_ruin_access_uses_explicit_owner_and_successors_only_graph() {
    let plan = initial_plan();
    let slot = successors_only_slot();
    assert!(ListRuinAccess::has_owner_binding(&slot));
    assert_eq!(
        ListRuinAccess::owner_restriction(
            &slot,
            &plan,
            plan.routes.len(),
            &RuntimeListElement::Dynamic(5),
        ),
        OwnerRestriction::Fixed(1),
    );

    let (elements, graph) = ListRuinAccess::recreate_precedence_graph(&slot, &plan)
        .expect("successors-only metadata is valid for list ruin recreation");
    assert_eq!(elements[0], RuntimeListElement::Dynamic(0));
    assert_eq!(graph.fixed_successors(0), &[1]);
    assert_eq!(graph.route(0), Some(&[0, 1, 2, 3][..]));
}

#[test]
fn successor_only_metadata_is_not_misrepresented_as_full_precedence() {
    let slot = successors_only_slot();
    assert!(!slot.precedence_policy().is_explicit());
    assert!(slot.precedence_policy().has_successors());
}
