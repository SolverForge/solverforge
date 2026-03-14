use serde::{Deserialize, Serialize};

use solverforge::SolverStatus;

/// Input: a CVRP instance with a distance matrix.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDto {
    pub capacity: i64,
    pub depot: usize,
    pub demands: Vec<i32>,
    pub distance_matrix: Vec<Vec<i64>>,
    /// Number of vehicles to use (default: 3).
    #[serde(default = "default_vehicles")]
    pub n_vehicles: usize,
    /// Time limit in seconds (configured via solver.toml).
    #[serde(default = "default_time_limit")]
    pub time_limit_secs: u64,
}

fn default_vehicles() -> usize { 3 }
fn default_time_limit() -> u64 { 60 }

/// Output: solved routes + cost + status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolutionDto {
    /// Each inner Vec is the ordered list of customer node indices for one vehicle.
    pub routes: Vec<Vec<usize>>,
    pub cost: i64,
    pub score: Option<String>,
    pub solver_status: SolverStatus,
}
