use solverforge::prelude::*;

#[planning_entity]
struct Task {
    #[planning_id]
    id: usize,
}

#[planning_solution]
struct Plan {
    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[constraint_weights]
    weights: ConstraintWeights,

    #[constraint_weights]
    more_weights: ConstraintWeights,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {}
