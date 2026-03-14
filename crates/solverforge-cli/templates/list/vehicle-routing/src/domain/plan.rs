use solverforge::cvrp::{ProblemData, VrpSolution};
use solverforge::prelude::*;

use super::Vehicle;

/// The root planning solution: a fleet of vehicles and a score.
#[planning_solution(constraints = "crate::constraints::create_constraints")]
#[shadow_variable_updates(
    list_owner = "Vehicle",
    list_field = "visits",
    element_type = "usize",
    element_collection = "all_visits",
    distance_meter = "solverforge::cvrp::MatrixDistanceMeter",
    intra_distance_meter = "solverforge::cvrp::MatrixIntraDistanceMeter",
    cw_depot_fn = "solverforge::cvrp::depot_for_cw",
    cw_distance_fn = "solverforge::cvrp::distance",
    cw_element_load_fn = "solverforge::cvrp::element_load",
    cw_capacity_fn = "solverforge::cvrp::capacity",
    cw_assign_route_fn = "solverforge::cvrp::assign_route",
)]
pub struct VrpPlan {
    #[planning_entity_collection]
    pub vehicles: Vec<Vehicle>,
    /// All customer node indices (populated from problem data).
    pub all_visits: Vec<usize>,
    #[planning_score]
    pub score: Option<HardSoftScore>,
    /// Owned problem data — must outlive all vehicle raw pointers.
    #[serde(skip)]
    pub problem_data: Option<Box<ProblemData>>,
}

impl VrpSolution for VrpPlan {
    fn vehicle_data_ptr(&self, _entity_idx: usize) -> *const ProblemData {
        self.problem_data
            .as_deref()
            .map(|p| p as *const ProblemData)
            .unwrap_or(std::ptr::null())
    }

    fn vehicle_visits(&self, entity_idx: usize) -> &[usize] {
        &self.vehicles[entity_idx].visits
    }

    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize> {
        &mut self.vehicles[entity_idx].visits
    }

    fn vehicle_count(&self) -> usize {
        self.vehicles.len()
    }
}
