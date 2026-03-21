// Re-export problem data and free functions from the framework's CVRP helpers.
pub use solverforge::cvrp::{
    capacity, depot_for_cw, distance, element_load, replace_route, ProblemData,
};
