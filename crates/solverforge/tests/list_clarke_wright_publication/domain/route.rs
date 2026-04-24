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
        cw_depot_fn = "solverforge::cvrp::depot_for_cw",
        cw_distance_fn = "solverforge::cvrp::distance",
        cw_element_load_fn = "solverforge::cvrp::element_load",
        cw_capacity_fn = "solverforge::cvrp::capacity",
        cw_assign_route_fn = "solverforge::cvrp::replace_route",
        k_opt_get_route = "solverforge::cvrp::get_route",
        k_opt_set_route = "solverforge::cvrp::replace_route",
        k_opt_depot_fn = "solverforge::cvrp::depot_for_entity",
        k_opt_distance_fn = "solverforge::cvrp::distance"
    )]
    pub visits: Vec<usize>,

    pub data_addr: usize,
}
