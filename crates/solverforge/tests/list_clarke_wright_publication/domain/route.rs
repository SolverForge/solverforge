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
        route_hooks = "solverforge::cvrp::route_hooks",
        savings_hooks = "solverforge::cvrp::savings_hooks",
        savings_metric_class_fn = "solverforge::cvrp::savings_metric_class"
    )]
    pub visits: Vec<usize>,

    pub data_addr: usize,
}
