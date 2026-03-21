/* Constraint definitions for vehicle routing.

   Two constraints mirror the CVRP standard:
     - HARD  vehicleCapacity — penalise demand overload per vehicle
     - SOFT  totalDistance   — minimise sum of route distances */

use crate::domain::{ProblemData, Vehicle, VrpPlan, VrpPlanConstraintStreams};
use solverforge::prelude::*;

pub fn create_constraints() -> impl ConstraintSet<VrpPlan, HardSoftScore> {
    let capacity = ConstraintFactory::<VrpPlan, HardSoftScore>::new()
        .vehicles()
        .filter(|v: &Vehicle| !v.visits.is_empty())
        .penalize_hard_with(|v: &Vehicle| {
            let data = unsafe { &*(v.data as *const ProblemData) };
            let demand: i64 = v.visits.iter().map(|&i| data.demands[i] as i64).sum();
            let overload = (demand - data.capacity).max(0);
            HardSoftScore::of(overload, 0)
        })
        .named("vehicleCapacity");

    let distance = ConstraintFactory::<VrpPlan, HardSoftScore>::new()
        .vehicles()
        .filter(|v: &Vehicle| !v.visits.is_empty())
        .penalize_with(|v: &Vehicle| {
            let data = unsafe { &*(v.data as *const ProblemData) };
            let depot = data.depot;
            let mut dist = data.distance_matrix[depot][v.visits[0]];
            for w in v.visits.windows(2) {
                dist += data.distance_matrix[w[0]][w[1]];
            }
            dist += data.distance_matrix[*v.visits.last().unwrap()][depot];
            HardSoftScore::of(0, dist)
        })
        .named("totalDistance");

    (capacity, distance)
}
