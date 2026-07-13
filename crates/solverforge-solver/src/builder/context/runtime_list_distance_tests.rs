//! Metric-boundary regression tests for the unified runtime list carrier.

use std::sync::Arc;

use solverforge_core::domain::{
    DynamicListAccess, DynamicListMetadata, DynamicListMetadataCapabilities,
    DynamicListVariableSlot, EntityClassId, PlanningSolution, VariableId,
};
use solverforge_core::score::SoftScore;

use super::list_access::ListAccess;
use super::{ListVariableSlot, RuntimeListSlot};
use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct Plan {
    score: Option<SoftScore>,
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

fn metric(entity: usize, from_position: usize, to_position: usize) -> i64 {
    (entity as i64) * 10_000 + (from_position as i64) * 100 + to_position as i64
}

/// Native list meters historically use the cross-entity shape. A different
/// entity would be observably wrong, so this returns a sentinel for it.
#[derive(Clone, Debug)]
struct StaticIntraMetric;

impl CrossEntityDistanceMeter<Plan> for StaticIntraMetric {
    fn distance(
        &self,
        _: &Plan,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> f64 {
        if from_entity == to_entity {
            metric(from_entity, from_position, to_position) as f64
        } else {
            -999.0
        }
    }
}

/// Dynamic slots must not fall back to their generic meter; their bound
/// metadata is the only authoritative position metric.
#[derive(Clone, Debug)]
struct DynamicFallbackMustNotRun;

impl CrossEntityDistanceMeter<Plan> for DynamicFallbackMustNotRun {
    fn distance(&self, _: &Plan, _: usize, _: usize, _: usize, _: usize) -> f64 {
        -777.0
    }
}

fn element_count(_: &Plan) -> usize {
    0
}

fn assigned_elements(_: &Plan) -> Vec<usize> {
    Vec::new()
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

fn construction_remove(plan: &mut Plan, entity: usize, position: usize) -> usize {
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

fn index_to_element(_: &Plan, source_index: usize) -> usize {
    source_index
}

fn source_key(_: &Plan, element: &usize) -> usize {
    *element
}

fn static_slot() -> RuntimeListSlot<Plan, usize, StaticIntraMetric, StaticIntraMetric> {
    RuntimeListSlot::from_static(
        ListVariableSlot::new(
            "Route",
            element_count,
            assigned_elements,
            list_len,
            list_remove,
            construction_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            construction_remove,
            list_insert,
            index_to_element,
            source_key,
            entity_count,
            StaticIntraMetric,
            StaticIntraMetric,
            "visits",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        0,
    )
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

    fn element(&self, _: &Plan, source_index: usize) -> Option<usize> {
        Some(source_index)
    }

    fn assigned_elements(&self, solution: &Plan) -> Vec<usize> {
        assigned_elements(solution)
    }

    fn len(&self, solution: &Plan, entity: usize) -> usize {
        list_len(solution, entity)
    }

    fn get(&self, solution: &Plan, entity: usize, position: usize) -> Option<usize> {
        list_get(solution, entity, position)
    }

    fn insert(&self, solution: &mut Plan, entity: usize, position: usize, value: usize) {
        list_insert(solution, entity, position, value);
    }

    fn remove(&self, solution: &mut Plan, entity: usize, position: usize) -> Option<usize> {
        list_remove(solution, entity, position)
    }
}

#[derive(Debug)]
struct DynamicIntraMetadata;

impl DynamicListMetadata<Plan> for DynamicIntraMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            intra_position_distance: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn construction_order_key(&self, _: &Plan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn extend_precedence_successors(&self, _: &Plan, _: usize, _: &mut Vec<usize>) -> bool {
        false
    }

    fn cross_position_distance(
        &self,
        _: &Plan,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
    ) -> Option<f64> {
        None
    }

    fn intra_position_distance(
        &self,
        _: &Plan,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Option<f64> {
        Some(metric(entity, from_position, to_position) as f64)
    }

    fn route_depot(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &Plan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn savings_metric_class(&self, _: &Plan, _: usize) -> Option<usize> {
        None
    }

    fn savings_distance(&self, _: &Plan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn savings_feasible(&self, _: &Plan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }
}

fn dynamic_slot(
) -> RuntimeListSlot<Plan, usize, DynamicFallbackMustNotRun, DynamicFallbackMustNotRun> {
    RuntimeListSlot::from_dynamic(
        DynamicListVariableSlot::with_access_and_metadata(
            EntityClassId(0),
            VariableId(0),
            "Route",
            "visits",
            Arc::new(DynamicAccess),
            Arc::new(DynamicIntraMetadata),
        )
        .expect("dynamic access and metadata identities match"),
    )
}

#[test]
fn runtime_list_slot_adapts_static_and_dynamic_intra_metrics_without_fallbacks() {
    let plan = Plan {
        score: None,
        routes: vec![Vec::new(), Vec::new(), Vec::new()],
    };
    let expected = metric(2, 3, 7) as f64;
    let static_slot = static_slot();
    let dynamic_slot = dynamic_slot();

    assert_eq!(
        ListAccess::intra_position_distance(&static_slot, &plan, 2, 3, 7),
        Ok(expected)
    );
    assert_eq!(
        ListPositionDistanceMeter::distance(&static_slot, &plan, 2, 3, 7),
        expected
    );
    assert_eq!(
        ListAccess::intra_position_distance(&dynamic_slot, &plan, 2, 3, 7),
        Ok(expected)
    );
    assert_eq!(
        ListPositionDistanceMeter::distance(&dynamic_slot, &plan, 2, 3, 7),
        expected
    );
}
