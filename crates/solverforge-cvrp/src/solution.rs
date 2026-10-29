use crate::ProblemData;

/// Trait implemented by a planning solution that holds a fleet of vehicles,
/// each with its `ProblemData` and a list of visited stops.
pub trait VrpSolution {
    /// The problem data of vehicle `entity_idx`.
    ///
    /// Vehicles that share one `ProblemData` should return the same reference;
    /// Clarke-Wright then computes their savings once. `None` for an in-range
    /// vehicle is an initialization bug: the route and savings helpers panic
    /// on it, and the feasibility gates reject the route.
    fn vehicle_data(&self, entity_idx: usize) -> Option<&ProblemData>;
    /// The visit (element) indexes of vehicle `entity_idx`'s route, in order.
    fn vehicle_visits(&self, entity_idx: usize) -> &[usize];
    /// Mutable access to vehicle `entity_idx`'s route.
    fn vehicle_visits_mut(&mut self, entity_idx: usize) -> &mut Vec<usize>;
    /// The number of vehicles.
    fn vehicle_count(&self) -> usize;
}
