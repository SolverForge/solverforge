use solverforge_core::domain::PlanningSolution;

use crate::{
    DynamicListVariableSlot, DynamicModelBackend, DynamicScalarVariableSlot, DynamicScore,
    EntityClassId, VariableId,
};

#[derive(Clone)]
struct DynamicRows {
    task_values: Vec<Option<usize>>,
    vehicle_routes: Vec<Vec<usize>>,
    candidates: Vec<usize>,
    score: Option<DynamicScore>,
}

impl PlanningSolution for DynamicRows {
    type Score = DynamicScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl DynamicModelBackend for DynamicRows {
    type Score = DynamicScore;

    fn entity_count(&self, entity: EntityClassId) -> usize {
        match entity.0 {
            0 => self.task_values.len(),
            1 => self.vehicle_routes.len(),
            _ => 0,
        }
    }

    fn get_scalar(&self, entity: EntityClassId, row: usize, variable: VariableId) -> Option<usize> {
        match (entity.0, variable.0) {
            (0, 0) => self.task_values[row],
            _ => None,
        }
    }

    fn set_scalar(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        value: Option<usize>,
    ) {
        if (entity.0, variable.0) == (0, 0) {
            self.task_values[row] = value;
        }
    }

    fn list_len(&self, entity: EntityClassId, row: usize, variable: VariableId) -> usize {
        match (entity.0, variable.0) {
            (1, 1) => self.vehicle_routes[row].len(),
            _ => 0,
        }
    }

    fn list_get(
        &self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize> {
        match (entity.0, variable.0) {
            (1, 1) => self.vehicle_routes[row].get(pos).copied(),
            _ => None,
        }
    }

    fn list_insert(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
        value: usize,
    ) {
        if (entity.0, variable.0) == (1, 1) {
            self.vehicle_routes[row].insert(pos, value);
        }
    }

    fn list_remove(
        &mut self,
        entity: EntityClassId,
        row: usize,
        variable: VariableId,
        pos: usize,
    ) -> Option<usize> {
        if (entity.0, variable.0) == (1, 1) {
            return Some(self.vehicle_routes[row].remove(pos));
        }
        None
    }

    fn candidate_values(
        &self,
        entity: EntityClassId,
        _row: usize,
        variable: VariableId,
    ) -> &[usize] {
        match (entity.0, variable.0) {
            (0, 0) => &self.candidates,
            _ => &[],
        }
    }

    fn list_element_count(&self, entity: EntityClassId, variable: VariableId) -> usize {
        match (entity.0, variable.0) {
            (1, 1) => 4,
            _ => 0,
        }
    }

    fn list_assigned_elements(&self, entity: EntityClassId, variable: VariableId) -> Vec<usize> {
        match (entity.0, variable.0) {
            (1, 1) => self
                .vehicle_routes
                .iter()
                .flat_map(|route| route.iter().copied())
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[test]
fn one_rust_state_type_can_host_multiple_logical_entity_classes() {
    let mut model = DynamicRows {
        task_values: vec![None, Some(1)],
        vehicle_routes: vec![vec![0, 2], vec![1]],
        candidates: vec![0, 1, 2],
        score: None,
    };

    let task = EntityClassId(0);
    let vehicle = EntityClassId(1);
    let task_assignment = VariableId(0);
    let visits = VariableId(1);

    assert_eq!(model.entity_count(task), 2);
    assert_eq!(model.entity_count(vehicle), 2);
    assert_eq!(model.get_scalar(task, 0, task_assignment), None);
    assert_eq!(model.candidate_values(task, 0, task_assignment), &[0, 1, 2]);

    model.set_scalar(task, 0, task_assignment, Some(2));
    assert_eq!(model.get_scalar(task, 0, task_assignment), Some(2));

    assert_eq!(model.list_get(vehicle, 0, visits, 1), Some(2));
    model.list_insert(vehicle, 1, visits, 1, 3);
    assert_eq!(model.list_len(vehicle, 1, visits), 2);
    assert_eq!(model.list_remove(vehicle, 1, visits, 0), Some(1));
    assert_eq!(model.list_get(vehicle, 1, visits, 0), Some(3));
}

#[test]
fn dynamic_scalar_slot_carries_logical_identity() {
    let mut model = DynamicRows {
        task_values: vec![None],
        vehicle_routes: Vec::new(),
        candidates: vec![2, 4, 6],
        score: None,
    };
    let slot =
        DynamicScalarVariableSlot::new(EntityClassId(0), VariableId(0), "Task", "worker", true);

    assert_eq!(slot.entity_count(&model), 1);
    assert_eq!(slot.current_value(&model, 0), None);
    assert!(slot.value_is_legal(&model, 0, None));
    assert!(slot.value_is_legal(&model, 0, Some(4)));
    assert!(!slot.value_is_legal(&model, 0, Some(5)));

    slot.set_value(&mut model, 0, Some(6));
    assert_eq!(slot.current_value(&model, 0), Some(6));
}

#[test]
fn dynamic_list_slot_carries_logical_identity() {
    let mut model = DynamicRows {
        task_values: Vec::new(),
        vehicle_routes: vec![vec![1], vec![2, 3]],
        candidates: Vec::new(),
        score: None,
    };
    let slot = DynamicListVariableSlot::new(EntityClassId(1), VariableId(1), "Vehicle", "visits");

    assert_eq!(slot.entity_count(&model), 2);
    assert_eq!(slot.element_count(&model), 4);
    assert_eq!(slot.assigned_elements(&model), vec![1, 2, 3]);
    assert_eq!(slot.list_len(&model, 1), 2);
    assert_eq!(slot.list_get(&model, 1, 0), Some(2));

    slot.list_insert(&mut model, 0, 1, 0);
    assert_eq!(slot.list_get(&model, 0, 1), Some(0));
    assert_eq!(slot.list_remove(&mut model, 1, 1), Some(3));
    assert_eq!(slot.assigned_elements(&model), vec![1, 0, 2]);
}
