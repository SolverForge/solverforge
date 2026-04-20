use solverforge::prelude::*;

#[problem_fact]
struct RouteTask {
    #[planning_id]
    id: usize,
    route: Option<usize>,
}

#[problem_fact]
struct ShiftTask {
    #[planning_id]
    id: usize,
}

#[planning_entity]
struct Route {
    #[planning_id]
    id: usize,

    #[planning_list_variable(element_collection = "route_tasks")]
    tasks: Vec<usize>,
}

#[planning_entity]
struct Shift {
    #[planning_id]
    id: usize,

    #[planning_list_variable(element_collection = "shift_tasks")]
    tasks: Vec<usize>,
}

#[planning_solution]
#[shadow_variable_updates(list_owner = "routes", inverse_field = "route")]
struct Plan {
    #[planning_entity_collection]
    routes: Vec<Route>,

    #[planning_entity_collection]
    shifts: Vec<Shift>,

    #[problem_fact_collection]
    route_tasks: Vec<RouteTask>,

    #[problem_fact_collection]
    shift_tasks: Vec<ShiftTask>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn main() {
    let _ = Plan {
        routes: Vec::new(),
        shifts: Vec::new(),
        route_tasks: Vec::new(),
        shift_tasks: Vec::new(),
        score: None,
    };
}
