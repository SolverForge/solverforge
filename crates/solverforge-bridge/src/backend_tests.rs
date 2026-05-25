use solverforge_core::domain::PlanningSolution;

use crate::{DynamicModelBackend, DynamicScore, EntityClassId, VariableId};

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
