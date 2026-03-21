use solverforge::prelude::*;

#[planning_entity]
struct Task {
    #[planning_id]
    id: String,
    #[planning_variable(allows_unassigned = true)]
    worker_idx: Option<usize>,
}

#[problem_fact]
struct Worker {
    #[planning_id]
    id: usize,
}

#[planning_solution]
struct Plan {
    #[planning_entity_collection]
    tasks: Vec<Task>,
    #[problem_fact_collection]
    workers: Vec<Worker>,
    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {
    let _ = Plan {
        tasks: Vec::new(),
        workers: Vec::new(),
        score: None,
    };
}
