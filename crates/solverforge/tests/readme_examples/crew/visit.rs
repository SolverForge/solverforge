use solverforge::prelude::*;

/// A visit, possibly one of several that technicians start together.
#[problem_fact]
pub struct Visit {
    /// Stable visit id.
    #[planning_id]
    pub id: usize,
    /// Service time.
    pub duration: i64,
    /// Travel time from every visit, indexed by visit.
    pub travel_from: Vec<i64>,
    /// The other visits of the same job, started together with this one.
    pub partners: Vec<usize>,
    /// The technician making the visit (list shadow).
    pub technician: Option<usize>,
    /// The visit before this one on the technician's route (list shadow).
    pub previous: Option<usize>,
    /// The visit after this one on the technician's route (list shadow).
    pub next: Option<usize>,
    /// When service starts (cascading list shadow).
    pub start: Option<i64>,
}
