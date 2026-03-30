use solverforge::prelude::*;

#[problem_fact]
struct Worker {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Route {
    #[planning_id]
    id: usize,

    #[planning_variable(value_range = "workers", allows_unassigned = true)]
    worker: Option<usize>,

    #[planning_list_variable]
    visits: Vec<usize>,
}

#[planning_solution]
#[standard_variable_config(
    entity_collection = "routes",
    variable_field = "worker",
    variable_type = "usize",
    value_range = "workers"
)]
#[shadow_variable_updates(
    list_owner = "routes",
    list_field = "visits",
    element_collection = "all_visits",
    element_type = "usize"
)]
struct MixedPlan {
    #[problem_fact_collection]
    workers: Vec<Worker>,

    all_visits: Vec<usize>,

    #[planning_entity_collection]
    routes: Vec<Route>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {
    let _ = MixedPlan {
        workers: Vec::new(),
        all_visits: Vec::new(),
        routes: Vec::new(),
        score: None,
    };
}
