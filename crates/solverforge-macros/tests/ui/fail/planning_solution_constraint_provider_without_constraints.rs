use solverforge::prelude::*;

#[planning_entity]
struct Task {
    #[planning_id]
    id: usize,
}

#[planning_solution(constraint_provider = "relaxed_constraints")]
struct Plan {
    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {}
