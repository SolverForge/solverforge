mod plan;
mod problem;
mod vehicle;

pub use plan::VrpPlan;
pub use problem::{assign_route, capacity, depot_for_cw, distance, element_load, ProblemData};
pub use vehicle::Vehicle;
