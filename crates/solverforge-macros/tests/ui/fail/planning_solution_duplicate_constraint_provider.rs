use solverforge::prelude::*;

#[planning_entity]
struct Task {
    #[planning_id]
    id: usize,
}

#[planning_solution(
    constraints = "define_constraints",
    constraint_provider = "relaxed_constraints",
    constraint_provider = "relaxed_constraints"
)]
struct Plan {
    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

#[planning_solution(constraints = "define_constraints", constraint_provider = "define_constraints")]
struct DefaultTwice {
    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {}
