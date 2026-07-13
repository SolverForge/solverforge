//! Regression coverage for the frozen Clarke-Wright metric-class policy.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListVariableSlot, EntityClassId,
    PlanningSolution, VariableId,
};
use solverforge_core::score::SoftScore;

use super::list_access::{
    ListAccess, ListAccessCapability, RouteAccess, RouteSequenceAccess, SavingsAccess,
};
use super::runtime_list::RuntimeListElement;
use super::{
    ConstructionOrderPolicy, ListVariableSlot, OwnershipPolicy, PrecedencePolicy,
    RouteFeasibilityPolicy, RouteReadPolicy, RuntimeListSlot, SavingsMetricClassPolicy,
};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;

type RuntimeSlot =
    RuntimeListSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>;

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
    elements: Vec<usize>,
    routes: Vec<Vec<usize>>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn element_count(plan: &Plan) -> usize {
    plan.elements.len()
}

fn assigned_elements(plan: &Plan) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn entity_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.routes[entity].len()
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
}

fn construction_list_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].insert(position, value);
}

fn list_get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.routes.get(entity)?.get(position).copied()
}

fn list_set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity][position] = value;
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    plan.routes[entity].splice(position..position, values);
}

fn ruin_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn ruin_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].insert(position, value);
}

fn index_to_element(plan: &Plan, element_index: usize) -> usize {
    plan.elements[element_index]
}

static TYPED_ROUTE_READS: AtomicUsize = AtomicUsize::new(0);
static TYPED_ROUTE_REPLACEMENTS: AtomicUsize = AtomicUsize::new(0);
static DYNAMIC_BASE_READS: AtomicUsize = AtomicUsize::new(0);

fn typed_route_set(plan: &mut Plan, entity: usize, values: Vec<usize>) {
    TYPED_ROUTE_REPLACEMENTS.fetch_add(1, Ordering::SeqCst);
    plan.routes[entity] = values;
}

fn dynamic_route_set(plan: &mut Plan, entity: usize, values: Vec<usize>) {
    plan.routes[entity] = values;
}

fn typed_route_read(_: &Plan, _: usize) -> Vec<usize> {
    TYPED_ROUTE_READS.fetch_add(1, Ordering::SeqCst);
    vec![90, 91]
}

fn route_depot(_: &Plan, _: usize) -> usize {
    0
}

fn route_distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn savings_depot(_: &Plan, _: usize) -> usize {
    0
}

fn savings_distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
    from.abs_diff(to) as i64
}

fn savings_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
    true
}

fn explicit_metric_class(_: &Plan, _: usize) -> usize {
    17
}

fn explicit_owner(_: &Plan, _: &usize) -> Option<usize> {
    Some(1)
}

fn explicit_order(_: &Plan, _: usize) -> i64 {
    -1
}

fn precedence_duration(_: &Plan, _: usize) -> usize {
    1
}

fn precedence_successors(_: &Plan, element: usize, successors: &mut Vec<usize>) {
    successors.push(element + 1);
}

fn typed_list_slot(
    metric_class: Option<fn(&Plan, usize) -> usize>,
    route_read: Option<fn(&Plan, usize) -> Vec<usize>>,
    route_feasible: Option<fn(&Plan, usize, &[usize]) -> bool>,
) -> ListVariableSlot<Plan, usize, DefaultCrossEntityDistanceMeter, DefaultCrossEntityDistanceMeter>
{
    ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        crate::builder::usize_element_source_key,
        entity_count,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
        "visits",
        0,
        route_read,
        Some(typed_route_set),
        Some(route_depot),
        Some(route_distance),
        route_feasible,
        Some(savings_depot),
        metric_class,
        Some(savings_distance),
        Some(savings_feasible),
    )
}

fn typed_slot(
    metric_class: Option<fn(&Plan, usize) -> usize>,
    route_read: Option<fn(&Plan, usize) -> Vec<usize>>,
    route_feasible: Option<fn(&Plan, usize, &[usize]) -> bool>,
) -> RuntimeSlot {
    RuntimeListSlot::from_static(typed_list_slot(metric_class, route_read, route_feasible), 0)
}

#[derive(Debug)]
struct DynamicAccess;

impl DynamicListAccess<Plan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, solution: &Plan) -> usize {
        entity_count(solution)
    }

    fn element_count(&self, solution: &Plan) -> usize {
        element_count(solution)
    }

    fn element(&self, solution: &Plan, element_index: usize) -> Option<usize> {
        solution.elements.get(element_index).copied()
    }

    fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
        assigned_elements(solution)
    }

    fn len(&self, solution: &Plan, entity: usize) -> usize {
        list_len(solution, entity)
    }

    fn get(&self, solution: &Plan, entity: usize, position: usize) -> Option<usize> {
        DYNAMIC_BASE_READS.fetch_add(1, Ordering::SeqCst);
        list_get(solution, entity, position)
    }

    fn insert(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(solution, entity, position, value);
    }

    fn remove(&self, solution: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(solution, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            replace: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn replace(&self, solution: &mut Plan, entity: usize, values: Vec<usize>) -> bool {
        dynamic_route_set(solution, entity, values);
        true
    }
}

#[test]
fn typed_omitted_metric_class_is_the_declared_entity_identity_policy() {
    let slot = typed_slot(None, None, None);
    let plan = Plan {
        score: None,
        elements: vec![0],
        routes: vec![vec![], vec![]],
    };

    assert_eq!(
        slot.savings_metric_class_policy(),
        SavingsMetricClassPolicy::DeclaredEntityIdentity
    );
    assert!(slot.supports(ListAccessCapability::Savings));
    assert!(SavingsAccess::validate_savings(&slot).is_ok());
    assert_eq!(SavingsAccess::savings_metric_class(&slot, &plan, 1), Ok(1));
}

#[test]
fn typed_explicit_metric_class_remains_an_explicit_provider_policy() {
    let slot = typed_slot(Some(explicit_metric_class), None, None);
    let plan = Plan {
        score: None,
        elements: vec![0],
        routes: vec![vec![]],
    };

    assert_eq!(
        slot.savings_metric_class_policy(),
        SavingsMetricClassPolicy::ExplicitStaticProvider
    );
    assert_eq!(SavingsAccess::savings_metric_class(&slot, &plan, 0), Ok(17));
}

#[test]
fn dynamic_omitted_savings_metadata_has_no_entity_identity_policy() {
    let slot: RuntimeSlot = RuntimeListSlot::from_dynamic(
        DynamicListVariableSlot::try_with_access(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(DynamicAccess),
        )
        .expect("dynamic test access identity matches its slot"),
    );

    assert_eq!(
        slot.savings_metric_class_policy(),
        SavingsMetricClassPolicy::Unavailable
    );
    assert!(!slot.supports(ListAccessCapability::Savings));
    assert_eq!(
        SavingsAccess::validate_savings(&slot)
            .expect_err("dynamic savings metadata must include metric class")
            .capability,
        ListAccessCapability::Savings
    );
    assert_eq!(
        slot.route_feasibility_policy(),
        RouteFeasibilityPolicy::Unavailable
    );
    assert!(!slot.supports(ListAccessCapability::Route));
    assert_eq!(
        RouteAccess::validate_route(&slot)
            .expect_err("dynamic route metadata must include feasibility")
            .capability,
        ListAccessCapability::Route
    );
}

#[test]
fn typed_route_uses_the_declared_read_and_replace_providers_once() {
    TYPED_ROUTE_READS.store(0, Ordering::SeqCst);
    TYPED_ROUTE_REPLACEMENTS.store(0, Ordering::SeqCst);
    let slot = typed_slot(None, Some(typed_route_read), None);
    let mut plan = Plan {
        score: None,
        elements: vec![1, 2],
        routes: vec![vec![1, 2]],
    };

    assert_eq!(
        slot.route_read_policy(),
        RouteReadPolicy::ExplicitStaticProvider
    );
    assert_eq!(
        slot.route_feasibility_policy(),
        RouteFeasibilityPolicy::DeclaredAlwaysFeasible
    );
    assert!(RouteAccess::validate_route(&slot).is_ok());
    assert_eq!(
        RouteSequenceAccess::route_values(&slot, &plan, 0),
        Ok(vec![90, 91])
    );
    assert_eq!(TYPED_ROUTE_READS.load(Ordering::SeqCst), 1);
    assert_eq!(
        RouteAccess::route_feasible(&slot, &plan, 0, &[1, 2]),
        Ok(true)
    );

    RouteSequenceAccess::replace_route(&slot, &mut plan, 0, vec![7, 8])
        .expect("typed replacement policy is available");
    assert_eq!(TYPED_ROUTE_REPLACEMENTS.load(Ordering::SeqCst), 1);
    assert_eq!(plan.routes, vec![vec![7, 8]]);
}

#[test]
fn dynamic_route_sequence_reads_the_dynamic_base_slot() {
    DYNAMIC_BASE_READS.store(0, Ordering::SeqCst);
    let slot: RuntimeSlot = RuntimeListSlot::from_dynamic(
        DynamicListVariableSlot::try_with_access(
            EntityClassId(0),
            VariableId(0),
            "Vehicle",
            "visits",
            Arc::new(DynamicAccess),
        )
        .expect("dynamic test access identity matches its slot"),
    );
    let plan = Plan {
        score: None,
        elements: vec![3, 4],
        routes: vec![vec![3, 4]],
    };

    assert_eq!(slot.route_read_policy(), RouteReadPolicy::DynamicBaseAccess);
    assert_eq!(
        RouteSequenceAccess::route_values(&slot, &plan, 0),
        Ok(vec![3, 4])
    );
    assert_eq!(DYNAMIC_BASE_READS.load(Ordering::SeqCst), 2);
}

#[test]
fn absent_typed_metadata_is_bound_to_named_public_policies() {
    let slot = typed_slot(None, None, None);
    let plan = Plan {
        score: None,
        elements: vec![2],
        routes: vec![vec![]],
    };

    assert_eq!(
        slot.ownership_policy(),
        OwnershipPolicy::DeclaredUnrestricted
    );
    assert_eq!(
        slot.construction_order_policy(),
        ConstructionOrderPolicy::DeclaredNaturalElementOrder
    );
    assert_eq!(slot.precedence_policy(), PrecedencePolicy::Absent);
    assert_eq!(
        ListAccess::element_owner(&slot, &plan, &RuntimeListElement::Static(2)),
        Ok(None)
    );
    assert_eq!(
        ListAccess::construction_order_key(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(0)
    );
    assert_eq!(
        ListAccess::extend_precedence_successors(
            &slot,
            &plan,
            RuntimeListElement::Static(2),
            &mut Vec::new(),
        )
        .expect_err("absent precedence must not enter precedence-only kernels")
        .capability,
        ListAccessCapability::Precedence
    );
}

#[test]
fn typed_successors_only_precedence_preserves_list_ruin_metadata() {
    let slot = RuntimeListSlot::from_static(
        typed_list_slot(None, None, None).with_precedence_hooks(None, Some(precedence_successors)),
        0,
    );
    let plan = Plan {
        score: None,
        elements: vec![2, 3],
        routes: vec![vec![]],
    };
    let mut successors = Vec::new();

    assert_eq!(slot.precedence_policy(), PrecedencePolicy::SuccessorsOnly);
    assert!(!slot.supports(ListAccessCapability::Precedence));
    ListAccess::extend_precedence_successors(
        &slot,
        &plan,
        RuntimeListElement::Static(2),
        &mut successors,
    )
    .expect("successor-only policy remains available to ListRuin");
    assert_eq!(successors, vec![RuntimeListElement::Static(3)]);
    assert_eq!(
        ListAccess::precedence_duration(&slot, &plan, RuntimeListElement::Static(2))
            .expect_err("scored precedence requires an explicit duration")
            .capability,
        ListAccessCapability::Precedence
    );
}

#[test]
fn typed_explicit_metadata_remains_available_to_required_families() {
    let slot = RuntimeListSlot::from_static(
        typed_list_slot(None, None, None)
            .with_element_owner_fn(Some(explicit_owner))
            .with_construction_element_order_key(Some(explicit_order))
            .with_precedence_hooks(Some(precedence_duration), Some(precedence_successors)),
        0,
    );
    let plan = Plan {
        score: None,
        elements: vec![2, 3],
        routes: vec![vec![]],
    };

    assert_eq!(
        slot.ownership_policy(),
        OwnershipPolicy::ExplicitStaticProvider
    );
    assert_eq!(
        slot.construction_order_policy(),
        ConstructionOrderPolicy::ExplicitStaticProvider
    );
    assert_eq!(slot.precedence_policy(), PrecedencePolicy::Explicit);
    assert!(slot.supports(ListAccessCapability::ElementOwner));
    assert!(slot.supports(ListAccessCapability::ConstructionOrderKey));
    assert!(slot.supports(ListAccessCapability::Precedence));
    assert_eq!(
        ListAccess::element_owner(&slot, &plan, &RuntimeListElement::Static(2)),
        Ok(Some(1))
    );
    assert_eq!(
        ListAccess::construction_order_key(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(-1)
    );
    assert_eq!(
        ListAccess::precedence_duration(&slot, &plan, RuntimeListElement::Static(2)),
        Ok(1)
    );
}
