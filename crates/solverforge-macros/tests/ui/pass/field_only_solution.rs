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
    let mut plan = Plan {
        routes: Vec::new(),
        visits: Vec::new(),
        score: None,
    };

    let _ = Plan::list_len_static(&plan, 0);
    let _ = Plan::element_count(&plan);
    let _ = Plan::routes_list_len_static(&plan, 0);
    let _ = Plan::routes_element_count(&plan);
    Plan::assign_element(&mut plan, 0, 0);
    Plan::routes_assign_element(&mut plan, 0, 0);
}
