use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "customer_values",
        solution_trait = "solverforge::cvrp::VrpSolution",
        distance_meter = "solverforge::cvrp::MatrixDistanceMeter",
        intra_distance_meter = "solverforge::cvrp::MatrixIntraDistanceMeter",
        route_get_fn = "solverforge::cvrp::get_route",
        route_set_fn = "solverforge::cvrp::replace_route",
        route_depot_fn = "solverforge::cvrp::depot_for_entity",
        route_metric_class_fn = "solverforge::cvrp::route_metric_class",
        route_distance_fn = "solverforge::cvrp::route_distance",
        route_feasible_fn = "solverforge::cvrp::route_feasible"
    )]
    pub visits: Vec<usize>,

    pub data_addr: usize,
}
