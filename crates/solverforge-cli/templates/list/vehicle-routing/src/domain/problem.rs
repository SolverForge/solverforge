// Re-export problem data and free functions from the framework's CVRP helpers.
pub use solverforge::cvrp::{
    assign_route, capacity, depot_for_cw, distance, element_load, ProblemData,
};
