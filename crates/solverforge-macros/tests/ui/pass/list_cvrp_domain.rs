use solverforge::cvrp::{ProblemData, VrpSolution};
use solverforge::prelude::*;

#[planning_entity]
pub struct Vehicle {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "deliveries", domain = "cvrp")]
    pub visits: Vec<usize>,
}

#[derive(Clone)]
struct Plan {
    vehicles: Vec<Vehicle>,
    data: ProblemData,
    score: Option<HardSoftScore>,
}

impl solverforge::__internal::PlanningSolution for Plan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl VrpSolution for Plan {
    fn vehicle_data(&self, entity_idx: usize) -> Option<&ProblemData> {
        (entity_idx < self.vehicles.len()).then_some(&self.data)
    }

    fn vehicle_visits(&self, entity_idx: usize) -> &[usize] {
        &self.vehicles[entity_idx].visits
    }

    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize> {
        &mut self.vehicles[entity_idx].visits
    }

    fn vehicle_count(&self) -> usize {
        self.vehicles.len()
    }
}

fn main() {
    let _metadata =
        <Vehicle as solverforge::__internal::ListVariableEntity<Plan>>::list_metadata();
}
