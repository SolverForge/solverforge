use solverforge::prelude::*;

#[planning_entity]
struct Visit {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Route {
    #[planning_id]
    id: usize,

    #[planning_list_variable(element_collection = "visits")]
    visits: Vec<usize>,
}

#[planning_solution]
struct Plan {
    #[planning_entity_collection]
    routes: Vec<Route>,

    #[planning_entity_collection]
    visits: Vec<Visit>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {
    let _ = Plan {
        routes: Vec::new(),
        visits: Vec::new(),
        score: None,
    };
}
