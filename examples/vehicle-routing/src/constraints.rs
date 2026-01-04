//! Constraints for Vehicle Routing Problem.
//!
//! Uses `IncrementalConstraint` for efficient O(1) incremental score updates.
//!
//! # Constraints
//!
//! - **Vehicle capacity** (hard): Total demand must not exceed vehicle capacity
//! - **Time windows** (hard): Service must complete before max end time
//! - **Minimize travel time** (soft): Reduce total driving time

#![allow(clippy::new_without_default)]

use solverforge::prelude::*;
use solverforge::IncrementalConstraint;
use std::collections::HashMap;

use crate::domain::VehicleRoutePlan;

/// All VRP constraints as a typed tuple for zero-erasure scoring.
pub type VrpConstraints = (
    VehicleCapacityConstraint,
    TimeWindowConstraint,
    MinimizeTravelTimeConstraint,
);

/// Creates all constraints for the vehicle routing problem.
///
/// # Examples
///
/// ```
/// use vehicle_routing::constraints::create_constraints;
/// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
/// use solverforge::{ConstraintSet, Score};
///
/// let locations = vec![Location::new(0, 0.0, 0.0)];
/// let visits = vec![Visit::new(0, "A", 0).with_demand(5)];
/// let mut vehicle = Vehicle::new(0, "V1", 10, 0);
/// vehicle.visits = vec![0];
///
/// let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
/// plan.finalize();
///
/// let constraints = create_constraints();
/// let score = constraints.evaluate_all(&plan);
/// assert!(score.is_feasible()); // Demand 5 <= capacity 10
/// ```
pub fn create_constraints() -> VrpConstraints {
    (
        VehicleCapacityConstraint::new(),
        TimeWindowConstraint::new(),
        MinimizeTravelTimeConstraint::new(),
    )
}

// ============================================================================
// HARD: Vehicle Capacity Constraint
// ============================================================================

/// Vehicle capacity constraint: total demand must not exceed vehicle capacity.
///
/// Penalty = excess demand (demand - capacity) for each over-capacity vehicle.
///
/// # Examples
///
/// ```
/// use vehicle_routing::constraints::VehicleCapacityConstraint;
/// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
/// use solverforge::IncrementalConstraint;
///
/// let locations = vec![Location::new(0, 0.0, 0.0)];
/// let visits = vec![
///     Visit::new(0, "A", 0).with_demand(60),
///     Visit::new(1, "B", 0).with_demand(50),
/// ];
/// let mut vehicle = Vehicle::new(0, "V1", 100, 0);
/// vehicle.visits = vec![0, 1]; // Total demand = 110
///
/// let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
/// plan.finalize();
///
/// let constraint = VehicleCapacityConstraint::new();
/// let score = constraint.evaluate(&plan);
///
/// // Excess = 110 - 100 = 10
/// assert_eq!(score.hard(), -10);
/// ```
pub struct VehicleCapacityConstraint {
    /// vehicle_idx → excess demand (demand - capacity), 0 if not over capacity
    excess: HashMap<usize, i32>,
}

impl VehicleCapacityConstraint {
    pub fn new() -> Self {
        Self {
            excess: HashMap::new(),
        }
    }

    /// Calculates excess demand for a vehicle (0 if under capacity).
    fn calculate_excess(solution: &VehicleRoutePlan, vehicle_idx: usize) -> i32 {
        let Some(vehicle) = solution.vehicles.get(vehicle_idx) else {
            return 0;
        };
        let total_demand = vehicle.total_demand(solution);
        (total_demand - vehicle.capacity).max(0)
    }
}

impl IncrementalConstraint<VehicleRoutePlan, HardSoftScore> for VehicleCapacityConstraint {
    fn evaluate(&self, solution: &VehicleRoutePlan) -> HardSoftScore {
        let mut total_excess = 0i64;
        for (idx, _) in solution.vehicles.iter().enumerate() {
            total_excess += Self::calculate_excess(solution, idx) as i64;
        }
        HardSoftScore::of_hard(-total_excess)
    }

    fn match_count(&self, solution: &VehicleRoutePlan) -> usize {
        solution
            .vehicles
            .iter()
            .enumerate()
            .filter(|(idx, _)| Self::calculate_excess(solution, *idx) > 0)
            .count()
    }

    fn initialize(&mut self, solution: &VehicleRoutePlan) -> HardSoftScore {
        self.excess.clear();
        let mut total_excess = 0i64;
        for (idx, _) in solution.vehicles.iter().enumerate() {
            let excess = Self::calculate_excess(solution, idx);
            if excess > 0 {
                self.excess.insert(idx, excess);
                total_excess += excess as i64;
            }
        }
        HardSoftScore::of_hard(-total_excess)
    }

    fn on_insert(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        // entity_index is the vehicle index
        if entity_index >= solution.vehicles.len() {
            return HardSoftScore::ZERO;
        }

        let old_excess = self.excess.get(&entity_index).copied().unwrap_or(0);
        let new_excess = Self::calculate_excess(solution, entity_index);

        if new_excess > 0 {
            self.excess.insert(entity_index, new_excess);
        } else {
            self.excess.remove(&entity_index);
        }

        let delta = (new_excess - old_excess) as i64;
        HardSoftScore::of_hard(-delta)
    }

    fn on_retract(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        // For VRP, retract is the same as recalculating after the change
        self.on_insert(solution, entity_index)
    }

    fn reset(&mut self) {
        self.excess.clear();
    }

    fn name(&self) -> &str {
        "Vehicle capacity"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// HARD: Time Window Constraint
// ============================================================================

/// Time window constraint: service must complete before max end time.
///
/// Penalty = total late minutes across all visits.
///
/// # Examples
///
/// ```
/// use vehicle_routing::constraints::TimeWindowConstraint;
/// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
/// use solverforge::IncrementalConstraint;
///
/// let locations = vec![
///     Location::new(0, 0.0, 0.0),  // Depot
///     Location::new(1, 0.0, 1.0),  // ~111 km away, ~2.2 hours at 50 km/h
/// ];
/// let visits = vec![
///     Visit::new(0, "A", 1)
///         .with_time_window(0, 8 * 3600 + 30 * 60)  // Must finish by 8:30am
///         .with_service_duration(300),  // 5 min service
/// ];
/// let mut vehicle = Vehicle::new(0, "V1", 100, 0);
/// vehicle.departure_time = 8 * 3600;  // Depart at 8am
/// vehicle.visits = vec![0];
///
/// let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
/// plan.finalize();
///
/// let constraint = TimeWindowConstraint::new();
/// let score = constraint.evaluate(&plan);
///
/// // Vehicle departs 8am, travels ~2.2 hours, arrives ~10:13am
/// // Service ends ~10:18am, but max_end is 8:30am
/// // Late by ~108 minutes
/// assert!(score.hard() < 0);
/// ```
pub struct TimeWindowConstraint {
    /// vehicle_idx → total late minutes for that vehicle's route
    late_minutes: HashMap<usize, i64>,
}

impl TimeWindowConstraint {
    pub fn new() -> Self {
        Self {
            late_minutes: HashMap::new(),
        }
    }

    /// Calculates total late minutes for a vehicle's route.
    fn calculate_late_minutes(solution: &VehicleRoutePlan, vehicle_idx: usize) -> i64 {
        let Some(vehicle) = solution.vehicles.get(vehicle_idx) else {
            return 0;
        };

        let timings = solution.calculate_route_times(vehicle);
        let mut total_late = 0i64;

        for timing in &timings {
            if let Some(visit) = solution.get_visit(timing.visit_idx) {
                let late_seconds = (timing.departure - visit.max_end_time).max(0);
                // Convert to minutes, rounding up
                let late_minutes = (late_seconds + 59) / 60;
                total_late += late_minutes;
            }
        }

        total_late
    }
}

impl IncrementalConstraint<VehicleRoutePlan, HardSoftScore> for TimeWindowConstraint {
    fn evaluate(&self, solution: &VehicleRoutePlan) -> HardSoftScore {
        let mut total_late = 0i64;
        for (idx, _) in solution.vehicles.iter().enumerate() {
            total_late += Self::calculate_late_minutes(solution, idx);
        }
        HardSoftScore::of_hard(-total_late)
    }

    fn match_count(&self, solution: &VehicleRoutePlan) -> usize {
        let mut count = 0;
        for vehicle in &solution.vehicles {
            let timings = solution.calculate_route_times(vehicle);
            for timing in &timings {
                if let Some(visit) = solution.get_visit(timing.visit_idx) {
                    if timing.departure > visit.max_end_time {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn initialize(&mut self, solution: &VehicleRoutePlan) -> HardSoftScore {
        self.late_minutes.clear();
        let mut total_late = 0i64;
        for (idx, _) in solution.vehicles.iter().enumerate() {
            let late = Self::calculate_late_minutes(solution, idx);
            if late > 0 {
                self.late_minutes.insert(idx, late);
                total_late += late;
            }
        }
        HardSoftScore::of_hard(-total_late)
    }

    fn on_insert(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        if entity_index >= solution.vehicles.len() {
            return HardSoftScore::ZERO;
        }

        let old_late = self.late_minutes.get(&entity_index).copied().unwrap_or(0);
        let new_late = Self::calculate_late_minutes(solution, entity_index);

        if new_late > 0 {
            self.late_minutes.insert(entity_index, new_late);
        } else {
            self.late_minutes.remove(&entity_index);
        }

        let delta = new_late - old_late;
        HardSoftScore::of_hard(-delta)
    }

    fn on_retract(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        self.on_insert(solution, entity_index)
    }

    fn reset(&mut self) {
        self.late_minutes.clear();
    }

    fn name(&self) -> &str {
        "Service finished after max end time"
    }

    fn is_hard(&self) -> bool {
        true
    }
}

// ============================================================================
// SOFT: Minimize Travel Time Constraint
// ============================================================================

/// Minimize travel time: penalize total driving time across all vehicles.
///
/// Penalty = total driving time in seconds.
///
/// # Examples
///
/// ```
/// use vehicle_routing::constraints::MinimizeTravelTimeConstraint;
/// use vehicle_routing::domain::{Location, Visit, Vehicle, VehicleRoutePlan};
/// use solverforge::IncrementalConstraint;
///
/// let locations = vec![
///     Location::new(0, 0.0, 0.0),   // Depot
///     Location::new(1, 0.0, 0.01),  // ~1.1 km away
/// ];
/// let visits = vec![Visit::new(0, "A", 1)];
/// let mut vehicle = Vehicle::new(0, "V1", 100, 0);
/// vehicle.visits = vec![0];
///
/// let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
/// plan.finalize();
///
/// let constraint = MinimizeTravelTimeConstraint::new();
/// let score = constraint.evaluate(&plan);
///
/// // Should penalize the travel time (to visit and back)
/// assert!(score.soft() < 0);
/// ```
pub struct MinimizeTravelTimeConstraint {
    /// vehicle_idx → driving time in seconds
    driving_times: HashMap<usize, i64>,
}

impl MinimizeTravelTimeConstraint {
    pub fn new() -> Self {
        Self {
            driving_times: HashMap::new(),
        }
    }
}

impl IncrementalConstraint<VehicleRoutePlan, HardSoftScore> for MinimizeTravelTimeConstraint {
    fn evaluate(&self, solution: &VehicleRoutePlan) -> HardSoftScore {
        let total: i64 = solution
            .vehicles
            .iter()
            .map(|v| solution.total_driving_time(v))
            .sum();
        HardSoftScore::of_soft(-total)
    }

    fn match_count(&self, solution: &VehicleRoutePlan) -> usize {
        solution.vehicles.iter().filter(|v| !v.visits.is_empty()).count()
    }

    fn initialize(&mut self, solution: &VehicleRoutePlan) -> HardSoftScore {
        self.driving_times.clear();
        let mut total = 0i64;
        for (idx, vehicle) in solution.vehicles.iter().enumerate() {
            let time = solution.total_driving_time(vehicle);
            if time > 0 {
                self.driving_times.insert(idx, time);
                total += time;
            }
        }
        HardSoftScore::of_soft(-total)
    }

    fn on_insert(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        let Some(vehicle) = solution.vehicles.get(entity_index) else {
            return HardSoftScore::ZERO;
        };

        let old_time = self.driving_times.get(&entity_index).copied().unwrap_or(0);
        let new_time = solution.total_driving_time(vehicle);

        if new_time > 0 {
            self.driving_times.insert(entity_index, new_time);
        } else {
            self.driving_times.remove(&entity_index);
        }

        let delta = new_time - old_time;
        HardSoftScore::of_soft(-delta)
    }

    fn on_retract(
        &mut self,
        solution: &VehicleRoutePlan,
        entity_index: usize,
    ) -> HardSoftScore {
        self.on_insert(solution, entity_index)
    }

    fn reset(&mut self) {
        self.driving_times.clear();
    }

    fn name(&self) -> &str {
        "Minimize travel time"
    }

    fn is_hard(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Location, Vehicle, Visit};

    fn simple_plan() -> VehicleRoutePlan {
        let locations = vec![
            Location::new(0, 0.0, 0.0),  // Depot
            Location::new(1, 0.0, 0.01), // ~1.1 km
            Location::new(2, 0.0, 0.02), // ~2.2 km
        ];
        let visits = vec![
            Visit::new(0, "A", 1).with_demand(5),
            Visit::new(1, "B", 2).with_demand(3),
        ];
        let vehicles = vec![
            Vehicle::new(0, "V1", 100, 0),
            Vehicle::new(1, "V2", 100, 0),
        ];
        let mut plan = VehicleRoutePlan::new("test", locations, visits, vehicles);
        plan.finalize();
        plan
    }

    #[test]
    fn test_capacity_constraint_feasible() {
        let mut plan = simple_plan();
        plan.vehicles[0].visits = vec![0, 1]; // Total demand = 8

        let constraint = VehicleCapacityConstraint::new();
        let score = constraint.evaluate(&plan);
        assert_eq!(score, HardSoftScore::ZERO);
    }

    #[test]
    fn test_capacity_constraint_violation() {
        let locations = vec![Location::new(0, 0.0, 0.0)];
        let visits = vec![
            Visit::new(0, "A", 0).with_demand(60),
            Visit::new(1, "B", 0).with_demand(50),
        ];
        let mut vehicle = Vehicle::new(0, "V1", 100, 0);
        vehicle.visits = vec![0, 1]; // Total = 110, over by 10

        let mut plan = VehicleRoutePlan::new("test", locations, visits, vec![vehicle]);
        plan.finalize();

        let constraint = VehicleCapacityConstraint::new();
        let score = constraint.evaluate(&plan);
        assert_eq!(score, HardSoftScore::of_hard(-10));
    }

    #[test]
    fn test_minimize_travel_time() {
        let mut plan = simple_plan();
        plan.vehicles[0].visits = vec![0];

        let constraint = MinimizeTravelTimeConstraint::new();
        let score = constraint.evaluate(&plan);

        // Should have negative soft score (penalizing travel time)
        assert!(score.soft() < 0);
    }
}
