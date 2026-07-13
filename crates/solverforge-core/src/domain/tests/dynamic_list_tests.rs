use std::sync::Arc;

use crate::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId, VariableId,
};

#[derive(Clone, Debug)]
struct TestPlan {
    rows: Vec<Vec<usize>>,
}

#[derive(Debug)]
struct TestAccess {
    entity: EntityClassId,
    variable: VariableId,
}

impl DynamicListAccess<TestPlan> for TestAccess {
    fn entity_class(&self) -> EntityClassId {
        self.entity
    }

    fn variable(&self) -> VariableId {
        self.variable
    }

    fn entity_count(&self, solution: &TestPlan) -> usize {
        solution.rows.len()
    }

    fn element_count(&self, solution: &TestPlan) -> usize {
        solution
            .rows
            .iter()
            .flatten()
            .copied()
            .max()
            .map_or(0, |value| value + 1)
    }

    fn element(&self, _solution: &TestPlan, element_index: usize) -> Option<usize> {
        Some(element_index)
    }

    fn assigned_elements(&self, solution: &TestPlan) -> Vec<usize> {
        solution.rows.iter().flatten().copied().collect()
    }

    fn len(&self, solution: &TestPlan, row: usize) -> usize {
        solution.rows[row].len()
    }

    fn get(&self, solution: &TestPlan, row: usize, pos: usize) -> Option<usize> {
        solution.rows.get(row)?.get(pos).copied()
    }

    fn insert(&self, solution: &mut TestPlan, row: usize, pos: usize, value: usize) {
        solution.rows[row].insert(pos, value);
    }

    fn remove(&self, solution: &mut TestPlan, row: usize, pos: usize) -> Option<usize> {
        (pos < solution.rows[row].len()).then(|| solution.rows[row].remove(pos))
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            replace: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn replace(&self, solution: &mut TestPlan, row: usize, values: Vec<usize>) -> bool {
        let Some(target) = solution.rows.get_mut(row) else {
            return false;
        };
        *target = values;
        true
    }
}

#[derive(Debug)]
struct TestMetadata {
    entity: EntityClassId,
    variable: VariableId,
    capabilities: DynamicListMetadataCapabilities,
}

impl DynamicListMetadata<TestPlan> for TestMetadata {
    fn entity_class(&self) -> EntityClassId {
        self.entity
    }

    fn variable(&self) -> VariableId {
        self.variable
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        self.capabilities
    }

    fn element_owner(&self, _solution: &TestPlan, _element: usize) -> Option<usize> {
        None
    }

    fn construction_order_key(&self, _solution: &TestPlan, _element: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _solution: &TestPlan, _element: usize) -> Option<usize> {
        None
    }

    fn extend_precedence_successors(
        &self,
        _solution: &TestPlan,
        _element: usize,
        _successors: &mut Vec<usize>,
    ) -> bool {
        false
    }

    fn cross_position_distance(
        &self,
        _solution: &TestPlan,
        _from_entity: usize,
        _from_position: usize,
        _to_entity: usize,
        _to_position: usize,
    ) -> Option<f64> {
        None
    }

    fn intra_position_distance(
        &self,
        _solution: &TestPlan,
        _entity: usize,
        _from_position: usize,
        _to_position: usize,
    ) -> Option<f64> {
        None
    }

    fn route_depot(&self, _solution: &TestPlan, _entity: usize) -> Option<usize> {
        Some(0)
    }

    fn route_distance(
        &self,
        _solution: &TestPlan,
        _entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        Some((from as i64 - to as i64).unsigned_abs() as i64)
    }

    fn route_feasible(
        &self,
        _solution: &TestPlan,
        _entity: usize,
        _route: &[usize],
    ) -> Option<bool> {
        Some(true)
    }

    fn savings_depot(&self, _solution: &TestPlan, _entity: usize) -> Option<usize> {
        Some(0)
    }

    fn savings_metric_class(&self, _solution: &TestPlan, _entity: usize) -> Option<usize> {
        Some(0)
    }

    fn savings_distance(
        &self,
        _solution: &TestPlan,
        _entity: usize,
        from: usize,
        to: usize,
    ) -> Option<i64> {
        Some((from as i64 - to as i64).unsigned_abs() as i64)
    }

    fn savings_feasible(
        &self,
        _solution: &TestPlan,
        _entity: usize,
        _route: &[usize],
    ) -> Option<bool> {
        Some(true)
    }
}

fn access(entity: usize, variable: usize) -> Arc<dyn DynamicListAccess<TestPlan>> {
    Arc::new(TestAccess {
        entity: EntityClassId(entity),
        variable: VariableId(variable),
    })
}

fn metadata(
    entity: usize,
    variable: usize,
    capabilities: DynamicListMetadataCapabilities,
) -> Arc<dyn DynamicListMetadata<TestPlan>> {
    Arc::new(TestMetadata {
        entity: EntityClassId(entity),
        variable: VariableId(variable),
        capabilities,
    })
}

#[test]
fn dynamic_list_slot_rejects_mismatched_access_identity() {
    let error = DynamicListVariableSlot::try_with_access(
        EntityClassId(1),
        VariableId(2),
        "Vehicle",
        "visits",
        access(3, 2),
    )
    .expect_err("a slot must reject access bound to another entity");

    assert!(error.contains("dynamic list access"));
    assert!(error.contains("Vehicle.visits"));
}

#[test]
fn dynamic_list_slot_rejects_mismatched_metadata_identity() {
    let error = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(1),
        VariableId(2),
        "Vehicle",
        "visits",
        access(1, 2),
        metadata(1, 3, DynamicListMetadataCapabilities::default()),
    )
    .expect_err("a slot must reject metadata bound to another variable");

    assert!(error.contains("dynamic list metadata"));
    assert!(error.contains("Vehicle.visits"));
}

#[test]
fn dynamic_list_slot_keeps_metadata_and_direct_replacement_separate() {
    let capabilities = DynamicListMetadataCapabilities {
        route: true,
        savings: true,
        ..DynamicListMetadataCapabilities::default()
    };
    let slot = DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(1),
        VariableId(2),
        "Vehicle",
        "visits",
        access(1, 2),
        metadata(1, 2, capabilities),
    )
    .expect("matching immutable bindings are valid");
    let mut solution = TestPlan {
        rows: vec![vec![4, 8]],
    };

    assert_eq!(slot.metadata_capabilities(), Some(capabilities));
    assert!(slot.access_capabilities().replace);
    assert!(slot.list_replace(&mut solution, 0, vec![3, 5, 7]));
    assert_eq!(solution.rows, vec![vec![3, 5, 7]]);
}
