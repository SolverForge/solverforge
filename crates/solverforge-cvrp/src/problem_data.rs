/// Matrix sentinel for a leg that cannot be traversed.
///
/// Stock CVRP helpers treat this as non-evaluable route-local travel and as a
/// very large construction distance. It is intentionally the same sentinel used
/// by `solverforge-maps` road-network matrices.
pub const UNREACHABLE: i64 = i64::MAX;

pub(crate) const MAX_SAFE_LEG_COST: i64 = i64::MAX / 4;

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

impl ProblemData {
    #[inline]
    pub(crate) fn distance_cost(&self, from: usize, to: usize) -> i64 {
        self.finite_matrix_value(&self.distance_matrix, from, to)
            .unwrap_or(MAX_SAFE_LEG_COST)
    }

    #[inline]
    pub(crate) fn finite_distance(&self, from: usize, to: usize) -> Option<i64> {
        self.finite_matrix_value(&self.distance_matrix, from, to)
    }

    #[inline]
    pub(crate) fn travel_time(&self, from: usize, to: usize) -> Option<i64> {
        self.finite_matrix_value(&self.travel_times, from, to)
    }

    #[inline]
    fn finite_matrix_value(&self, matrix: &[Vec<i64>], from: usize, to: usize) -> Option<i64> {
        let value = matrix.get(from)?.get(to).copied()?;
        (value >= 0 && value != UNREACHABLE).then_some(value)
    }
}
