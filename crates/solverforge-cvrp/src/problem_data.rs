/// Immutable problem data shared by all vehicles.
///
/// Stored via raw pointer in each vehicle so the framework can clone vehicles
/// freely during local search without copying matrices.
#[derive(Clone, Debug)]
pub struct ProblemData {
    pub capacity: i64,
    pub depot: usize,
    pub demands: Vec<i32>,
    pub distance_matrix: Vec<Vec<i64>>,
    pub time_windows: Vec<(i64, i64)>,
    pub service_durations: Vec<i64>,
    pub travel_times: Vec<Vec<i64>>,
    pub vehicle_departure_time: i64,
}
