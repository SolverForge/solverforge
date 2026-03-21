use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

use super::ProblemData;

/// A vehicle that serves an ordered list of customer visits.
///
/// `visits` is the list variable: the solver inserts, removes, and reorders
/// customer indices within and across vehicles to minimise total distance.
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct Vehicle {
    #[planning_id]
    pub id: usize,
    /// Ordered sequence of customer node indices served by this vehicle.
    pub visits: Vec<usize>,
    /// Raw pointer to shared problem data (capacity, demands, distances).
    /// Stored here so constraints can access it without an extra map lookup.
    #[serde(skip)]
    pub data: *const ProblemData,
}

// The pointer is read-only and the data outlives the plan — safe to share.
unsafe impl Send for Vehicle {}
unsafe impl Sync for Vehicle {}
