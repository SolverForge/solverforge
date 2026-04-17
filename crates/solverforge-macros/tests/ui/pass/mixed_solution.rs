use solverforge::prelude::*;

#[problem_fact]
struct Worker {
    #[planning_id]
    id: usize,
}

#[problem_fact]
struct Visit {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Route {
    #[planning_id]
    id: usize,

    #[planning_variable(value_range = "workers", allows_unassigned = true)]
    worker: Option<usize>,

    #[planning_list_variable(element_collection = "visits")]
    visits: Vec<usize>,
}

#[planning_solution]
struct MixedPlan {
    #[problem_fact_collection]
    workers: Vec<Worker>,

    #[planning_entity_collection]
    routes: Vec<Route>,

    #[problem_fact_collection]
    visits: Vec<Visit>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {
    let _ = MixedPlan {
        workers: Vec::new(),
        routes: Vec::new(),
        visits: Vec::new(),
        score: None,
    };
}
